//! Reference-aware cleanup of the private active-copy library.

use super::{Library, MAX_ENTRIES, MAX_RECORD, SkillRecord, hash, verified_record};
use crate::{
    Error, State,
    changes::{LinkAudit, Status},
    library_io::{directory, identity, invalid, read},
};
use rustix::fs::{RenameFlags, fsync, renameat_with};
use serde::Serialize;
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

const REMOVAL_PREFIX: &str = ".remove-";
const STAGED_OBJECT: &str = "object";

/// Read-only cleanup inventory.
#[derive(Debug, Serialize)]
pub struct CleanupReport {
    /// Entries with no recorded references. External coverage is separate.
    pub unused: Vec<SkillRecord>,
    /// Every retained link or journal reference, including detached projects.
    pub references: Vec<LinkAudit>,
    /// Journal destinations requiring recovery or pointing to unavailable artifacts.
    pub unresolved: Vec<PathBuf>,
    /// Object directories that could not be verified.
    pub unverifiable_objects: Vec<PathBuf>,
    /// Whether the caller supplied a complete external reference assessment.
    pub external_references_checked: bool,
}

/// Review of a single active copy and the references that protect it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RemovePreview {
    /// Verified active copy.
    pub skill: SkillRecord,
    /// Recorded references to the copy.
    pub references: Vec<LinkAudit>,
    /// Whether external references were checked and none remain.
    pub external_references_checked: bool,
    /// Reason the copy cannot be removed.
    pub blocker: Option<&'static str>,
    /// Approval bound to the active copy, its inode and reference assessment.
    pub fingerprint: String,
    #[serde(skip)]
    object_identity: (u64, u64),
}

impl Library {
    /// Inventory entries and all journal references without mutating them.
    ///
    /// # Errors
    ///
    /// Fails if the journal inventory is unreadable or the inventory exceeds its limits.
    pub fn cleanup_report(&self, state: &State) -> Result<CleanupReport, Error> {
        let _mutation = self.lock_mutation()?;
        let references = self.audit_links(state)?;
        let targets: BTreeSet<_> = references.iter().map(|link| &link.source).collect();
        let used: BTreeSet<_> = references
            .iter()
            .map(|link| link.artifact.as_str())
            .collect();
        let unresolved = references
            .iter()
            .filter(|link| link.status == Status::RecoveryRequired)
            .map(|link| link.destination.clone())
            .collect();
        let (entries, unverifiable_objects) = self.cleanup_entries()?;
        Ok(CleanupReport {
            unused: entries
                .into_iter()
                .filter(|entry| {
                    !used.contains(entry.id.as_str())
                        && !targets.contains(&self.object_dir(&entry.id).join("skill"))
                })
                .collect(),
            references,
            unresolved,
            unverifiable_objects,
            external_references_checked: false,
        })
    }

    /// Review entries originating from an exact stored local source path.
    /// This read-only operation preserves every installed copy.
    ///
    /// # Errors
    ///
    /// Fails if either inventory cannot be read.
    pub fn preview_removed_source(
        &self,
        state: &State,
        source: &Path,
    ) -> Result<Vec<RemovePreview>, Error> {
        let _mutation = self.lock_mutation()?;
        let (entries, _) = self.cleanup_entries()?;
        let links = self.audit_links(state)?;
        entries
            .into_iter()
            .filter(|entry| entry.source.path == source)
            .map(|entry| self.removal_preview(entry, &links, false))
            .collect()
    }

    /// Review removal of a verified copy. Set external coverage only after
    /// checking for unmanaged references to this exact object.
    ///
    /// # Errors
    ///
    /// Fails if the object or journal inventory cannot be verified.
    pub fn preview_remove_unused(
        &self,
        state: &State,
        artifact: &str,
        external_references_checked: bool,
    ) -> Result<RemovePreview, Error> {
        let _mutation = self.lock_mutation()?;
        self.removal_preview(
            self.get(artifact)?,
            &self.audit_links(state)?,
            external_references_checked,
        )
    }

    /// Stage, verify and remove the explicitly approved unreferenced copy.
    ///
    /// # Errors
    ///
    /// Rejects stale or blocked approvals and preserves a stage if verification fails.
    pub fn apply_remove_unused(
        &self,
        state: &State,
        approved: &RemovePreview,
    ) -> Result<(), Error> {
        let _mutation = self.lock_mutation()?;
        let artifact = &approved.skill.id;
        let fresh = self.removal_preview(
            self.get(artifact)?,
            &self.audit_links(state)?,
            approved.external_references_checked,
        )?;
        if &fresh != approved {
            return Err(invalid("stale removal approval; review again"));
        }
        if let Some(blocker) = fresh.blocker {
            return Err(invalid(blocker));
        }
        let object = self.object_dir(artifact);
        let leaf = object.file_name().ok_or_else(|| invalid("object name"))?;
        let expected = fresh.object_identity;
        let objects = self.root.join("objects");
        let parent = directory(&objects)?;
        let quarantine = tempfile::Builder::new()
            .prefix(REMOVAL_PREFIX)
            .tempdir_in(&objects)?;
        let stage_fd = directory(quarantine.path())?;
        renameat_with(
            &parent,
            leaf,
            &stage_fd,
            STAGED_OBJECT,
            RenameFlags::NOREPLACE,
        )
        .map_err(std::io::Error::from)?;
        let quarantine = quarantine.keep();
        let staged = quarantine.join(STAGED_OBJECT);
        if identity(&directory(&staged)?)? != expected || verified_record(&staged)? != fresh.skill {
            return Err(invalid("staged object changed; preserved for recovery"));
        }
        fsync(&parent).map_err(std::io::Error::from)?;
        fs::remove_dir_all(quarantine)?;
        fsync(&parent).map_err(std::io::Error::from)?;
        Ok(())
    }

    fn removal_preview(
        &self,
        skill: SkillRecord,
        links: &[LinkAudit],
        external_references_checked: bool,
    ) -> Result<RemovePreview, Error> {
        // The reference inventory is also the extension point for global and preset ownership.
        let source = self.object_dir(&skill.id).join("skill");
        let references: Vec<_> = links
            .iter()
            .filter(|link| link.artifact == skill.id || link.source == source)
            .cloned()
            .collect();
        let blocker = if !references.is_empty() {
            Some("managed or unresolved references still retain the library copy")
        } else if !external_references_checked {
            Some("external references have not been ruled out")
        } else {
            None
        };
        let object = directory(&self.object_dir(&skill.id))?;
        let object_identity = identity(&object)?;
        let fingerprint = hash(&serde_json::to_vec(&(
            "remove_unused_library_skill",
            &skill,
            &references,
            external_references_checked,
            object_identity,
        ))?);
        Ok(RemovePreview {
            skill,
            references,
            external_references_checked,
            blocker,
            fingerprint,
            object_identity,
        })
    }

    fn cleanup_entries(&self) -> Result<(Vec<SkillRecord>, Vec<PathBuf>), Error> {
        let objects = self.root.join("objects");
        let mut entries = Vec::new();
        let mut unverifiable = Vec::new();
        for (index, entry) in fs::read_dir(objects)?.enumerate() {
            if index >= MAX_ENTRIES {
                return Err(invalid("library object inventory limit"));
            }
            let path = entry?.path();
            let record = directory(&path).and_then(|fd| {
                let bytes = read(&fd, Path::new("record.json"), MAX_RECORD)?;
                let record: SkillRecord = serde_json::from_slice(&bytes)?;
                if path != self.object_dir(&record.id) {
                    return Err(invalid("library identity mismatch"));
                }
                self.get(&record.id)
            });
            match record {
                Ok(record) => entries.push(record),
                Err(_) => unverifiable.push(path),
            }
        }
        entries.sort_by(|a, b| a.id.cmp(&b.id));
        unverifiable.sort();
        Ok((entries, unverifiable))
    }
}
