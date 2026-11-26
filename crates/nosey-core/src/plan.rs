//! Shared change planning for installs, unlinks and library updates.
//!
//! A plan is an actionable preview, never a solver or background job. Every
//! request is previewed independently; failures become blockers on that item
//! rather than failing the whole plan. One plan fingerprint approves exactly
//! the reviewed set. Apply revalidates each item immediately before mutating,
//! continues past individual failures and reports a per-item outcome.

use crate::{
    Error, State,
    changes::{InstallPreview, InstallReason, LinkAction, LinkPreview, Status},
    library::{Library, UpdatePreview, hash},
    library_io::invalid,
    targets::InstallScope,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    fs,
    io::Read,
    path::{Path, PathBuf},
};

/// Largest number of changes accepted in one plan.
pub const MAX_PLAN_CHANGES: usize = 256;
/// Largest accepted change request file.
const MAX_REQUEST_FILE: u64 = 1024 * 1024;

/// One requested change, independent of current filesystem state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum ChangeRequest {
    /// Link a library skill into an existing project-relative target directory.
    InstallLink {
        /// Registered project root.
        project: PathBuf,
        /// Existing project-relative target directory.
        target: PathBuf,
        /// Library artifact identity.
        artifact: String,
        /// Independent reason for the link.
        reason: InstallReason,
    },
    /// Install into the enabled targets for a global or project scope.
    InstallScoped {
        /// Scope and its root.
        scope: InstallScope,
        /// Select one enabled target, or expand to all compatible targets when absent.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        target: Option<PathBuf>,
        /// Library artifact identity.
        artifact: String,
        /// Independent reason for the link.
        reason: InstallReason,
    },
    /// Remove one reason from a managed link, and the link with its last reason.
    UnlinkManaged {
        /// Exact absolute link destination.
        destination: PathBuf,
        /// Reason to remove.
        reason: InstallReason,
    },
    /// Replace a library skill's content from an explicit local source.
    UpdateLibrarySkill {
        /// Library artifact identity.
        artifact: String,
        /// Replacement source directory.
        source: PathBuf,
    },
}

impl ChangeRequest {
    /// Link changes run before library replacement so a content swap never
    /// invalidates a reviewed link preview later in the same plan.
    const fn apply_rank(&self) -> u8 {
        match self {
            Self::UnlinkManaged { .. } => 0,
            Self::InstallLink { .. } | Self::InstallScoped { .. } => 1,
            Self::UpdateLibrarySkill { .. } => 2,
        }
    }
}

/// Current preview backing a planned change.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeDetail {
    /// Install preview.
    Install(InstallPreview),
    /// Unlink preview.
    Unlink(LinkPreview),
    /// Library update preview.
    Update(UpdatePreview),
}

/// One reviewed item of a change plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PlannedChange {
    /// Stable identity derived from the request.
    pub id: String,
    /// The original request.
    pub request: ChangeRequest,
    /// Current preview, when one could be produced.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<ChangeDetail>,
    /// Why this item will not be executed, if blocked.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocker: Option<String>,
    /// Fingerprint of this item's observed state.
    pub fingerprint: String,
}

/// A reviewed set of changes with a single approval fingerprint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ChangePlan {
    /// Deduplicated changes in request order.
    pub changes: Vec<PlannedChange>,
    /// Fingerprint over every item, supplied for explicit approval.
    pub fingerprint: String,
    #[serde(skip)]
    requests: Vec<ChangeRequest>,
}

impl ChangePlan {
    /// Number of changes that will be attempted on apply.
    #[must_use]
    pub fn eligible(&self) -> usize {
        self.changes.iter().filter(|c| c.blocker.is_none()).count()
    }
}

/// Result of attempting one planned change.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum ChangeOutcome {
    /// The change was applied and verified.
    Applied,
    /// The change was blocked at review time and not attempted.
    Blocked {
        /// Review-time blocker.
        reason: String,
    },
    /// State changed after review, possibly by an earlier item; plan again.
    Stale,
    /// The change was attempted and failed; plan again to retry.
    Failed {
        /// Failure description.
        error: String,
    },
}

/// Outcome of one planned change, in plan order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ChangeResult {
    /// Planned change identity.
    pub id: String,
    /// What happened.
    #[serde(flatten)]
    pub outcome: ChangeOutcome,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RequestFile {
    changes: Vec<ChangeRequest>,
}

/// Load a bounded JSON change request file (`{"changes": [...]}`).
///
/// # Errors
///
/// Rejects non-regular or oversized files, unknown fields and too many changes.
pub fn load_requests(path: &Path) -> Result<Vec<ChangeRequest>, Error> {
    if !fs::symlink_metadata(path)?.is_file() {
        return Err(invalid("change request file must be a regular file"));
    }
    let mut data = Vec::new();
    fs::File::open(path)?
        .take(MAX_REQUEST_FILE + 1)
        .read_to_end(&mut data)?;
    if data.len() as u64 > MAX_REQUEST_FILE {
        return Err(invalid("change request file size limit"));
    }
    let file: RequestFile = serde_json::from_slice(&data)?;
    if file.changes.len() > MAX_PLAN_CHANGES {
        return Err(invalid("too many changes in one plan"));
    }
    Ok(file.changes)
}

fn request_id(request: &ChangeRequest) -> Result<String, Error> {
    Ok(hash(&serde_json::to_vec(request)?))
}

fn expand(state: &State, request: &ChangeRequest) -> Vec<ChangeRequest> {
    let ChangeRequest::InstallScoped {
        scope,
        target: None,
        artifact,
        reason,
    } = request
    else {
        return vec![request.clone()];
    };
    let paths = state
        .targets()
        .paths(matches!(scope, InstallScope::Global { .. }));
    if paths.is_empty() {
        return vec![request.clone()];
    }
    paths
        .into_iter()
        .map(|target| ChangeRequest::InstallScoped {
            scope: scope.clone(),
            target: Some(target),
            artifact: artifact.clone(),
            reason: reason.clone(),
        })
        .collect()
}

impl Library {
    /// Preview every request without changing anything.
    ///
    /// # Errors
    ///
    /// Fails only for too many requests or unserializable state; individual
    /// preview failures are reported as blockers on their item.
    pub fn plan(&self, state: &State, requests: &[ChangeRequest]) -> Result<ChangePlan, Error> {
        if requests.len() > MAX_PLAN_CHANGES {
            return Err(invalid("too many changes in one plan"));
        }
        let mut seen = BTreeSet::new();
        let mut changes = Vec::new();
        for request in requests {
            for request in expand(state, request) {
                let id = request_id(&request)?;
                if seen.insert(id.clone()) {
                    if changes.len() == MAX_PLAN_CHANGES {
                        return Err(invalid("too many changes after target expansion"));
                    }
                    changes.push(self.plan_one(state, id, &request)?);
                }
            }
        }
        let fingerprint = hash(&serde_json::to_vec(
            &changes
                .iter()
                .map(|c| (&c.id, &c.fingerprint, &c.blocker))
                .collect::<Vec<_>>(),
        )?);
        Ok(ChangePlan {
            changes,
            fingerprint,
            requests: requests.to_vec(),
        })
    }

    /// Apply an approved plan. The plan must still match a fresh review; each
    /// eligible item is then revalidated and applied independently.
    ///
    /// # Errors
    ///
    /// Rejects a plan that no longer matches current state before any mutation.
    /// Individual item failures are reported in the results, not as errors.
    pub fn apply_plan(
        &self,
        state: &State,
        approved: &ChangePlan,
    ) -> Result<Vec<ChangeResult>, Error> {
        let fresh = self.plan(state, &approved.requests)?;
        if &fresh != approved {
            return Err(invalid("stale change plan; review again"));
        }
        let mut order: Vec<usize> = (0..fresh.changes.len()).collect();
        order.sort_by_key(|&i| fresh.changes[i].request.apply_rank());
        let mut outcomes = vec![None; fresh.changes.len()];
        for i in order {
            let change = &fresh.changes[i];
            outcomes[i] = Some(change.blocker.as_ref().map_or_else(
                || self.apply_one(state, change),
                |reason| ChangeOutcome::Blocked {
                    reason: reason.clone(),
                },
            ));
        }
        Ok(fresh
            .changes
            .iter()
            .zip(outcomes)
            .map(|(change, outcome)| ChangeResult {
                id: change.id.clone(),
                outcome: outcome.unwrap_or(ChangeOutcome::Stale),
            })
            .collect())
    }

    fn plan_one(
        &self,
        state: &State,
        id: String,
        request: &ChangeRequest,
    ) -> Result<PlannedChange, Error> {
        let (detail, blocker) = match self.preview_request(state, request) {
            Ok((detail, blocker)) => (Some(detail), blocker),
            Err(error) => (None, Some(error.to_string())),
        };
        let fingerprint = match &detail {
            Some(ChangeDetail::Install(p)) => p.fingerprint.clone(),
            Some(ChangeDetail::Unlink(p)) => p.fingerprint.clone(),
            Some(ChangeDetail::Update(p)) => p.fingerprint.clone(),
            None => hash(&serde_json::to_vec(&("blocked", &id, &blocker))?),
        };
        Ok(PlannedChange {
            id,
            request: request.clone(),
            detail,
            blocker,
            fingerprint,
        })
    }

    fn preview_request(
        &self,
        state: &State,
        request: &ChangeRequest,
    ) -> Result<(ChangeDetail, Option<String>), Error> {
        let install = match request {
            ChangeRequest::InstallLink {
                project,
                target,
                artifact,
                reason,
            } => Some(self.preview(state, project, target, artifact, reason.clone())?),
            ChangeRequest::InstallScoped {
                scope,
                target,
                artifact,
                reason,
            } => {
                let target = target
                    .as_deref()
                    .ok_or_else(|| invalid("no enabled targets for scope"))?;
                Some(self.preview_scoped(state, scope, target, artifact, reason.clone())?)
            }
            _ => None,
        };
        if let Some(preview) = install {
            let blocker = match preview.status {
                Status::Available | Status::Managed => None,
                Status::Occupied => Some("destination is occupied by a path Nosey does not own"),
                Status::RecoveryRequired => Some("destination link journal needs recovery"),
            };
            return Ok((ChangeDetail::Install(preview), blocker.map(Into::into)));
        }
        Ok(match request {
            ChangeRequest::UnlinkManaged {
                destination,
                reason,
            } => {
                let preview = self.preview_unlink(destination, reason)?;
                let blocker = matches!(preview.action, LinkAction::Blocked)
                    .then(|| preview.detail.to_owned());
                (ChangeDetail::Unlink(preview), blocker)
            }
            ChangeRequest::UpdateLibrarySkill { artifact, source } => (
                ChangeDetail::Update(self.preview_update(artifact, source)?),
                None,
            ),
            ChangeRequest::InstallLink { .. } | ChangeRequest::InstallScoped { .. } => {
                return Err(invalid("install preview missing"));
            }
        })
    }

    fn apply_one(&self, state: &State, reviewed: &PlannedChange) -> ChangeOutcome {
        // Earlier items in this plan may have changed the observed state.
        let current = match self.plan_one(state, reviewed.id.clone(), &reviewed.request) {
            Ok(current) if current.fingerprint == reviewed.fingerprint => current,
            Ok(_) => return ChangeOutcome::Stale,
            Err(error) => {
                return ChangeOutcome::Failed {
                    error: error.to_string(),
                };
            }
        };
        let applied = match &current.detail {
            Some(ChangeDetail::Install(p)) => self.apply(state, p).map(drop),
            Some(ChangeDetail::Unlink(p)) => self.apply_unlink(p).map(drop),
            Some(ChangeDetail::Update(p)) => self.apply_update(p).map(drop),
            None => Err(invalid("change has no preview")),
        };
        match applied {
            Ok(()) => ChangeOutcome::Applied,
            Err(error) => ChangeOutcome::Failed {
                error: error.to_string(),
            },
        }
    }
}
