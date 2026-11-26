//! Deterministic priority and canonical identity selection.
use crate::{Error, Intelligence, intelligence};
use ai_catalog::{
    AiCatalog, CatalogEntry, MEDIA_TYPE_AGENT_SKILLS_GZIP, MEDIA_TYPE_AGENT_SKILLS_JSON,
    MEDIA_TYPE_AGENT_SKILLS_MARKDOWN, MEDIA_TYPE_AGENT_SKILLS_ZIP,
};
use std::collections::{BTreeMap, BTreeSet};
/// Input from an explicitly approved registry. Smaller priority means higher precedence.
#[derive(Debug, Clone)]
pub struct RegistryInput {
    /// Locally configured registry identity.
    pub id: String,
    /// Priority rank.
    pub priority: u32,
    /// Explicit approval gate.
    pub approved: bool,
    /// Enabled state.
    pub enabled: bool,
    /// SDK-validated catalog.
    pub catalog: AiCatalog,
}
/// Chosen skill and all alternative registry/release occurrences.
#[derive(Debug, Clone)]
pub struct SelectedSkill {
    /// Winning registry.
    pub registry: String,
    /// Selected SDK entry.
    pub entry: CatalogEntry,
    /// All alternative source/release occurrences.
    pub alternatives: Vec<(String, CatalogEntry)>,
}
/// Effective immutable catalog index.
#[derive(Debug, Default)]
pub struct CatalogIndex {
    /// Canonical skill ID to selected representation.
    pub skills: BTreeMap<String, SelectedSkill>,
    /// Approved, enabled registries represented by the supplied snapshots.
    pub registries: BTreeSet<String>,
    /// Active intelligence in priority order.
    pub intelligence: Vec<(u32, Intelligence)>,
    /// Resolvable technology identifiers.
    pub technologies: BTreeSet<String>,
    /// Nonfatal diagnostics.
    pub warnings: Vec<String>,
}
impl CatalogIndex {
    /// Build from approved enabled inputs; ties use registry ID and highest lexical version.
    /// The lexical policy is explicit, provisional, and never interprets arbitrary versions as `SemVer`.
    /// # Errors
    /// Invalid extension data or mismatched registry identity fails closed.
    pub fn build(mut inputs: Vec<RegistryInput>, extension_key: &str) -> Result<Self, Error> {
        inputs.sort_by(|a, b| (a.priority, &a.id).cmp(&(b.priority, &b.id)));
        let mut index = Self::default();
        for input in inputs.into_iter().filter(|x| x.enabled && x.approved) {
            index.registries.insert(input.id.clone());
            let mut entries = input.catalog.entries.clone();
            entries.sort_by(|a, b| {
                a.identifier
                    .cmp(&b.identifier)
                    .then_with(|| b.version.cmp(&a.version))
            });
            for entry in entries.into_iter().filter(|e| {
                matches!(
                    e.entry_type.as_str(),
                    MEDIA_TYPE_AGENT_SKILLS_MARKDOWN
                        | MEDIA_TYPE_AGENT_SKILLS_ZIP
                        | MEDIA_TYPE_AGENT_SKILLS_GZIP
                        | MEDIA_TYPE_AGENT_SKILLS_JSON
                )
            }) {
                let id = entry.identifier.clone();
                if let Some(selected) = index.skills.get_mut(&id) {
                    selected.alternatives.push((input.id.clone(), entry));
                } else {
                    index.skills.insert(
                        id,
                        SelectedSkill {
                            registry: input.id.clone(),
                            entry,
                            alternatives: Vec::new(),
                        },
                    );
                }
            }
            if let Some(mut intel) = intelligence(&input.catalog, extension_key)? {
                if intel.registry_id != input.id {
                    return Err(Error::Invalid("registry identity mismatch".into()));
                }
                let prefix = format!("{}:technology.", input.id);
                intel.technologies.retain(|t| {
                    if t.id.starts_with(&prefix) {
                        index.technologies.insert(t.id.clone());
                        true
                    } else {
                        index
                            .warnings
                            .push(format!("unauthorized technology definition: {}", t.id));
                        false
                    }
                });
                index.intelligence.push((input.priority, intel));
            }
        }
        Ok(index)
    }
}
