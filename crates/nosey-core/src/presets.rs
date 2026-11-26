//! Desired preset membership and explicit applications.

use crate::{Error, State, targets::InstallScope};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Longest accepted preset identity in an install reason.
pub(crate) const MAX_PRESET_ID: usize = 128;
const MAX_DISPLAY_NAME: usize = 256;
const MAX_ARTIFACT_ID: usize = 4096;

/// A named set of library artifact IDs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Preset {
    /// Stable identifier also used as the link ownership reason.
    pub id: String,
    /// User-facing name.
    pub name: String,
    /// Desired library artifacts.
    pub members: BTreeSet<String>,
}

/// A preset linked to one installation scope.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PresetApplication {
    /// Identifier of the preset.
    pub preset: String,
    /// Scope where its members are desired.
    pub scope: InstallScope,
}

fn valid_text(text: &str, limit: usize) -> bool {
    !text.trim().is_empty() && text.len() <= limit && !text.chars().any(char::is_control)
}

impl Preset {
    pub(crate) fn validate(&self) -> Result<(), Error> {
        if !valid_text(&self.id, MAX_PRESET_ID)
            || !valid_text(&self.name, MAX_DISPLAY_NAME)
            || self
                .members
                .iter()
                .any(|id| !valid_text(id, MAX_ARTIFACT_ID))
        {
            return Err(Error::Invalid("invalid preset id, name, or member".into()));
        }
        Ok(())
    }
}

impl State {
    /// Look up the desired definition by its stable ID.
    #[must_use]
    pub fn preset(&self, id: &str) -> Option<&Preset> {
        self.presets.get(id)
    }

    /// Define or replace a preset. Existing applications follow its desired membership.
    ///
    /// # Errors
    /// Rejects invalid identifiers and display names.
    pub fn set_preset(&mut self, preset: Preset) -> Result<(), Error> {
        preset.validate()?;
        self.presets.insert(preset.id.clone(), preset);
        Ok(())
    }

    /// Mark or unmark a preset as desired in a scope. Link changes require a sync plan.
    ///
    /// # Errors
    /// Rejects unknown presets and invalid or unregistered scope directories.
    pub fn set_preset_application(
        &mut self,
        mut application: PresetApplication,
        enabled: bool,
    ) -> Result<(), Error> {
        if self.preset(&application.preset).is_none() {
            return Err(Error::Invalid("unknown preset".into()));
        }
        let path = match &mut application.scope {
            InstallScope::Project { root } => root,
            InstallScope::Global { home } => home,
        };
        if !path.is_absolute()
            || path
                .to_str()
                .is_none_or(|path| path.chars().any(char::is_control))
        {
            return Err(Error::Invalid(
                "preset scope requires an absolute path".into(),
            ));
        }
        if enabled {
            *path = path.canonicalize()?;
            if path
                .to_str()
                .is_none_or(|path| path.chars().any(char::is_control))
                || !path.is_dir()
                || matches!(&application.scope, InstallScope::Project { root }
                    if !self.projects.contains_key(root))
            {
                return Err(Error::Invalid("invalid preset scope directory".into()));
            }
            self.preset_applications.insert(application);
        } else {
            if let Ok(canonical) = path.canonicalize() {
                *path = canonical;
            }
            self.preset_applications.remove(&application);
        }
        Ok(())
    }

    pub(crate) fn validate_presets(&self) -> Result<(), Error> {
        for (id, preset) in &self.presets {
            preset.validate()?;
            if id != &preset.id {
                return Err(Error::Invalid("preset identity mismatch".into()));
            }
        }
        for application in &self.preset_applications {
            let root = match &application.scope {
                InstallScope::Project { root } => root,
                InstallScope::Global { home } => home,
            };
            if !root.is_absolute()
                || root
                    .to_str()
                    .is_none_or(|path| path.chars().any(char::is_control))
                || !self.presets.contains_key(&application.preset)
            {
                return Err(Error::Invalid("invalid preset application".into()));
            }
        }
        Ok(())
    }
}
