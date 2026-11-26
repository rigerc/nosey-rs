//! Apply a derived plan under the library mutation lock.

use super::{ChangeDetail, ChangeOutcome, ChangePlan, ChangeRequest, ChangeResult, PlannedChange};
use crate::{
    Error, State,
    library::Library,
    library_io::invalid,
    sync::{Sync, scope},
};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

impl Library {
    pub(super) fn apply_sync(
        &self,
        state: &State,
        approved: &ChangePlan,
        kind: Sync,
    ) -> Result<Vec<ChangeResult>, Error> {
        let _mutation = self.lock_mutation()?;
        let fresh = self.plan_sync(state, kind)?;
        if &fresh != approved {
            return Err(invalid("stale change plan; review again"));
        }

        let links: BTreeMap<_, _> = if kind == Sync::Targets {
            self.audit_links(state)?
                .into_iter()
                .map(|link| (link.destination.clone(), link))
                .collect()
        } else {
            BTreeMap::new()
        };
        let mut installations_failed = BTreeSet::new();
        let mut touched = BTreeSet::new();
        let mut order: Vec<_> = fresh.changes.iter().enumerate().collect();
        order.sort_by_key(|(_, change)| {
            matches!(change.request, ChangeRequest::UnlinkManaged { .. })
        });
        let mut results = vec![ChangeOutcome::Stale; order.len()];
        for (index, change) in order {
            let outcome = match &change.request {
                ChangeRequest::UnlinkManaged {
                    destination,
                    reason,
                } if links.get(destination).is_some_and(|link| {
                    installations_failed.contains(&(scope(link), &link.artifact, reason))
                }) =>
                {
                    ChangeOutcome::Blocked {
                        reason: "an enabled target still needs this installation reason".into(),
                    }
                }
                _ => self.apply_sync_item(state, change, &touched),
            };
            if matches!(outcome, ChangeOutcome::Applied) {
                touched.extend(destination(change));
            } else if let ChangeRequest::InstallScoped {
                scope,
                artifact,
                reason,
                ..
            } = &change.request
            {
                installations_failed.insert((scope.clone(), artifact, reason));
            }
            results[index] = outcome;
        }
        Ok(fresh
            .changes
            .into_iter()
            .zip(results)
            .map(|(change, outcome)| ChangeResult {
                id: change.id,
                outcome,
            })
            .collect())
    }

    fn apply_sync_item(
        &self,
        state: &State,
        reviewed: &PlannedChange,
        touched: &BTreeSet<&Path>,
    ) -> ChangeOutcome {
        if let Some(reason) = &reviewed.blocker {
            return ChangeOutcome::Blocked {
                reason: reason.clone(),
            };
        }
        let current = match self.plan_one(state, reviewed.id.clone(), &reviewed.request) {
            Ok(change) => change,
            Err(error) => {
                return ChangeOutcome::Failed {
                    error: error.to_string(),
                };
            }
        };
        // A preceding approved change may have created the owned link or updated
        // its reasons. Refresh only for that destination, under the mutation lock.
        if current.blocker.is_some()
            || (current.fingerprint != reviewed.fingerprint
                && !destination(reviewed).is_some_and(|path| touched.contains(path)))
        {
            return ChangeOutcome::Stale;
        }
        let applied = match &current.detail {
            Some(ChangeDetail::Install(preview)) => {
                self.apply_install_locked(state, preview).map(drop)
            }
            Some(ChangeDetail::Unlink(preview)) => self.apply_unlink_locked(preview).map(drop),
            _ => Err(invalid("unexpected synchronization change")),
        };
        match applied {
            Ok(()) => ChangeOutcome::Applied,
            Err(error) => ChangeOutcome::Failed {
                error: error.to_string(),
            },
        }
    }
}

fn destination(change: &PlannedChange) -> Option<&Path> {
    match &change.detail {
        Some(ChangeDetail::Install(preview)) => Some(&preview.destination),
        Some(ChangeDetail::Unlink(preview)) => Some(&preview.destination),
        _ => None,
    }
}
