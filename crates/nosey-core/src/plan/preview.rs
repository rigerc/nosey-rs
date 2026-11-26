//! Read-only previews backing each planned change.

use super::{ChangeDetail, ChangeRequest, acquisition::Acquisitions};
use crate::{
    Error, State,
    changes::{InstallPreview, InstallReason, LinkAction, Status},
    library::{Library, skill_name},
    library_io::invalid,
    targets::InstallScope,
};
use std::path::Path;

/// Where an install request lands.
enum InstallAt<'a> {
    Explicit {
        project: &'a Path,
        target: &'a Path,
    },
    Scoped {
        scope: &'a InstallScope,
        target: &'a Path,
    },
}

impl Library {
    pub(super) fn preview_request(
        &self,
        state: &State,
        request: &ChangeRequest,
        acquisitions: &Acquisitions<'_>,
    ) -> Result<(ChangeDetail, Option<String>), Error> {
        Ok(match request {
            ChangeRequest::InstallLink {
                project,
                target,
                artifact,
                reason,
            } => self.preview_install(
                state,
                &InstallAt::Explicit { project, target },
                artifact,
                reason,
                acquisitions,
            )?,
            ChangeRequest::InstallScoped {
                scope,
                target,
                artifact,
                reason,
            } => {
                let target = target
                    .as_deref()
                    .ok_or_else(|| invalid("no enabled targets for scope"))?;
                self.preview_install(
                    state,
                    &InstallAt::Scoped { scope, target },
                    artifact,
                    reason,
                    acquisitions,
                )?
            }
            ChangeRequest::AcquireLibrarySkill {
                source,
                install_name,
            } => {
                if acquisitions
                    .get(source.artifact())
                    .is_some_and(Option::is_none)
                {
                    return Err(invalid("conflicting acquisitions for one artifact"));
                }
                let preview = self.preview_acquisition(source)?;
                if let Some(name) = install_name {
                    skill_name(name)?;
                    if preview.cached().is_some_and(|cached| &cached.name != name) {
                        return Err(invalid("cached skill name differs from the expected name"));
                    }
                }
                (ChangeDetail::Acquire(preview), None)
            }
            ChangeRequest::UnlinkManaged {
                destination,
                reason,
            } => {
                let preview = self.preview_unlink(destination, reason)?;
                let blocker = matches!(preview.action, LinkAction::Blocked)
                    .then(|| preview.detail.to_owned());
                (ChangeDetail::Unlink(preview), blocker)
            }
            ChangeRequest::RemoveUnusedLibrarySkill {
                artifact,
                external_references_checked,
            } => {
                let preview =
                    self.preview_remove_unused(state, artifact, *external_references_checked)?;
                let blocker = preview.blocker.map(Into::into);
                (ChangeDetail::Remove(preview), blocker)
            }
            ChangeRequest::UpdateRemoteLibrarySkill { source } => (
                ChangeDetail::RemoteUpdate(Box::new(self.preview_remote_update(source)?)),
                None,
            ),
            ChangeRequest::UpdateLibrarySkill { artifact, source } => (
                ChangeDetail::Update(self.preview_update(artifact, source)?),
                None,
            ),
        })
    }

    /// Preview an install from local content, or, when the artifact is not yet
    /// in the library, against a source acquired earlier in the same plan.
    fn preview_install(
        &self,
        state: &State,
        at: &InstallAt<'_>,
        artifact: &str,
        reason: &InstallReason,
        acquisitions: &Acquisitions<'_>,
    ) -> Result<(ChangeDetail, Option<String>), Error> {
        let local = match at {
            InstallAt::Explicit { project, target } => {
                self.preview(state, project, target, artifact, reason.clone())
            }
            InstallAt::Scoped { scope, target } => {
                self.preview_scoped(state, scope, target, artifact, reason.clone())
            }
        };
        let error = match local {
            Ok(preview) => {
                let blocker = install_blocker(&preview);
                return Ok((ChangeDetail::Install(preview), blocker));
            }
            Err(error) => error,
        };
        let Some(Some(ChangeRequest::AcquireLibrarySkill {
            source,
            install_name: Some(name),
        })) = acquisitions.get(artifact)
        else {
            return Err(error);
        };
        if self.preview_acquisition(source)?.cached().is_some() {
            return Err(error);
        }
        // The local tree digest is only known after extraction.
        let pending = (name.as_str(), "");
        let install = match at {
            InstallAt::Explicit { project, target } => {
                self.preview_skill(state, project, target, artifact, reason.clone(), pending)?
            }
            InstallAt::Scoped { scope, target } => {
                self.preview_scoped_skill(state, scope, target, artifact, reason.clone(), pending)?
            }
        };
        let blocker = install_blocker(&install);
        Ok((
            ChangeDetail::PendingInstall {
                install,
                source: source.clone(),
            },
            blocker,
        ))
    }
}

fn install_blocker(preview: &InstallPreview) -> Option<String> {
    match preview.status {
        Status::Available | Status::Managed => None,
        Status::Occupied => Some("destination is occupied by a path Nosey does not own".into()),
        Status::RecoveryRequired => Some("destination link journal needs recovery".into()),
    }
}
