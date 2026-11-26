//! Advisory comparisons against effective skills in the destination scope.

use super::{ChangeDetail, PlannedChange};
use crate::{
    State,
    changes::{InstallPreview, LinkAction, Status},
    inventory::unmanaged_library_links,
    library::Library,
};
use nosey_registry::{CatalogIndex, SkillExtension};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// A nonblocking observation about a declared skill relationship.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Warning {
    /// A known dependency is absent from the scope.
    MissingDependency {
        /// Canonical reference.
        reference: String,
    },
    /// An incompatible skill is present or planned in the scope.
    DeclaredConflict {
        /// Canonical reference.
        reference: String,
    },
    /// The catalog cannot resolve the declared canonical reference.
    UnresolvedReference {
        /// Canonical reference.
        reference: String,
    },
    /// Effective skills could not be fully inspected.
    InventoryUnavailable,
}

fn install(change: &PlannedChange) -> Option<&InstallPreview> {
    match change.detail.as_ref()? {
        ChangeDetail::Install(install) | ChangeDetail::PendingInstall { install, .. } => {
            Some(install)
        }
        _ => None,
    }
}

impl Library {
    pub(super) fn add_advisories(
        &self,
        state: &State,
        changes: &mut [PlannedChange],
        catalog: &CatalogIndex,
    ) {
        let removed: BTreeSet<_> = changes
            .iter()
            .filter_map(|change| match &change.detail {
                Some(ChangeDetail::Unlink(link)) if link.action == LinkAction::RemoveLink => {
                    Some(link.destination.clone())
                }
                _ => None,
            })
            .collect();
        let mut planned = BTreeMap::<_, BTreeSet<_>>::new();
        let mut targets = BTreeMap::<_, BTreeSet<_>>::new();
        for change in changes.iter() {
            if let Some(preview) = install(change) {
                let scope = (preview.global, preview.project.clone());
                targets
                    .entry(scope.clone())
                    .or_default()
                    .insert(preview.target.clone());
                if change.blocker.is_none() {
                    planned
                        .entry(scope)
                        .or_default()
                        .insert(preview.artifact.clone());
                }
            }
        }
        let sources = catalog
            .skills
            .keys()
            .map(|id| (self.object_dir(id).join("skill"), id))
            .collect();
        let links = self.audit_links(state);
        let mut effective = BTreeMap::new();
        for change in changes {
            let Some(preview) = install(change) else {
                continue;
            };
            let Some(selected) = catalog.skills.get(&preview.artifact) else {
                continue;
            };
            let Some(extension) = &selected.extension else {
                continue;
            };
            let registry = match &change.detail {
                Some(ChangeDetail::PendingInstall { source, .. }) => Some(source.registry.clone()),
                _ => self
                    .get(&preview.artifact)
                    .ok()
                    .and_then(|record| record.registry),
            };
            if registry.as_deref() != Some(selected.registry.as_str()) {
                continue;
            }
            let key = (preview.global, preview.project.clone());
            let (present, complete) = effective.entry(key.clone()).or_insert_with(|| {
                let mut present = planned.remove(&key).unwrap_or_default();
                let mut owned = BTreeSet::new();
                if let Ok(links) = &links {
                    for link in links.iter().filter(|link| {
                        link.global == key.0
                            && link.project == key.1
                            && link.status == Status::Managed
                    }) {
                        owned.insert(link.destination.clone());
                        if !removed.contains(&link.destination) {
                            present.insert(link.artifact.clone());
                        }
                    }
                }
                let mut paths = state.targets().paths(key.0);
                paths.extend(targets.remove(&key).unwrap_or_default());
                let mut complete = links.is_ok();
                for path in paths {
                    match unmanaged_library_links(self, &key.1.join(path), &owned, &sources) {
                        Ok(artifacts) => present.extend(artifacts),
                        Err(_) => complete = false,
                    }
                }
                (present, complete)
            });
            change.warnings = relationship_warnings(extension, catalog, present, *complete);
        }
    }
}

fn relationship_warnings(
    extension: &SkillExtension,
    catalog: &CatalogIndex,
    present: &BTreeSet<String>,
    complete: bool,
) -> Vec<Warning> {
    let mut warnings = BTreeSet::new();
    if !complete {
        warnings.insert(Warning::InventoryUnavailable);
    }
    for (reference, dependency) in extension
        .dependencies
        .iter()
        .map(|r| (r, true))
        .chain(extension.conflicts.iter().map(|r| (r, false)))
    {
        let reference = reference.clone();
        if !catalog.skills.contains_key(&reference) {
            warnings.insert(Warning::UnresolvedReference { reference });
        } else if dependency && !present.contains(&reference) && complete {
            warnings.insert(Warning::MissingDependency { reference });
        } else if !dependency && present.contains(&reference) {
            warnings.insert(Warning::DeclaredConflict { reference });
        }
    }
    warnings.into_iter().collect()
}
