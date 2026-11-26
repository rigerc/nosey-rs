//! Nosey extension models and rule grammar from the pinned Nosey-9 evaluator.
use crate::Error;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Registry-provided technology metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Technology {
    /// Canonical namespaced ID.
    pub id: String,
    /// Display label.
    pub name: String,
    /// Optional grouping.
    #[serde(default)]
    pub category: String,
    /// Alternative names.
    #[serde(default)]
    pub aliases: Vec<String>,
}
/// Exactly the audited upstream rule kinds.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RuleKind {
    /// Match a path glob or regular expression.
    File {
        #[serde(default)]
        glob: String,
        #[serde(default, rename = "pathRegex")]
        path_regex: String,
        #[serde(default, rename = "pathScope")]
        path_scope: String,
    },
    /// Match a path suffix.
    Extension { extension: String },
    /// Match bounded file contents.
    Content {
        glob: String,
        #[serde(default)]
        contains: String,
        #[serde(default)]
        regex: String,
        #[serde(default, rename = "regexFlags")]
        regex_flags: String,
    },
    /// Match names, never values, of dotenv keys.
    Dotenv {
        #[serde(default)]
        glob: String,
        #[serde(default)]
        key: String,
        #[serde(default, rename = "keyRegex")]
        key_regex: String,
    },
    /// Match a parsed ecosystem dependency.
    Dependency {
        ecosystem: String,
        #[serde(default)]
        name: String,
        #[serde(default, rename = "nameRegex")]
        name_regex: String,
    },
    /// Match a configured MCP package, without running it.
    McpPackage {
        #[serde(default)]
        name: String,
        #[serde(default, rename = "nameRegex")]
        name_regex: String,
    },
    /// External detector; unavailable until explicitly approved adapter execution is implemented.
    Detector { detector: String },
    /// Forward-compatible unknown operator: never execute.
    #[serde(other)]
    Unknown,
}
/// Detection rule linked to a canonical technology.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    /// Stable rule ID.
    pub id: String,
    /// Technology reference.
    pub technology: String,
    /// Disabled rules are not evaluated.
    #[serde(default)]
    pub disabled: bool,
    /// Operator and arguments.
    #[serde(flatten)]
    pub matcher: RuleKind,
}
/// Explicit technology-to-skill association.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mapping {
    /// Mapping ID.
    pub id: String,
    /// Technology reference.
    pub technology: String,
    /// Canonical skill identifiers.
    pub skills: Vec<String>,
}
/// Conjunctive technology-to-skill association.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Combination {
    /// Combination ID.
    pub id: String,
    /// All required technologies.
    pub technologies: Vec<String>,
    /// Canonical skill identifiers.
    pub skills: Vec<String>,
}
/// Versioned catalog extension; standard catalog entries remain in SDK models.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Intelligence {
    /// Extension version.
    pub schema_version: String,
    /// Approved registry identifier.
    pub registry_id: String,
    /// Technology definitions.
    #[serde(default)]
    pub technologies: Vec<Technology>,
    /// Rules.
    #[serde(default)]
    pub rules: Vec<Rule>,
    /// Individual mappings.
    #[serde(default)]
    pub mappings: Vec<Mapping>,
    /// Combination mappings.
    #[serde(default)]
    pub combinations: Vec<Combination>,
}
impl Intelligence {
    /// Validate local definitions and duplicates. External references are resolved by the index.
    /// # Errors
    /// Invalid or duplicate IDs and empty combinations are rejected.
    pub fn validate(&self) -> Result<(), Error> {
        fn unique<'a>(ids: impl Iterator<Item = &'a str>) -> bool {
            let mut seen = BTreeSet::new();
            ids.into_iter()
                .all(|id| !id.is_empty() && !id.chars().any(char::is_control) && seen.insert(id))
        }
        if self.registry_id.is_empty()
            || !unique(self.technologies.iter().map(|x| x.id.as_str()))
            || !unique(self.rules.iter().map(|x| x.id.as_str()))
            || !unique(self.mappings.iter().map(|x| x.id.as_str()))
            || !unique(self.combinations.iter().map(|x| x.id.as_str()))
            || self.combinations.iter().any(|x| x.technologies.is_empty())
        {
            return Err(Error::Invalid(
                "duplicate/empty entity ID or empty combination".into(),
            ));
        }
        Ok(())
    }
}
