//! Approval revalidation and ordered application.

use super::{ChangeDetail, ChangeOutcome, ChangePlan, ChangeRequest, ChangeResult, PlannedChange};
use crate::{Error, State, library::Library, library_io::invalid};
use nosey_registry::{CatalogIndex, Fetch};
use std::collections::BTreeMap;

use super::acquisition::HttpsFetch;

impl Library {
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
        self.apply_plan_with_catalog_and_fetch(state, approved, None, fetch)
    }

    /// Apply with fresh advisory metadata from the supplied catalog.
    ///
    /// # Errors
    /// Rejects stale approvals before applying any change.
    pub fn apply_plan_with_catalog(
        &self,
        state: &State,
        approved: &ChangePlan,
        catalog: Option<&CatalogIndex>,
    ) -> Result<Vec<ChangeResult>, Error> {
        self.apply_plan_with_catalog_and_fetch(state, approved, catalog, &HttpsFetch)
    }

    /// Apply with an explicit catalog and acquisition transport.
    ///
    /// # Errors
    /// Rejects stale approvals before applying any change.
    pub fn apply_plan_with_catalog_and_fetch(
        &self,
        state: &State,
        approved: &ChangePlan,
        catalog: Option<&CatalogIndex>,
        fetch: &impl Fetch,
    ) -> Result<Vec<ChangeResult>, Error> {
        if let Some(kind) = approved.sync {
            return self.apply_sync(state, approved, kind);
        }
        let fresh = self.plan_with_catalog(state, &approved.requests, catalog)?;
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

    fn apply_one(
        &self,
        state: &State,
        reviewed: &PlannedChange,
        fetch: &impl Fetch,
    ) -> ChangeOutcome {
        if let Some(ChangeDetail::PendingInstall { install, source }) = &reviewed.detail {
            // The install was reviewed against the pinned source. apply_install
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
                self.acquire_source(p.source(), install_name.as_deref(), fetch)
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
