//! Selective invalidation and saved-scan recommendation orchestration.
//! This module does not perform network requests or filesystem scans.

use crate::{Error, Project, State};
use nosey_recommendation::{Evaluation, TargetState, evaluate, removal_candidates};
use nosey_registry::CatalogIndex;
use nosey_scanner::{Completeness, Settings, fingerprints};
use serde::Serialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

/// Why saved observations cannot be treated as current.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Freshness {
    /// No usable scan has been saved.
    Missing,
    /// A complete scan matches the current effective rule set and settings.
    Current,
    /// Last scan attempt was incomplete, failed, or cancelled.
    Incomplete,
    /// Effective rule or technology definitions changed.
    RulesChanged,
    /// Global scan preferences changed.
    SettingsChanged,
}

/// Mapping evaluation with explicit observation freshness and removal candidates.
#[derive(Debug, Serialize)]
pub struct SavedEvaluation {
    /// Status of the saved observations.
    pub freshness: Freshness,
    /// Recommendations from surviving observations, possibly historical.
    pub evaluation: Evaluation,
    /// Suggestions only; never executes an unlink.
    pub removal_candidates: BTreeSet<String>,
}

impl Project {
    /// Determine freshness without opening project files.
    ///
    /// # Errors
    ///
    /// Returns a fingerprint serialization error.
    pub fn freshness(&self, index: &CatalogIndex, settings: &Settings) -> Result<Freshness, Error> {
        let Some(scan) = self.scan.as_ref() else {
            return Ok(Freshness::Missing);
        };
        let (rule_digest, settings_digest) = fingerprints(index, settings)?;
        if scan.rule_digest != rule_digest {
            return Ok(Freshness::RulesChanged);
        }
        if scan.settings_digest != settings_digest {
            return Ok(Freshness::SettingsChanged);
        }
        if scan.completeness != Completeness::Complete
            || self.last_attempt != Some(Completeness::Complete)
        {
            return Ok(Freshness::Incomplete);
        }
        Ok(Freshness::Current)
    }

    /// Reevaluate mappings and priorities using saved observations.
    ///
    /// Connectivity is supplied by the runtime's global connectivity service.
    /// It is not inferred from the cache or from one registry response.
    /// Historical observations are visibly labeled; only a current complete
    /// scan can generate removal candidates. No target identity is inferred.
    ///
    /// # Errors
    ///
    /// Rejects offline operation, a missing scan, or fingerprint failure.
    pub fn reevaluate(
        &self,
        index: &CatalogIndex,
        settings: &Settings,
        online: bool,
        targets: &BTreeSet<String>,
        coverage: &BTreeMap<String, BTreeMap<String, TargetState>>,
        previous_mapping_only: &BTreeSet<String>,
    ) -> Result<SavedEvaluation, Error> {
        if !online {
            return Err(Error::Invalid(
                "recommendation evaluation unavailable offline".into(),
            ));
        }
        let freshness = self.freshness(index, settings)?;
        let Some(scan) = self.scan.as_ref() else {
            return Err(Error::Invalid("no saved scan".into()));
        };
        let evaluation = evaluate(index, scan, &self.dismissed, targets, coverage);
        let removal_candidates = removal_candidates(
            scan,
            freshness == Freshness::Current,
            online,
            previous_mapping_only,
            &evaluation,
        );
        Ok(SavedEvaluation {
            freshness,
            evaluation,
            removal_candidates,
        })
    }

    /// Persist an explicit dismissal in this project record.
    ///
    /// # Errors
    ///
    /// Rejects empty, oversized or control-bearing canonical identifiers.
    pub fn dismiss(&mut self, canonical_id: &str) -> Result<bool, Error> {
        if canonical_id.is_empty()
            || canonical_id.len() > 4096
            || canonical_id.chars().any(char::is_control)
        {
            return Err(Error::Invalid("invalid canonical skill identifier".into()));
        }
        Ok(self.dismissed.insert(canonical_id.to_owned()))
    }

    /// Restore a dismissed identifier without installing anything.
    pub fn restore(&mut self, canonical_id: &str) -> bool {
        self.dismissed.remove(canonical_id)
    }
}

impl State {
    /// Get a registered project by its canonical root without scanning.
    ///
    /// # Errors
    ///
    /// Returns an error if the root is not registered.
    pub fn project_mut(&mut self, root: &Path) -> Result<&mut Project, Error> {
        self.projects
            .get_mut(root)
            .ok_or_else(|| Error::Invalid("project not registered".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nosey_registry::{Intelligence, Mapping, Rule, RuleKind, Technology};
    use nosey_scanner::ScanResult;

    fn intelligence(id: &str) -> Intelligence {
        let tech = format!("{id}:technology.example");
        Intelligence {
            schema_version: "1.0.0".into(),
            registry_id: id.into(),
            technologies: vec![Technology {
                id: tech.clone(),
                name: "Example".into(),
                category: String::new(),
                aliases: vec![],
            }],
            rules: vec![Rule {
                id: format!("{id}:rule.example"),
                technology: tech,
                disabled: false,
                matcher: RuleKind::Extension {
                    extension: ".demo".into(),
                },
            }],
            mappings: vec![],
            combinations: vec![],
        }
    }

    #[test]
    fn invalidates_only_detection_changes_and_guards_removals() -> Result<(), Error> {
        let a = intelligence("a");
        let b = intelligence("b");
        let mut index = CatalogIndex {
            technologies: a
                .technologies
                .iter()
                .chain(&b.technologies)
                .map(|t| t.id.clone())
                .collect(),
            intelligence: vec![(0, a), (1, b)],
            ..Default::default()
        };
        let settings = Settings::default();
        let (rule_digest, settings_digest) = fingerprints(&index, &settings)?;
        let mut project = Project {
            scan: Some(ScanResult {
                completeness: Completeness::Complete,
                technologies: BTreeMap::new(),
                diagnostics: BTreeSet::new(),
                rule_digest,
                settings_digest,
            }),
            last_attempt: Some(Completeness::Complete),
            ..Default::default()
        };
        assert_eq!(project.freshness(&index, &settings)?, Freshness::Current);
        index.intelligence[0].1.mappings.push(Mapping {
            id: "a:map.demo".into(),
            technology: "a:technology.example".into(),
            skills: vec![],
        });
        index.intelligence.swap(0, 1);
        assert_eq!(project.freshness(&index, &settings)?, Freshness::Current);
        let former = BTreeSet::from(["urn:example:former".into()]);
        let result = project.reevaluate(
            &index,
            &settings,
            true,
            &BTreeSet::new(),
            &BTreeMap::new(),
            &former,
        )?;
        assert_eq!(result.removal_candidates, former);
        assert!(
            project
                .reevaluate(
                    &index,
                    &settings,
                    false,
                    &BTreeSet::new(),
                    &BTreeMap::new(),
                    &former
                )
                .is_err()
        );
        project.last_attempt = Some(Completeness::Cancelled);
        assert_eq!(project.freshness(&index, &settings)?, Freshness::Incomplete);
        assert!(
            project
                .reevaluate(
                    &index,
                    &settings,
                    true,
                    &BTreeSet::new(),
                    &BTreeMap::new(),
                    &former
                )?
                .removal_candidates
                .is_empty()
        );
        let new_settings = Settings {
            max_depth: 10,
            ..settings.clone()
        };
        assert_eq!(
            project.freshness(&index, &new_settings)?,
            Freshness::SettingsChanged
        );
        index.intelligence[0].1.rules[0].disabled = true;
        assert_eq!(
            project.freshness(&index, &settings)?,
            Freshness::RulesChanged
        );
        assert!(project.dismiss("urn:example:skill")?);
        assert!(!project.dismiss("urn:example:skill")?);
        assert!(project.restore("urn:example:skill"));
        Ok(())
    }
}
