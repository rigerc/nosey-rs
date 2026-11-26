//! Previewed atomic replacement of a local fixture object.

use super::{Library, LocalSource, SkillRecord, directory, hash, invalid, tree};
use crate::{Error, library_io::identity};
use rustix::fs::RenameFlags;
use serde::Serialize;
use std::path::Path;

/// The current and proposed records for one stable library object path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UpdatePreview {
    /// The currently verified, active fixture.
    pub current: SkillRecord,
    /// The proposed replacement from the selected local source.
    pub replacement: SkillRecord,
    /// Approval fingerprint binding the old object and replacement content.
    pub fingerprint: String,
}

impl Library {
    /// Preview replacement of an existing intact fixture without staging it.
    ///
    /// # Errors
    ///
    /// Rejects invalid content, absent objects and skill name changes.
    pub fn preview_update(&self, id: &str, source: &Path) -> Result<UpdatePreview, Error> {
        let current = self.get(id)?;
        let source = self.fixture_source(source)?;
        let (tree_sha256, name) = tree(&source, None)?;
        let replacement = SkillRecord {
            id: id.into(),
            name,
            source: LocalSource {
                path: source,
                tree_sha256,
            },
        };
        self.update_plan(current, replacement)
    }

    /// Stage and atomically exchange the exact approved fixture for the active copy.
    ///
    /// # Errors
    ///
    /// Rejects stale approvals, invalid trees and filesystems without exchange support.
    pub fn apply_update(&self, approved: &UpdatePreview) -> Result<SkillRecord, Error> {
        let _mutation = self.lock_mutation()?;
        let id = &approved.current.id;
        if &self.preview_update(id, &approved.replacement.source.path)? != approved {
            return Err(invalid("stale update approval; review again"));
        }
        let (stage, replacement) = self.stage_fixture(id, &approved.replacement.source.path)?;
        let fresh = self.update_plan(self.get(id)?, replacement)?;
        if &fresh != approved {
            return Err(invalid("fixture changed during staging; review again"));
        }
        self.publish_stage(&stage, &self.object_dir(id), RenameFlags::EXCHANGE)?;
        Ok(fresh.replacement)
    }

    fn update_plan(
        &self,
        current: SkillRecord,
        replacement: SkillRecord,
    ) -> Result<UpdatePreview, Error> {
        if current.name != replacement.name {
            return Err(invalid(
                "fixture update must preserve the installed skill name",
            ));
        }
        if current == replacement {
            return Err(invalid("fixture update has no changes"));
        }
        let object = directory(&self.object_dir(&current.id))?;
        let fingerprint = hash(&serde_json::to_vec(&(
            "update",
            &current,
            &replacement,
            identity(&object)?,
        ))?);
        Ok(UpdatePreview {
            current,
            replacement,
            fingerprint,
        })
    }
}
