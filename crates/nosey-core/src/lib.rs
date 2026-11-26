//! Central project state for the headless scan/recommendation development slice.
#![forbid(unsafe_code)]
use nosey_registry::CatalogIndex;
use nosey_scanner::{Completeness, ScanResult, Settings, scan};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
};
use thiserror::Error;

/// Explicit installation preview, journal and approval services.
#[cfg(target_os = "linux")]
pub mod changes;
mod freshness;
/// Read-only effective skill inventory.
#[cfg(target_os = "linux")]
pub mod inventory;
/// Central local fixture library.
#[cfg(target_os = "linux")]
pub mod library;
#[cfg(target_os = "linux")]
mod library_io;
/// Shared batch change planning with per-item outcomes.
#[cfg(target_os = "linux")]
pub mod plan;
mod project_activity;
/// Native read-only project target discovery.
pub mod skill_discovery;
/// Agent target configuration and install scopes.
pub mod targets;
pub use freshness::{Freshness, SavedEvaluation};

/// Central state failures.
#[derive(Debug, Error)]
pub enum Error {
    /// Filesystem failure.
    #[error("state I/O: {0}")]
    Io(#[from] std::io::Error),
    /// Invalid state data.
    #[error("state data: {0}")]
    Json(#[from] serde_json::Error),
    /// Unsafe or invalid input.
    #[error("{0}")]
    Invalid(String),
}
/// Project record keyed by its canonical absolute directory, pending production domain ID integration.
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Project {
    /// Saved latest usable scan.
    pub scan: Option<ScanResult>,
    /// Latest attempted scan coverage.
    pub last_attempt: Option<Completeness>,
    /// Persistent dismissals keyed by canonical skill identity.
    pub dismissed: BTreeSet<String>,
}
/// Prototype Nosey-owned central state; production persistence is outstanding.
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct State {
    /// Registered project directories.
    pub projects: BTreeMap<PathBuf, Project>,
    /// Last explicit project activity, using Kitter's millisecond ordering.
    #[serde(default)]
    pub project_activity: BTreeMap<PathBuf, u64>,
    #[serde(default)]
    targets: targets::TargetConfig,
}

impl State {
    /// Load the explicitly selected central state file.
    /// # Errors
    /// Rejects symlinks, oversized data and invalid JSON.
    pub fn load(path: &Path) -> Result<Self, Error> {
        let metadata = match fs::symlink_metadata(path) {
            Ok(m) => m,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(e) => return Err(e.into()),
        };
        if !metadata.is_file() || metadata.len() > 32 * 1024 * 1024 {
            return Err(Error::Invalid("invalid state file".into()));
        }
        let mut data = Vec::new();
        fs::File::open(path)?
            .take(32 * 1024 * 1024 + 1)
            .read_to_end(&mut data)?;
        if data.len() > 32 * 1024 * 1024 {
            return Err(Error::Invalid("state size limit".into()));
        }
        let state: Self = serde_json::from_slice(&data)?;
        state.targets.validate()?;
        Ok(state)
    }
    /// Persist atomically to an existing central directory outside all registered projects.
    /// # Errors
    /// Refuses project-local state and invalid destinations.
    pub fn save(&self, path: &Path) -> Result<(), Error> {
        let parent = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        let parent = parent.canonicalize()?;
        if self.projects.keys().any(|p| parent.starts_with(p)) {
            return Err(Error::Invalid(
                "central state must be outside projects".into(),
            ));
        }
        let mut temp = tempfile::NamedTempFile::new_in(&parent)?;
        temp.write_all(&serde_json::to_vec_pretty(self)?)?;
        temp.as_file().sync_all()?;
        temp.persist(path).map_err(|e| e.error)?;
        Ok(())
    }
    /// Explicitly register an existing project directory.
    /// # Errors
    /// Refuses non-directories or inaccessible paths.
    pub fn register(&mut self, path: &Path) -> Result<PathBuf, Error> {
        let root = path.canonicalize()?;
        if !root.is_dir() {
            return Err(Error::Invalid("project is not a directory".into()));
        }
        if !self.projects.contains_key(&root) {
            self.projects.insert(root.clone(), Project::default());
            self.touch_project(&root);
        }
        Ok(root)
    }
    /// Run a manual scan and retain prior usable results after failed/cancelled attempts.
    /// # Errors
    /// Unregistered roots and invalid settings fail without state mutation.
    pub fn scan(
        &mut self,
        root: &Path,
        index: &CatalogIndex,
        settings: &Settings,
        cancel: &AtomicBool,
    ) -> Result<&ScanResult, Error> {
        let root = root.canonicalize()?;
        let project = self
            .projects
            .get_mut(&root)
            .ok_or_else(|| Error::Invalid("project not registered".into()))?;
        let result = scan(&root, index, settings, cancel).map_err(Error::Invalid)?;
        project.last_attempt = Some(result.completeness);
        if matches!(
            result.completeness,
            Completeness::Complete | Completeness::Incomplete
        ) {
            project.scan = Some(result);
        }
        project
            .scan
            .as_ref()
            .ok_or_else(|| Error::Invalid("no usable scan".into()))
    }
}
