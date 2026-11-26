//! Bounded, read-only project scanner. No project-local state or executable rule execution.
#![forbid(unsafe_code)]
mod bounded_io;
mod jsonc;
mod matcher;
mod mcp;
mod pattern;
mod providers;
mod terraform;
mod versions;
mod yaml_providers;
use globset::{Glob, GlobSetBuilder};
use nosey_registry::{CatalogIndex, Rule};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};
use walkdir::WalkDir;
/// Scan coverage state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Completeness {
    /// All active rules evaluated.
    Complete,
    /// Limits, parser gaps or I/O prevented full evaluation.
    Incomplete,
    /// Explicit cancellation.
    Cancelled,
    /// Root could not be inspected.
    Failed,
}
/// Global scan limits. Values are provisional defaults from the plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    /// Maximum directory depth.
    pub max_depth: usize,
    /// Maximum bytes read per file.
    pub max_bytes: usize,
    /// Overall deadline in seconds.
    pub timeout_secs: u64,
    /// Extra exclusion globs, project-relative.
    pub exclusions: Vec<String>,
    /// Enable passive MCP package detection.
    pub mcp: bool,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            max_depth: 20,
            max_bytes: 5 * 1024 * 1024,
            timeout_secs: 60,
            exclusions: Vec::new(),
            mcp: false,
        }
    }
}
/// Evidence kind from an existing Nosey-9 rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MatchKind {
    /// Path rule.
    File,
    /// File suffix.
    Extension,
    /// Content predicate (no raw content).
    Content,
    /// Dotenv key predicate (no values).
    Dotenv,
    /// Manifest dependency.
    Dependency,
    /// Passive MCP package.
    McpPackage,
}
/// Interpretation of a version observation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VersionKind {
    /// Constraint or selected minimum declared in a manifest.
    ManifestConstraint,
    /// Verified resolved version from a lockfile or installed metadata.
    Resolved,
}
/// A typed version observation; never inferred as a project-wide version.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct VersionObservation {
    /// Meaning of the value.
    pub kind: VersionKind,
    /// Bounded numeric version/constraint text.
    pub value: String,
}
/// Evidence does not include raw contents, environment values or arbitrary snippets.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Evidence {
    /// Relative project path.
    pub path: PathBuf,
    /// Registry identity.
    pub registry: String,
    /// Rule identity.
    pub rule: String,
    /// Optional observed identifier, omitted for content and dotenv values.
    pub identifier: Option<String>,
    /// Match kind; absent in legacy saved scan records.
    #[serde(default)]
    pub match_kind: Option<MatchKind>,
    /// Version information, if a safe manifest constraint was observed.
    #[serde(default)]
    pub version: Option<VersionObservation>,
}
/// Project-wide aggregated scan result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResult {
    /// State of coverage.
    pub completeness: Completeness,
    /// Positive technology observations, sorted by canonical ID.
    pub technologies: BTreeMap<String, BTreeSet<Evidence>>,
    /// Safe diagnostic codes (no raw file content).
    pub diagnostics: BTreeSet<String>,
    /// Effective rule fingerprint.
    pub rule_digest: String,
    /// Scan settings fingerprint.
    pub settings_digest: String,
}
/// Fingerprint rule and technology definitions, excluding mappings and metadata.
/// # Errors
/// Serialization errors are propagated.
pub fn fingerprints(
    index: &CatalogIndex,
    settings: &Settings,
) -> Result<(String, String), serde_json::Error> {
    let mut rules: Vec<_> = index
        .intelligence
        .iter()
        .map(|(_, i)| {
            let mut tech: Vec<_> = i.technologies.iter().collect();
            tech.sort_by(|a, b| a.id.cmp(&b.id));
            let mut active: Vec<_> = i.rules.iter().filter(|r| !r.disabled).collect();
            active.sort_by(|a, b| a.id.cmp(&b.id));
            (&i.registry_id, tech, active)
        })
        .collect();
    rules.sort_by(|a, b| a.0.cmp(b.0));
    Ok((
        format!("{:x}", Sha256::digest(serde_json::to_vec(&("v5", rules))?)),
        format!("{:x}", Sha256::digest(serde_json::to_vec(settings)?)),
    ))
}
fn mark(result: &mut ScanResult, code: &str) {
    if result.completeness == Completeness::Complete {
        result.completeness = Completeness::Incomplete;
    }
    result.diagnostics.insert(code.into());
}
fn expired(
    start: Instant,
    settings: &Settings,
    cancel: &AtomicBool,
    result: &mut ScanResult,
) -> bool {
    if cancel.load(Ordering::Relaxed) {
        result.completeness = Completeness::Cancelled;
        return true;
    }
    if start.elapsed() >= Duration::from_secs(settings.timeout_secs) {
        mark(result, "scan-timeout");
        return true;
    }
    false
}
/// Scan a manually supplied root without following symbolic links.
/// # Errors
/// Invalid settings or serialization failures are returned.
pub fn scan(
    root: &Path,
    index: &CatalogIndex,
    settings: &Settings,
    cancel: &AtomicBool,
) -> Result<ScanResult, String> {
    if !(1..=100).contains(&settings.max_depth)
        || !(1..=16 * 1024 * 1024).contains(&settings.max_bytes)
        || !(1..=600).contains(&settings.timeout_secs)
        || settings.exclusions.len() > 128
    {
        return Err("invalid scan limits".into());
    }
    let (rule_digest, settings_digest) =
        fingerprints(index, settings).map_err(|e| e.to_string())?;
    let mut result = ScanResult {
        completeness: Completeness::Complete,
        technologies: BTreeMap::new(),
        diagnostics: BTreeSet::new(),
        rule_digest,
        settings_digest,
    };
    let mut excludes = GlobSetBuilder::new();
    for glob in &settings.exclusions {
        excludes.add(Glob::new(glob).map_err(|_| "invalid exclusion glob")?);
    }
    let excludes = excludes.build().map_err(|_| "invalid exclusions")?;
    if !root.is_dir()
        || fs::symlink_metadata(root).is_err()
        || fs::symlink_metadata(root).is_ok_and(|m| m.file_type().is_symlink())
    {
        result.completeness = Completeness::Failed;
        result.diagnostics.insert("invalid-root".into());
        return Ok(result);
    }
    let Ok(reader) = bounded_io::RootReader::new(root) else {
        result.completeness = Completeness::Failed;
        result.diagnostics.insert("safe-root-open-failed".into());
        return Ok(result);
    };
    let compiled = compile_rules(index, settings.mcp, &mut result);
    let start = Instant::now();
    let mut count = 0_usize;
    let default_pruned = std::cell::Cell::new(false);
    let custom_pruned = std::cell::Cell::new(false);
    let walker = WalkDir::new(root)
        .follow_links(false)
        .max_depth(settings.max_depth + 1)
        .sort_by_file_name()
        .into_iter()
        .filter_entry(|e| include_entry(e, root, &excludes, &default_pruned, &custom_pruned));
    for item in walker {
        if expired(start, settings, cancel, &mut result) {
            break;
        }
        count += 1;
        if count > 100_000 {
            mark(&mut result, "entry-limit");
            break;
        }
        let Ok(entry) = item else {
            mark(&mut result, "walk-error");
            continue;
        };
        if entry.depth() == 0 {
            continue;
        }
        if entry.depth() > settings.max_depth {
            mark(&mut result, "depth-limit");
            continue;
        }
        if entry.file_type().is_symlink() {
            mark(&mut result, "symlink-skipped");
            continue;
        }
        if !entry.file_type().is_file() && !entry.file_type().is_dir() {
            mark(&mut result, "special-file-skipped");
            continue;
        }
        let Ok(relative) = entry.path().strip_prefix(root) else {
            mark(&mut result, "path-escape");
            continue;
        };
        evaluate_entry(
            &entry,
            relative,
            &reader,
            &compiled,
            &mut result,
            |result| expired(start, settings, cancel, result),
            settings.max_bytes,
        );
    }
    if default_pruned.get() {
        result
            .diagnostics
            .insert("default-exclusions-applied".into());
    }
    if custom_pruned.get() {
        result
            .diagnostics
            .insert("custom-exclusions-applied".into());
    }
    // Cancellation during the final entry or an empty tree is still visible.
    expired(start, settings, cancel, &mut result);
    Ok(result)
}

fn include_entry(
    e: &walkdir::DirEntry,
    root: &Path,
    excludes: &globset::GlobSet,
    default_pruned: &std::cell::Cell<bool>,
    custom_pruned: &std::cell::Cell<bool>,
) -> bool {
    if e.depth() == 0 {
        return true;
    }
    let path = e.path().strip_prefix(root).unwrap_or_else(|_| e.path());
    // Preserve the prior built-in exclusions. Additional names from
    // projectdetect apply only to directories, not similarly named files.
    let built_in = e.file_name().to_str().is_some_and(|s| {
        matches!(
            s,
            ".git" | "node_modules" | "target" | ".venv" | "dist" | "build" | "__pycache__"
        )
    });
    let additional = e.file_type().is_dir()
        && e.file_name().to_str().is_some_and(|s| {
            matches!(
                s,
                ".tox"
                    | ".pytest_cache"
                    | ".mypy_cache"
                    | ".ruff_cache"
                    | ".gradle"
                    | ".terraform"
                    | ".swiftpm"
                    | "zig-out"
                    | "zig-cache"
                    | ".zig-cache"
                    | ".astro"
                    | ".gatsby"
                    | ".docusaurus"
                    | ".bundle"
                    | ".jekyll-cache"
                    | ".sass-cache"
                    | "autom4te.cache"
            )
        });
    if built_in || additional {
        default_pruned.set(true);
        return false;
    }
    if excludes.is_match(path) {
        custom_pruned.set(true);
        return false;
    }
    true
}

fn evaluate_entry(
    entry: &walkdir::DirEntry,
    relative: &Path,
    reader: &bounded_io::RootReader,
    compiled: &[(&str, &Rule, matcher::Matcher)],
    result: &mut ScanResult,
    mut is_expired: impl FnMut(&mut ScanResult) -> bool,
    max_bytes: usize,
) {
    let Some(path) = relative.to_str() else {
        mark(result, "non-utf8-path");
        return;
    };
    let needs = entry.file_type().is_file() && compiled.iter().any(|(_, _, m)| m.needs_bytes(path));
    let mut bytes = None;
    if needs {
        match reader.read(relative, max_bytes) {
            Ok(data) => bytes = Some(data),
            Err(_) => mark(result, "file-read-or-size-limit"),
        }
    }
    for (registry, rule, m) in compiled {
        if is_expired(result) {
            break;
        }
        match m.matches(path, entry.file_type().is_dir(), bytes.as_deref()) {
            Ok(ids) => {
                for observed in ids {
                    result
                        .technologies
                        .entry(rule.technology.clone())
                        .or_default()
                        .insert(Evidence {
                            path: relative.to_path_buf(),
                            registry: (*registry).to_owned(),
                            rule: rule.id.clone(),
                            identifier: observed.identifier,
                            match_kind: Some(observed.kind),
                            version: observed.version,
                        });
                }
            }
            Err(code) => mark(result, code),
        }
    }
}

fn compile_rules<'a>(
    index: &'a CatalogIndex,
    mcp: bool,
    result: &mut ScanResult,
) -> Vec<(&'a str, &'a Rule, matcher::Matcher)> {
    let rules: Vec<(&str, &Rule)> = index
        .intelligence
        .iter()
        .flat_map(|(_, i)| {
            i.rules
                .iter()
                .filter(|r| !r.disabled)
                .map(|r| (i.registry_id.as_str(), r))
        })
        .collect();
    let mut compiled = Vec::new();
    for (registry, rule) in rules {
        if !index.technologies.contains(&rule.technology) {
            mark(result, "unresolved-technology");
            continue;
        }
        match matcher::Matcher::new(&rule.matcher, mcp) {
            Ok(m) => compiled.push((registry, rule, m)),
            Err(code) => mark(result, code),
        }
    }
    compiled
}
