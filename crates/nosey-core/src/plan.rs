//! Shared change planning for installs, unlinks and library updates.
//!
//! A plan is an actionable preview, never a solver or background job. Every
//! request is previewed independently; failures become blockers on that item
//! rather than failing the whole plan. One plan fingerprint approves exactly
//! the reviewed set. Apply revalidates each item immediately before mutating,
//! continues past individual failures and reports a per-item outcome.

use crate::{
    Error, State,
    changes::{InstallPreview, InstallReason, LinkPreview},
    library::{
        AcquisitionPreview, BundleSource, Library, RemoteUpdatePreview, RemovePreview,
        UpdatePreview, hash,
    },
    library_io::invalid,
    targets::InstallScope,
};
use nosey_registry::Fetch;
mod acquisition;
mod preview;
use acquisition::{Acquisitions, HttpsFetch, acquisitions};
mod apply_sync;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
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
    /// Retrieve a digest-pinned Agent Skill bundle from a selected catalog entry.
    AcquireLibrarySkill {
        /// Reviewed catalog source and canonical identity.
        source: BundleSource,
        /// Expected manifest name for any dependent installs in this batch.
        #[serde(default)]
        install_name: Option<String>,
    },
    /// Remove an unreferenced active library copy.
    RemoveUnusedLibrarySkill {
        /// Library artifact identity.
        artifact: String,
        /// Whether unmanaged references to this object have been ruled out.
        #[serde(default)]
        external_references_checked: bool,
    },
    /// Replace a library skill's content from an explicit local source.
    UpdateLibrarySkill {
        /// Library artifact identity.
        artifact: String,
        /// Replacement source directory.
        source: PathBuf,
    },
    /// Replace a catalog bundle at its existing library path.
    UpdateRemoteLibrarySkill {
        /// Exact replacement declaration from the selected catalog.
        source: BundleSource,
    },
}

impl ChangeRequest {
    /// Acquisitions run before the installs that need them, and link changes
    /// run before library replacement so a content swap never invalidates a
    /// reviewed link preview later in the same plan.
    const fn apply_rank(&self) -> u8 {
        match self {
            Self::UnlinkManaged { .. } => 0,
            Self::AcquireLibrarySkill { .. } => 1,
            Self::InstallLink { .. } | Self::InstallScoped { .. } => 2,
            Self::UpdateLibrarySkill { .. } | Self::UpdateRemoteLibrarySkill { .. } => 3,
            Self::RemoveUnusedLibrarySkill { .. } => 4,
        }
    }
}

/// Current preview backing a planned change.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeDetail {
    /// Install preview.
    Install(InstallPreview),
    /// Install destination reviewed before a bundle is available locally.
    PendingInstall {
        /// Project and destination state bound to the bundle digest.
        install: InstallPreview,
        /// Exact source whose acquisition must succeed first.
        source: BundleSource,
    },
    /// Acquisition preview.
    Acquire(AcquisitionPreview),
    /// Unlink preview.
    Unlink(LinkPreview),
    /// Library update preview.
    Update(UpdatePreview),
    /// Preview of a catalog bundle replacement and any provenance change.
    RemoteUpdate(Box<RemoteUpdatePreview>),
    /// Library removal preview.
    Remove(RemovePreview),
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
    #[serde(skip)]
    pub(crate) sync: Option<crate::sync::Sync>,
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
        let acquisitions = acquisitions(requests);
        let mut seen = BTreeSet::new();
        let mut changes = Vec::new();
        for request in requests {
            for request in expand(state, request) {
                let id = request_id(&request)?;
                if seen.insert(id.clone()) {
                    if changes.len() == MAX_PLAN_CHANGES {
                        return Err(invalid("too many changes after target expansion"));
                    }
                    changes.push(self.plan_one(state, id, &request, &acquisitions)?);
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
            sync: None,
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
        self.apply_plan_with_fetch(state, approved, &HttpsFetch)
    }

    /// Apply an approved plan using the supplied byte transport for acquisitions.
    ///
    /// # Errors
    /// Rejects a stale plan before mutation. Individual failures are reported per item.
    pub fn apply_plan_with_fetch(
        &self,
        state: &State,
        approved: &ChangePlan,
        fetch: &impl Fetch,
    ) -> Result<Vec<ChangeResult>, Error> {
        if let Some(kind) = approved.sync {
            return self.apply_sync(state, approved, kind);
        }
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
                || self.apply_one(state, change, fetch),
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
        acquisitions: &Acquisitions<'_>,
    ) -> Result<PlannedChange, Error> {
        let (detail, blocker) = match self.preview_request(state, request, acquisitions) {
            Ok((detail, blocker)) => (Some(detail), blocker),
            Err(error) => (None, Some(error.to_string())),
        };
        let fingerprint = match &detail {
            Some(ChangeDetail::Install(p)) => p.fingerprint.clone(),
            Some(ChangeDetail::Unlink(p)) => p.fingerprint.clone(),
            Some(ChangeDetail::Acquire(_) | ChangeDetail::PendingInstall { .. }) => {
                hash(&serde_json::to_vec(&(&id, &detail))?)
            }
            Some(ChangeDetail::Update(p)) => p.fingerprint.clone(),
            Some(ChangeDetail::RemoteUpdate(p)) => p.fingerprint.clone(),
            Some(ChangeDetail::Remove(p)) => p.fingerprint.clone(),
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

    fn apply_one(
        &self,
        state: &State,
        reviewed: &PlannedChange,
        fetch: &impl Fetch,
    ) -> ChangeOutcome {
        if let Some(ChangeDetail::PendingInstall { install, source }) = &reviewed.detail {
            // The install was reviewed against the bundle digest; apply_install
            // requires the acquisition to have succeeded and revalidates the destination.
            return match self.apply_install(state, install, Some(source)) {
                Ok(_) => ChangeOutcome::Applied,
                Err(error) => ChangeOutcome::Failed {
                    error: error.to_string(),
                },
            };
        }
        // Earlier items in this plan may have changed the observed state.
        let current = match self.plan_one(
            state,
            reviewed.id.clone(),
            &reviewed.request,
            &BTreeMap::new(),
        ) {
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
            Some(ChangeDetail::RemoteUpdate(p)) => self.apply_remote_update(p, fetch).map(drop),
            Some(ChangeDetail::Remove(p)) => self.apply_remove_unused(state, p),
            Some(ChangeDetail::Acquire(p)) => {
                let ChangeRequest::AcquireLibrarySkill { install_name, .. } = &reviewed.request
                else {
                    return ChangeOutcome::Stale;
                };
                self.acquire_named_bundle(&p.source, install_name.as_deref(), fetch)
                    .map(drop)
            }
            Some(ChangeDetail::PendingInstall { .. }) | None => {
                Err(invalid("change has no executable preview"))
            }
        };
        match applied {
            Ok(()) => ChangeOutcome::Applied,
            Err(error) => ChangeOutcome::Failed {
                error: error.to_string(),
            },
        }
    }
}
