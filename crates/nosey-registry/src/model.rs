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
        /// Glob selecting candidate paths.
        #[serde(default)]
        glob: String,
        /// Regular expression matched against the path.
        #[serde(default, rename = "pathRegex")]
        path_regex: String,
        /// Scope used when matching candidate paths.
        #[serde(default, rename = "pathScope")]
        path_scope: String,
    },
    /// Match a path suffix.
    Extension {
        /// Path suffix to match.
        extension: String,
    },
    /// Match bounded file contents.
    Content {
        /// Glob selecting candidate paths.
        glob: String,
        /// Literal text to match in file contents.
        #[serde(default)]
        contains: String,
        /// Regular expression matched against file contents.
        #[serde(default)]
        regex: String,
        /// Flags for the content regular expression.
        #[serde(default, rename = "regexFlags")]
        regex_flags: String,
    },
    /// Match names, never values, of dotenv keys.
    Dotenv {
        /// Glob selecting candidate paths.
        #[serde(default)]
        glob: String,
        /// Exact dotenv key name.
        #[serde(default)]
        key: String,
        /// Regular expression matched against dotenv key names.
        #[serde(default, rename = "keyRegex")]
        key_regex: String,
    },
    /// Match a parsed ecosystem dependency.
    Dependency {
        /// Dependency ecosystem to search.
        ecosystem: String,
        /// Exact package name.
        #[serde(default)]
        name: String,
        /// Regular expression matched against package names.
        #[serde(default, rename = "nameRegex")]
        name_regex: String,
    },
    /// Match a configured MCP package, without running it.
    McpPackage {
        /// Exact package name.
        #[serde(default)]
        name: String,
        /// Regular expression matched against package names.
        #[serde(default, rename = "nameRegex")]
        name_regex: String,
    },
    /// External detector; unavailable until explicitly approved adapter execution is implemented.
    Detector {
        /// External detector identifier.
        detector: String,
    },
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
