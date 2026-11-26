//! Advisory metadata attached to standard skill entries.

use crate::{Error, extension_major};
use serde::Deserialize;
use serde_json::Value;

/// Default skill extension namespace used by local snapshots.
pub const SKILL_EXTENSION_KEY: &str = "io.example.nosey.skill";

/// Install metadata declared by a registry for a skill.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillExtension {
    /// Directory name declared by the registry.
    pub install_name: String,
    /// Canonical skill identifiers expected in the same scope.
    #[serde(default)]
    pub dependencies: Vec<String>,
    /// Canonical skill identifiers declared incompatible in the same scope.
    #[serde(default)]
    pub conflicts: Vec<String>,
}

/// Decode metadata only for a compatible extension major.
pub fn decode(value: &Value) -> Result<Option<SkillExtension>, Error> {
    if extension_major(value)? != 1 {
        return Ok(None);
    }
    serde_json::from_value(value.clone())
        .map(Some)
        .map_err(|_| Error::Invalid("skill extension model".into()))
}

#[cfg(test)]
mod tests {
    use super::decode;
    use serde_json::json;

    #[test]
    fn accepts_compatible_minor_and_skips_unknown_major() -> Result<(), crate::Error> {
        let mut value = json!({
            "schemaVersion": "1.4.2",
            "installName": "sample",
            "dependencies": ["example:skill.other"],
            "futureField": true
        });
        assert_eq!(
            decode(&value)?.map(|skill| skill.dependencies),
            Some(vec!["example:skill.other".into()])
        );
        value["schemaVersion"] = json!("2.0.0");
        assert!(decode(&value)?.is_none());
        Ok(())
    }
}
