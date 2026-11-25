//! In-memory, read-only search over validated, enabled registry data.
//! Search ranking never affects recommendation priority or installation.

use crate::CatalogIndex;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

/// Current searchable entity types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    /// Standard skill entry.
    Skill,
    /// Technology definition.
    Technology,
    /// Detection rule.
    Rule,
    /// Individual mapping.
    Mapping,
    /// Combination mapping.
    Combination,
}

/// One indexed result with provenance and known facets.
#[derive(Debug, Clone, Serialize)]
pub struct Entity {
    /// Entity type.
    pub kind: Kind,
    /// Canonical or registry-local identifier.
    pub id: String,
    /// Name or fallback identifier.
    pub name: String,
    /// Origin registry, including alternative skill occurrences.
    pub registry: String,
    /// Known technology relationships from explicit mappings.
    pub technologies: BTreeSet<String>,
    /// Known category; absent means not applicable or unknown.
    pub category: Option<String>,
    #[serde(skip)]
    terms: Vec<String>,
}

/// Optional conjunctive search facets.
#[derive(Debug, Default)]
pub struct Filter<'a> {
    /// Entity type.
    pub kind: Option<Kind>,
    /// Origin registry.
    pub registry: Option<&'a str>,
    /// Canonical technology association.
    pub technology: Option<&'a str>,
    /// Known category.
    pub category: Option<&'a str>,
}

/// Search result with a presentation-only relevance score.
#[derive(Debug, Serialize)]
pub struct Hit<'a> {
    /// Indexed entity.
    pub entity: &'a Entity,
    /// Partial-match score, lower is better.
    pub score: usize,
}

/// Immutable index rebuilt from the effective catalog set.
#[derive(Debug, Default)]
pub struct ExploreIndex {
    entities: Vec<Entity>,
}

impl Entity {
    fn new(kind: Kind, id: &str, name: &str, registry: &str) -> Self {
        Self { kind, id: id.into(), name: name.into(), registry: registry.into(),
            technologies: BTreeSet::new(), category: None,
            terms: vec![id.to_lowercase(), name.to_lowercase(), registry.to_lowercase()] }
    }
    fn term(&mut self, value: &str) {
        // Bound each indexed field independently.
        self.terms.push(value.chars().take(4096).collect::<String>().to_lowercase());
    }
}

impl ExploreIndex {
    /// Rebuild from an already approved, validated catalog index.
    #[must_use]
    pub fn build(index: &CatalogIndex) -> Self {
        let mut entities = Vec::new();
        let mut links: BTreeMap<&str, BTreeSet<String>> = BTreeMap::new();
        for (_, intel) in &index.intelligence {
            let reg = intel.registry_id.as_str();
            for tech in &intel.technologies {
                let mut e = Entity::new(Kind::Technology, &tech.id, &tech.name, reg);
                e.technologies.insert(tech.id.clone());
                if !tech.category.is_empty() {
                    e.category = Some(tech.category.clone());
                    e.term(&tech.category);
                }
                for alias in &tech.aliases { e.term(alias); }
                entities.push(e);
            }
            for rule in &intel.rules {
                let mut e = Entity::new(Kind::Rule, &rule.id, &rule.id, reg);
                e.technologies.insert(rule.technology.clone());
                e.term(&rule.technology);
                entities.push(e);
            }
            for mapping in &intel.mappings {
                let mut e = Entity::new(Kind::Mapping, &mapping.id, &mapping.id, reg);
                e.technologies.insert(mapping.technology.clone());
                e.term(&mapping.technology);
                for skill in &mapping.skills {
                    e.term(skill);
                    links.entry(skill).or_default().insert(mapping.technology.clone());
                }
                entities.push(e);
            }
            for combination in &intel.combinations {
                let mut e = Entity::new(Kind::Combination, &combination.id, &combination.id, reg);
                e.technologies.extend(combination.technologies.iter().cloned());
                for tech in &combination.technologies { e.term(tech); }
                for skill in &combination.skills {
                    e.term(skill);
                    links.entry(skill).or_default().extend(combination.technologies.iter().cloned());
                }
                entities.push(e);
            }
        }
        for (id, skill) in &index.skills {
            for (reg, entry) in std::iter::once((&skill.registry, &skill.entry))
                .chain(skill.alternatives.iter().map(|(r,e)| (r,e)))
            {
                let mut entity = Entity::new(Kind::Skill, id, entry.display_name.as_deref().unwrap_or(id), reg);
                if let Some(description) = &entry.description { entity.term(description); }
                for tag in &entry.tags { entity.term(tag); }
                if let Some(tech) = links.get(id.as_str()) {
                    entity.technologies.clone_from(tech);
                    for t in tech { entity.term(t); }
                }
                entities.push(entity);
            }
        }
        Self { entities }
    }

    /// Search with bounded case-insensitive substring or ordered-subsequence
    /// matching. All query words must match; all supplied facets are ANDed.
    /// An unknown category never satisfies an explicit category filter.
    #[must_use]
    pub fn search(&self, query: &str, filter: &Filter<'_>, limit: usize) -> Vec<Hit<'_>> {
        if query.len() > 512 || limit == 0 { return Vec::new(); }
        let lower = query.to_lowercase();
        let words: Vec<&str> = lower.split_whitespace().take(17).collect();
        if words.len() > 16 { return Vec::new(); }
        let mut hits: Vec<_> = self.entities.iter().filter_map(|entity| {
            if filter.kind.is_some_and(|kind| kind != entity.kind)
                || filter.registry.is_some_and(|r| r != entity.registry)
                || filter.technology.is_some_and(|t| !entity.technologies.contains(t))
                || filter.category.is_some_and(|c| entity.category.as_deref() != Some(c))
            {
                return None;
            }
            let mut score = 0_usize;
            for word in &words {
                score = score.saturating_add(entity.terms.iter().filter_map(|term| rank(term, word)).min()?);
            }
            Some(Hit { entity, score })
        }).collect();
        hits.sort_by(|a,b| {
            (a.score, a.entity.name.to_lowercase(), a.entity.kind, &a.entity.id, &a.entity.registry)
                .cmp(&(b.score, b.entity.name.to_lowercase(), b.entity.kind, &b.entity.id, &b.entity.registry))
        });
        hits.truncate(limit.min(1000));
        hits
    }
}

fn rank(term: &str, word: &str) -> Option<usize> {
    if term == word { return Some(0); }
    if term.starts_with(word) { return Some(1); }
    if term.contains(word) { return Some(2); }
    let mut cursor = term.chars();
    let mut skipped = 0_usize;
    for wanted in word.chars() {
        loop {
            if cursor.next()? == wanted { break; }
            skipped = skipped.saturating_add(1);
        }
    }
    Some(10 + skipped)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Intelligence, Technology};

    #[test]
    fn fuzzy_alias_and_combined_facets() {
        let index = CatalogIndex {
            intelligence: vec![(0, Intelligence {
                schema_version: "1.0.0".into(), registry_id: "fixture".into(),
                technologies: vec![Technology {
                    id: "fixture:technology.example".into(), name: "Example Framework".into(),
                    category: "framework".into(), aliases: vec!["exframe".into()],
                }],
                rules: vec![], mappings: vec![], combinations: vec![],
            })], ..Default::default()
        };
        let explore = ExploreIndex::build(&index);
        let filter = Filter { kind: Some(Kind::Technology), registry: Some("fixture"),
            category: Some("framework"), ..Default::default() };
        assert_eq!(explore.search("exfrm", &filter, 20).len(), 1);
        let wrong = Filter { category: Some("database"), ..filter };
        assert!(explore.search("example", &wrong, 20).is_empty());
    }
}
