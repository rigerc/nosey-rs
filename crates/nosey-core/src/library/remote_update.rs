//! Snapshot-based review and atomic replacement of a catalog bundle.

use super::{
    Acquisition, Availability, BundleSource, Library, SkillRecord, directory, hash, invalid,
};
use crate::{Error, State, UpdatePolicy, library_io::identity};
use nosey_registry::{CatalogIndex, Fetch};
use rustix::fs::RenameFlags;
use serde::Serialize;
use url::Url;

/// A selected catalog entry whose version or acquisition identity differs.
#[derive(Debug, Serialize)]
pub struct AvailableUpdate {
    /// Stable artifact ID.
    pub artifact: String,
    /// Registry currently selected by catalog priority.
    pub registry: String,
    /// URL declared for the selected representation.
    pub url: Option<String>,
    /// Media type declared for the selected representation.
    pub media_type: String,
    /// Digest declared for the selected representation.
    pub digest: Option<String>,
    /// Catalog release selected for replacement.
    pub revision: Option<String>,
    /// Whether the registry, URL authority or media type changed.
    pub provenance_changed: bool,
    /// Verified retrieval metadata, when the selected representation is supported.
    pub source: Option<BundleSource>,
    /// Why the selected representation cannot be installed.
    pub blocker: Option<String>,
}

/// Approval for a single content swap at an existing library path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RemoteUpdatePreview {
    /// Active record at preview time.
    pub current: SkillRecord,
    /// Exact replacement declaration, before verification of downloaded bytes.
    pub source: BundleSource,
    /// Whether the registry, URL authority or media type changed.
    pub provenance_changed: bool,
    /// Approval bound to the replacement and the active filesystem object.
    pub fingerprint: String,
}

/// Result of considering one catalog update under automatic policy.
#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AutomaticUpdateOutcome {
    /// Verified replacement published at the existing path.
    Applied,
    /// Acquisition identity changed and requires explicit review.
    ReviewRequired,
    /// The replacement was blocked or could not be applied.
    Failed {
        /// Reason the item failed.
        error: String,
    },
}

impl Library {
    /// Report catalog changes for intact library bundles, without fetching.
    ///
    /// # Errors
    /// Returns an error if the library inventory cannot be enumerated.
    pub fn available_updates(&self, index: &CatalogIndex) -> Result<Vec<AvailableUpdate>, Error> {
        let _mutation = self.lock_mutation()?;
        let (records, _) = self.cleanup_entries()?;
        Ok(records
            .into_iter()
            .filter_map(|current| {
                let selected = index.skills.get(&current.id)?;
                let Acquisition::Bundle { digest, .. } = &current.acquisition else {
                    return None;
                };
                let entry = &selected.entry;
                let provenance_changed = provenance_changed(
                    &current,
                    &selected.registry,
                    entry.url.as_deref(),
                    &entry.entry_type,
                );
                if !provenance_changed
                    && entry.digest.as_ref() == Some(digest)
                    && entry.version == current.revision
                {
                    return None;
                }
                let (source, blocker) = match BundleSource::from_selected(selected) {
                    Ok(source) => (Some(source), None),
                    Err(error) => (None, Some(error.to_string())),
                };
                Some(AvailableUpdate {
                    artifact: current.id,
                    registry: selected.registry.clone(),
                    url: entry.url.clone(),
                    media_type: entry.entry_type.clone(),
                    digest: entry.digest.clone(),
                    revision: entry.version.clone(),
                    provenance_changed,
                    source,
                    blocker,
                })
            })
            .collect())
    }

    /// Compute fresh availability marks without modifying the library or its links.
    ///
    /// # Errors
    /// Returns an error if the library inventory cannot be enumerated.
    pub fn refresh_availability(&self, index: &CatalogIndex) -> Result<Vec<SkillRecord>, Error> {
        let _mutation = self.lock_mutation()?;
        let (mut records, _) = self.cleanup_entries()?;
        for record in &mut records {
            let Some(registry) = &record.registry else {
                continue;
            };
            record.availability = if index.skills.get(&record.id).is_some_and(|selected| {
                &selected.registry == registry
                    || selected.alternatives.iter().any(|(id, _)| id == registry)
            }) {
                Availability::Available
            } else if index.registries.contains(registry) {
                Availability::MissingFromCatalog
            } else {
                Availability::RegistryRemoved
            };
        }
        Ok(records)
    }

    /// Bind a replacement to the active bundle before fetching any content.
    ///
    /// # Errors
    /// Rejects absent or modified copies, invalid sources and unchanged content declarations.
    pub fn preview_remote_update(
        &self,
        source: &BundleSource,
    ) -> Result<RemoteUpdatePreview, Error> {
        source
            .validate()
            .map_err(|error| invalid(&error.to_string()))?;
        let current = self.get(&source.artifact)?;
        let Acquisition::Bundle { digest, .. } = &current.acquisition else {
            return Err(invalid(
                "remote updates require an installed catalog bundle",
            ));
        };
        let provenance_changed = provenance_changed(
            &current,
            &source.registry,
            Some(&source.url),
            &source.media_type,
        );
        if !provenance_changed && digest == &source.digest && current.revision == source.revision {
            return Err(invalid("remote update has no changes"));
        }
        let object = directory(&self.object_dir(&current.id))?;
        let fingerprint = hash(&serde_json::to_vec(&(
            "remote_update",
            &current,
            source,
            identity(&object)?,
        ))?);
        Ok(RemoteUpdatePreview {
            current,
            source: source.clone(),
            provenance_changed,
            fingerprint,
        })
    }

    /// Verify and atomically publish exactly the reviewed replacement.
    ///
    /// # Errors
    /// Rejects stale approval, retrieval or validation failure, and failed publication.
    pub fn apply_remote_update(
        &self,
        approved: &RemoteUpdatePreview,
        fetch: &impl Fetch,
    ) -> Result<SkillRecord, Error> {
        let _mutation = self.lock_mutation()?;
        let source = &approved.source;
        if &self.preview_remote_update(source)? != approved {
            return Err(invalid("stale remote update approval; review again"));
        }
        let (stage, replacement) =
            self.stage_bundle(source, Some(&approved.current.name), fetch)?;
        if &self.preview_remote_update(source)? != approved {
            return Err(invalid(
                "active bundle changed during staging; review again",
            ));
        }
        self.publish_stage(
            &stage,
            &self.object_dir(&source.artifact),
            RenameFlags::EXCHANGE,
        )?;
        Ok(replacement)
    }

    /// Apply eligible content updates under automatic policy and report each outcome.
    ///
    /// # Errors
    /// Returns an error if the library inventory cannot be enumerated.
    pub fn apply_automatic_updates(
        &self,
        state: &State,
        index: &CatalogIndex,
        fetch: &impl Fetch,
    ) -> Result<Vec<(String, AutomaticUpdateOutcome)>, Error> {
        if state.update_policy != UpdatePolicy::Automatic {
            return Ok(Vec::new());
        }
        Ok(self
            .available_updates(index)?
            .into_iter()
            .map(|update| {
                let outcome = if update.provenance_changed {
                    AutomaticUpdateOutcome::ReviewRequired
                } else if let Some(source) = update.source {
                    match self.preview_remote_update(&source) {
                        Ok(preview) if preview.provenance_changed => {
                            AutomaticUpdateOutcome::ReviewRequired
                        }
                        Ok(preview) => match self.apply_remote_update(&preview, fetch) {
                            Ok(_) => AutomaticUpdateOutcome::Applied,
                            Err(error) => AutomaticUpdateOutcome::Failed {
                                error: error.to_string(),
                            },
                        },
                        Err(error) => AutomaticUpdateOutcome::Failed {
                            error: error.to_string(),
                        },
                    }
                } else {
                    AutomaticUpdateOutcome::Failed {
                        error: update
                            .blocker
                            .unwrap_or_else(|| "missing replacement source".into()),
                    }
                };
                (update.artifact, outcome)
            })
            .collect())
    }
}

fn provenance_changed(
    current: &SkillRecord,
    registry: &str,
    url: Option<&str>,
    media_type: &str,
) -> bool {
    let Acquisition::Bundle {
        url: old_url,
        media_type: old_type,
        ..
    } = &current.acquisition
    else {
        return true;
    };
    let old = Url::parse(old_url).ok();
    let new = url.and_then(|url| Url::parse(url).ok());
    current.registry.as_deref() != Some(registry)
        || old_type != media_type
        || !old
            .zip(new)
            .is_some_and(|(old, new)| old.origin() == new.origin())
}
