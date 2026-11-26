//! Derive explicit change plans from desired settings and owned link journals.

use crate::{
    Error, State,
    changes::{InstallReason, LinkAudit, Status},
    library::Library,
    library_io::invalid,
    plan::{ChangePlan, ChangeRequest, MAX_PLAN_CHANGES},
    presets::PresetApplication,
    targets::InstallScope,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

/// Desired state source for a derived plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sync {
    /// Enabled targets.
    Targets,
    /// Linked preset membership.
    Presets,
}

/// Recover a scope from its journaled identity.
pub fn scope(link: &LinkAudit) -> InstallScope {
    if link.global {
        InstallScope::Global {
            home: link.project.clone(),
        }
    } else {
        InstallScope::Project {
            root: link.project.clone(),
        }
    }
}

fn relative_target(link: &LinkAudit) -> Option<&Path> {
    link.destination.parent()?.strip_prefix(&link.project).ok()
}

impl Library {
    /// Review changes needed to migrate managed installs to enabled targets.
    ///
    /// # Errors
    /// Fails if journals cannot be read or the plan exceeds the batch limit.
    pub fn plan_target_sync(&self, state: &State) -> Result<ChangePlan, Error> {
        self.plan_sync(state, Sync::Targets)
    }

    /// Review changes needed to reconcile linked preset membership.
    ///
    /// # Errors
    /// Fails if journals cannot be read or the plan exceeds the batch limit.
    pub fn plan_preset_sync(&self, state: &State) -> Result<ChangePlan, Error> {
        self.plan_sync(state, Sync::Presets)
    }

    pub(crate) fn plan_sync(&self, state: &State, kind: Sync) -> Result<ChangePlan, Error> {
        let links = self.audit_links(state)?;
        let links: Vec<_> = links.iter().filter(|link| !link.detached).collect();
        let requests = match kind {
            Sync::Targets => target_requests(state, &links),
            Sync::Presets => preset_requests(state, &links),
        }?;
        let mut plan = self.plan(state, &requests)?;
        plan.sync = Some(kind);
        Ok(plan)
    }
}

fn target_requests(state: &State, links: &[&LinkAudit]) -> Result<Vec<ChangeRequest>, Error> {
    let mut coverage = BTreeMap::<_, BTreeSet<_>>::new();
    for link in links
        .iter()
        .copied()
        .filter(|link| link.status == Status::Managed)
    {
        if let Some(target) = relative_target(link) {
            for reason in &link.reasons {
                coverage
                    .entry((scope(link), &link.artifact, reason))
                    .or_default()
                    .insert(target);
            }
        }
    }
    let mut requests = Vec::new();
    for ((scope, artifact, reason), covered) in coverage {
        for target in state
            .targets()
            .paths(matches!(scope, InstallScope::Global { .. }))
        {
            if !covered.contains(target.as_path()) {
                push_request(
                    &mut requests,
                    ChangeRequest::InstallScoped {
                        scope: scope.clone(),
                        target: Some(target),
                        artifact: artifact.clone(),
                        reason: reason.clone(),
                    },
                )?;
            }
        }
    }
    for link in links {
        if let Some(target) = relative_target(link)
            && !state.targets().paths(link.global).contains(target)
        {
            for reason in &link.reasons {
                push_request(
                    &mut requests,
                    ChangeRequest::UnlinkManaged {
                        destination: link.destination.clone(),
                        reason: reason.clone(),
                    },
                )?;
            }
        }
    }
    Ok(requests)
}

fn preset_requests(state: &State, links: &[&LinkAudit]) -> Result<Vec<ChangeRequest>, Error> {
    let mut requests = Vec::new();
    let mut covered = BTreeSet::new();
    for link in links {
        let scope = scope(link);
        for reason in &link.reasons {
            let InstallReason::Preset(id) = reason else {
                continue;
            };
            let application = PresetApplication {
                preset: id.clone(),
                scope: scope.clone(),
            };
            // Preserve independently installed preset reasons for IDs never defined in state.
            if let Some(preset) = state.preset(id) {
                if !state.preset_applications.contains(&application)
                    || !preset.members.contains(&link.artifact)
                {
                    push_request(
                        &mut requests,
                        ChangeRequest::UnlinkManaged {
                            destination: link.destination.clone(),
                            reason: reason.clone(),
                        },
                    )?;
                }
                if link.status == Status::Managed
                    && let Some(target) = relative_target(link)
                {
                    covered.insert((application, link.artifact.as_str(), target));
                }
            }
        }
    }
    for application in &state.preset_applications {
        if matches!(&application.scope, InstallScope::Project { root } if !state.projects.contains_key(root))
        {
            continue;
        }
        let Some(preset) = state.preset(&application.preset) else {
            continue;
        };
        let global = matches!(application.scope, InstallScope::Global { .. });
        let targets = state.targets().paths(global);
        for artifact in &preset.members {
            for target in &targets {
                if !covered.contains(&(application.clone(), artifact.as_str(), target.as_path())) {
                    push_request(
                        &mut requests,
                        ChangeRequest::InstallScoped {
                            scope: application.scope.clone(),
                            target: Some(target.clone()),
                            artifact: artifact.clone(),
                            reason: InstallReason::Preset(preset.id.clone()),
                        },
                    )?;
                }
            }
        }
    }
    Ok(requests)
}

fn push_request(requests: &mut Vec<ChangeRequest>, request: ChangeRequest) -> Result<(), Error> {
    if requests.len() == MAX_PLAN_CHANGES {
        return Err(invalid("too many changes in one synchronization plan"));
    }
    requests.push(request);
    Ok(())
}
