//! Pure additive recommendation evaluation; never performs installation or filesystem mutation.
#![forbid(unsafe_code)]
use nosey_registry::CatalogIndex;
use nosey_scanner::{Completeness, ScanResult};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
/// Mapping provenance.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Reason {
    /// Combination match precedes individual match.
    pub combination: bool,
    /// Mapping registry priority.
    pub priority: u32,
    /// Origin registry.
    pub registry: String,
    /// Mapping or combination identifier.
    pub mapping: String,
}
/// Effective target state as supplied by the caller.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetState {
    /// Not available to this target.
    Missing,
    /// Managed skill.
    Managed,
    /// Unmanaged with reliable canonical provenance.
    Unmanaged,
}
/// Presentation group.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Group {
    /// At least one missing target.
    Missing,
    /// All managed.
    Managed,
    /// At least one unmanaged and none missing.
    Unmanaged,
    /// Explicit dismissal.
    Dismissed,
}
/// Canonical recommendation and all reasons.
#[derive(Debug, Serialize, Deserialize)]
pub struct Recommendation {
    /// Canonical artifact identifier.
    pub id: String,
    /// Selected display name.
    pub name: String,
    /// Selected entry registry.
    pub registry: String,
    /// Contributing matches.
    pub reasons: BTreeSet<Reason>,
    /// Group for display.
    pub group: Group,
    /// Per-target coverage.
    pub coverage: BTreeMap<String, TargetState>,
}
/// Evaluation result and warnings.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Evaluation {
    /// Sorted recommendations.
    pub recommendations: Vec<Recommendation>,
    /// Unresolved references.
    pub warnings: BTreeSet<String>,
}
/// Evaluate all resolvable explicit mappings; combination matches sort first.
#[must_use]
pub fn evaluate(
    index: &CatalogIndex,
    scan: &ScanResult,
    dismissed: &BTreeSet<String>,
    targets: &BTreeSet<String>,
    coverage: &BTreeMap<String, BTreeMap<String, TargetState>>,
) -> Evaluation {
    let mut result = Evaluation::default();
    let mut matches: BTreeMap<String, BTreeSet<Reason>> = BTreeMap::new();
    for (priority, intel) in &index.intelligence {
        let mut add = |id: &str, skills: &[String], combination: bool| {
            for skill in skills {
                if index.skills.contains_key(skill) {
                    matches.entry(skill.clone()).or_default().insert(Reason {
                        combination,
                        priority: *priority,
                        registry: intel.registry_id.clone(),
                        mapping: id.into(),
                    });
                } else {
                    result.warnings.insert(format!("unresolved skill: {skill}"));
                }
            }
        };
        for m in &intel.mappings {
            if index.technologies.contains(&m.technology)
                && scan.technologies.contains_key(&m.technology)
            {
                add(&m.id, &m.skills, false);
            }
        }
        for m in &intel.combinations {
            if !m.technologies.is_empty()
                && m.technologies
                    .iter()
                    .all(|t| index.technologies.contains(t) && scan.technologies.contains_key(t))
            {
                add(&m.id, &m.skills, true);
            }
        }
    }
    for (id, reasons) in matches {
        let Some(s) = index.skills.get(&id) else {
            continue;
        };
        let coverage: BTreeMap<_, _> = targets
            .iter()
            .map(|t| {
                (
                    t.clone(),
                    coverage
                        .get(&id)
                        .and_then(|m| m.get(t))
                        .copied()
                        .unwrap_or(TargetState::Missing),
                )
            })
            .collect();
        let group = if dismissed.contains(&id) {
            Group::Dismissed
        } else if coverage.is_empty() || coverage.values().any(|x| *x == TargetState::Missing) {
            Group::Missing
        } else if coverage.values().any(|x| *x == TargetState::Unmanaged) {
            Group::Unmanaged
        } else {
            Group::Managed
        };
        result.recommendations.push(Recommendation {
            id: id.clone(),
            name: s.entry.display_name.clone().unwrap_or(id),
            registry: s.registry.clone(),
            reasons,
            group,
            coverage,
        });
    }
    fn order(r: &Recommendation) -> (bool, u32, String, &str) {
        let combination = r.reasons.iter().any(|x| x.combination);
        let priority = r
            .reasons
            .iter()
            .filter(|x| x.combination == combination)
            .map(|x| x.priority)
            .min()
            .unwrap_or(u32::MAX);
        (!combination, priority, r.name.to_lowercase(), &r.id)
    }
    result
        .recommendations
        .sort_by(|a, b| order(a).cmp(&order(b)));
    result
}
/// Previously mapped project-owned skills with no other installation reason.
#[must_use]
pub fn removal_candidates(
    scan: &ScanResult,
    current: bool,
    online: bool,
    previous_mapping_only: &BTreeSet<String>,
    evaluation: &Evaluation,
) -> BTreeSet<String> {
    if !current
        || !online
        || scan.completeness != Completeness::Complete
        || !evaluation.warnings.is_empty()
    {
        return BTreeSet::new();
    }
    let now: BTreeSet<_> = evaluation
        .recommendations
        .iter()
        .map(|r| r.id.as_str())
        .collect();
    previous_mapping_only
        .iter()
        .filter(|id| !now.contains(id.as_str()))
        .cloned()
        .collect()
}
