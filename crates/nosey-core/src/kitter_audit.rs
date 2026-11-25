//! Read-only development adapter for the pinned Kitter project's JSON audit.
//!
//! The provider identifiers and reported managed flags are deliberately not
//! AI Catalog identities or proof of Nosey ownership. This module cannot
//! adopt, install, remove, or execute any supplied data.

use crate::{Error, State};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
};

const MAX_AUDIT_BYTES: u64 = 8 * 1024 * 1024;

/// A direct installation reported by Kitter; ownership is not verified.
#[derive(Debug, Deserialize, Serialize)]
pub struct DirectInstallation {
    /// Kitter's target ID.
    pub target: String,
    /// Path reported by Kitter.
    pub path: PathBuf,
    /// Upstream heuristic, not proof of Nosey ownership.
    #[serde(rename(deserialize = "managed", serialize = "upstream_managed"))]
    pub upstream_managed: bool,
}

/// Group of direct installations reported by Kitter.
#[derive(Debug, Deserialize, Serialize)]
pub struct DirectSkill {
    /// Upstream display name; not a canonical artifact identifier.
    pub name: String,
    /// Direct installations, retaining target distinctions.
    pub installations: Vec<DirectInstallation>,
}

/// Upstream source kind of an effective skill.
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SkillSource {
    /// Filesystem-discovered.
    Filesystem,
    /// Built into the agent.
    Builtin,
    /// Agent plugin contribution.
    Plugin {
        /// Provider plugin identifier, not an AI Catalog identity.
        id: String,
        /// Plugin display name.
        display_name: String,
    },
}

/// One provider-reported effective skill, with descriptive text discarded.
#[derive(Debug, Deserialize, Serialize)]
pub struct EffectiveSkill {
    /// Provider registration ID, not a canonical artifact identifier.
    pub id: String,
    /// Provider display name.
    pub name: String,
    /// Skill path as reported by the provider.
    pub path: PathBuf,
    /// Discovery root as reported by the provider.
    pub root_path: Option<PathBuf>,
    /// Upstream scope.
    pub scope: Scope,
    /// Upstream visibility.
    pub visibility: Visibility,
    /// Provider source.
    pub source: SkillSource,
}

/// Upstream effective-skill scope.
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Scope {
    /// Local project scope.
    Local,
    /// Repository scope.
    Repository,
    /// User-wide scope.
    User,
    /// System scope.
    System,
}

/// Upstream effective-skill visibility.
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Visibility {
    /// Automatically exposed.
    Automatic,
    /// Only its name is exposed.
    NameOnly,
    /// Only manually available.
    ManualOnly,
    /// Conditionally available.
    Conditional,
}

/// One upstream agent's inspection result.
#[derive(Debug, Deserialize, Serialize)]
pub struct AgentAudit {
    /// Agent registration ID.
    pub agent: String,
    /// Upstream informational estimate, not an installation budget.
    pub estimated_tokens: usize,
    /// Provider-reported skills.
    pub skills: Vec<EffectiveSkill>,
}

/// Untrusted read-only snapshot, never a source of mutation authority.
#[derive(Debug, Deserialize, Serialize)]
pub struct ProjectAudit {
    /// Project directory in the original snapshot.
    pub path: PathBuf,
    /// Reported direct installations.
    pub direct_installations: Vec<DirectSkill>,
    /// Reported effective skills per agent.
    pub agents: Vec<AgentAudit>,
}

impl ProjectAudit {
    /// Read a bounded Kitter audit for an explicitly registered project.
    ///
    /// Uses the pinned CLI's ProjectOutput shape; unknown descriptive fields
    /// are ignored. It never executes Kitter or opens reported skill paths.
    ///
    /// # Errors
    ///
    /// Rejects unregistered projects, mismatched snapshot roots, symbolic-link
    /// inputs, oversize files and malformed upstream data.
    pub fn load(state: &State, project: &Path, snapshot: &Path) -> Result<Self, Error> {
        let root = project.canonicalize()?;
        if !state.projects.contains_key(&root) {
            return Err(Error::Invalid("project not registered".into()));
        }
        let metadata = fs::symlink_metadata(snapshot)?;
        if !metadata.is_file() || metadata.len() > MAX_AUDIT_BYTES {
            return Err(Error::Invalid("invalid Kitter audit file".into()));
        }
        let mut bytes = Vec::new();
        fs::File::open(snapshot)?
            .take(MAX_AUDIT_BYTES + 1)
            .read_to_end(&mut bytes)?;
        if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > MAX_AUDIT_BYTES {
            return Err(Error::Invalid("Kitter audit size limit".into()));
        }
        let audit: Self = serde_json::from_slice(&bytes)?;
        if audit.path != root {
            return Err(Error::Invalid("Kitter audit project mismatch".into()));
        }
        Ok(audit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn preserves_provider_identity_without_claiming_canonical_ownership()
        -> Result<(), Box<dyn std::error::Error>>
    {
        let temp = tempfile::tempdir()?;
        let root = temp.path().join("project");
        fs::create_dir(&root)?;
        let mut state = State::default();
        let root = state.register(&root)?;
        let snapshot = temp.path().join("audit.json");
        let data = json!({
            "path": root,
            "direct_installations": [{
                "name": "demo",
                "installations": [{"target":"codex","path":"/fixture/demo","managed":true}]
            }],
            "agents": [{
                "agent": "codex",
                "estimated_tokens": 42,
                "skills": [{
                    "id":"demo","name":"demo","description":"discard this text",
                    "path":"/fixture/demo","root_path":"/fixture",
                    "scope":"local","visibility":"automatic",
                    "source":{"type":"filesystem"}
                }]
            }]
        });
        fs::write(&snapshot, serde_json::to_vec(&data)?)?;
        let audit = ProjectAudit::load(&state, &root, &snapshot)?;
        let output = serde_json::to_value(audit)?;
        assert_eq!(output["direct_installations"][0]["installations"][0]["upstream_managed"], true);
        assert!(output["agents"][0]["skills"][0].get("description").is_none());
        assert!(output["agents"][0]["skills"][0].get("canonical_id").is_none());
        let mut other = State::default();
        assert!(ProjectAudit::load(&other, &root, &snapshot).is_err());
        let other_root = temp.path().join("other");
        fs::create_dir(&other_root)?;
        let other_root = other.register(&other_root)?;
        assert!(ProjectAudit::load(&other, &other_root, &snapshot).is_err());
        Ok(())
    }
}
