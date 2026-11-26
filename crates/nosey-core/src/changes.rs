// SPDX-License-Identifier: MIT AND Apache-2.0
//! Install preview/revalidation adapted from Kitter src/project/install.rs.
//! Unlike Kitter, no occupied path is replaced, and no unmanaged link adopted.
//! Pending journal records survive an interrupted apply.

mod maintenance;
pub use maintenance::{LinkAction, LinkPreview};

use crate::{
    Error, State,
    library::{Library, MAX_RECORD, atomic_json, hash},
    library_io::{descend, directory, identity, invalid, read, relative, stat, stat_identity},
};
use rustix::fs::{FileType, readlinkat, symlinkat};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    os::fd::OwnedFd,
    path::{Path, PathBuf},
};

/// Independent reason a managed link is needed.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "kind", content = "id", rename_all = "snake_case")]
pub enum InstallReason {
    /// Explicit manual selection.
    Manual,
    /// Explicitly accepted recommendation.
    Recommendation,
    /// Applied preset identity (preset synchronization not yet connected).
    Preset(String),
}

/// Destination ownership status, never inferred from a name alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    /// Missing destination, no existing journal record.
    Available,
    /// Verified managed link with matching recorded identity.
    Managed,
    /// Existing path not owned by Nosey.
    Occupied,
    /// Pending, missing, changed or detached journal record needs review.
    RecoveryRequired,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct LinkRecord {
    project: PathBuf,
    destination: PathBuf,
    artifact: String,
    source: PathBuf,
    reasons: BTreeSet<InstallReason>,
    parent_identity: (u64, u64),
    // None is a pending record, never proof of ownership.
    link_identity: Option<(u64, u64)>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    tree_sha256: Option<String>,
}

/// Explicit read-only preview for a single project-local installation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InstallPreview {
    /// Registered canonical project.
    pub project: PathBuf,
    /// Explicit existing project-relative target directory.
    pub target: PathBuf,
    /// Fixture artifact identity.
    pub artifact: String,
    /// Exact destination path.
    pub destination: PathBuf,
    /// Active central skill directory.
    pub source: PathBuf,
    /// Local tree digest checked at preview.
    pub tree_sha256: String,
    /// Independent reason requested.
    pub reason: InstallReason,
    /// Current destination status.
    pub status: Status,
    /// Fingerprint of complete preview state, supplied for explicit apply.
    pub fingerprint: String,
}

/// Read-only ownership and retained-reference report.
#[derive(Debug, Clone, Serialize)]
pub struct LinkAudit {
    /// Artifact identifier.
    pub artifact: String,
    /// Original registered project path, even after unregister.
    pub project: PathBuf,
    /// Destination path.
    pub destination: PathBuf,
    /// Independent retained reasons.
    pub reasons: BTreeSet<InstallReason>,
    /// True if project is no longer registered.
    pub detached: bool,
    /// Current observed ownership status.
    pub status: Status,
}

/// Longest accepted preset identity in an install reason.
const MAX_PRESET_ID: usize = 128;

fn name(path: &Path) -> Result<&Path, Error> {
    path.file_name()
        .map(Path::new)
        .ok_or_else(|| invalid("missing link name"))
}

fn exact_link(parent: &OwnedFd, leaf: &Path, source: &Path) -> Result<Option<(u64, u64)>, Error> {
    let Some(s) = stat(parent, leaf)? else {
        return Ok(None);
    };
    if FileType::from_raw_mode(s.st_mode) != FileType::Symlink {
        return Ok(None);
    }
    let target = readlinkat(parent, leaf, Vec::new()).map_err(std::io::Error::from)?;
    if target.to_bytes() != source.as_os_str().as_encoded_bytes() {
        return Ok(None);
    }
    let Some(after) = stat(parent, leaf)? else {
        return Ok(None);
    };
    if after.st_dev != s.st_dev
        || after.st_ino != s.st_ino
        || FileType::from_raw_mode(after.st_mode) != FileType::Symlink
    {
        return Ok(None);
    }
    Ok(Some(stat_identity(&s)))
}

fn target_dir(project: &Path, target: &Path) -> Result<OwnedFd, Error> {
    descend(&directory(project)?, target)
}

fn classify(
    parent: &OwnedFd,
    leaf: &Path,
    source: &Path,
    record: Option<&LinkRecord>,
) -> Result<Status, Error> {
    if let Some(record) = record {
        let observed = exact_link(parent, leaf, source)?;
        if record.source == source
            && record.parent_identity == identity(parent)?
            && record.link_identity.is_some()
            && record.link_identity == observed
        {
            return Ok(Status::Managed);
        }
        return Ok(Status::RecoveryRequired);
    }
    Ok(if stat(parent, leaf)?.is_some() {
        Status::Occupied
    } else {
        Status::Available
    })
}

impl Library {
    fn link_path(&self, destination: &Path) -> PathBuf {
        self.root.join("links").join(format!(
            "{}.json",
            hash(destination.as_os_str().as_encoded_bytes())
        ))
    }

    fn link_record(&self, destination: &Path) -> Result<Option<LinkRecord>, Error> {
        let path = self.link_path(destination);
        match std::fs::symlink_metadata(&path) {
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e.into()),
        }
        let root = directory(&self.root.join("links"))?;
        let record: LinkRecord = serde_json::from_slice(&read(&root, name(&path)?, MAX_RECORD)?)?;
        if record.destination != destination {
            return Err(invalid("journal destination mismatch"));
        }
        Ok(Some(record))
    }

    /// Preview an install into an existing explicit project-relative directory.
    ///
    /// # Errors
    ///
    /// Rejects unregistered projects, unsafe paths, invalid source or journal.
    pub fn preview(
        &self,
        state: &State,
        project: &Path,
        target: &Path,
        artifact: &str,
        reason: InstallReason,
    ) -> Result<InstallPreview, Error> {
        let project = project.canonicalize()?;
        if !state.projects.contains_key(&project)
            || project.starts_with(&self.root)
            || self.root.starts_with(&project)
        {
            return Err(invalid("unregistered or overlapping project"));
        }
        if let InstallReason::Preset(id) = &reason
            && (id.is_empty() || id.len() > MAX_PRESET_ID || id.chars().any(char::is_control))
        {
            return Err(invalid("invalid preset reason"));
        }
        let target = relative(target)?;
        let skill = self.get(artifact)?;
        let parent = target_dir(&project, &target)?;
        let destination = project.join(&target).join(&skill.name);
        let source = self.object_dir(artifact).join("skill");
        let record = self.link_record(&destination)?;
        let status = classify(&parent, Path::new(&skill.name), &source, record.as_ref())?;
        let status = if record
            .as_ref()
            .is_some_and(|r| r.artifact != artifact || r.project != project)
        {
            Status::RecoveryRequired
        } else {
            status
        };
        let mut preview = InstallPreview {
            project,
            target,
            artifact: artifact.into(),
            destination,
            source,
            tree_sha256: skill.source.tree_sha256,
            reason,
            status,
            fingerprint: String::new(),
        };
        let leaf = name(&preview.destination)?;
        let observed = stat(&parent, leaf)?.map(|s| (stat_identity(&s), s.st_mode));
        preview.fingerprint = hash(&serde_json::to_vec(&(
            &preview,
            identity(&parent)?,
            observed,
            &record,
        ))?);
        Ok(preview)
    }

    /// Apply only a fresh, explicitly approved preview. Existing matching managed
    /// links gain the independent reason without replacing the link.
    ///
    /// # Errors
    ///
    /// Rejects stale, occupied, recovery-needed or unsafe changes. A crash or
    /// post-creation error leaves a pending journal rather than claiming ownership.
    pub fn apply(&self, state: &State, approved: &InstallPreview) -> Result<Status, Error> {
        let fresh = self.preview(
            state,
            &approved.project,
            &approved.target,
            &approved.artifact,
            approved.reason.clone(),
        )?;
        if &fresh != approved {
            return Err(invalid("stale preview; review again"));
        }
        if !matches!(fresh.status, Status::Available | Status::Managed) {
            return Err(invalid("destination blocked"));
        }
        let parent = target_dir(&fresh.project, &fresh.target)?;
        let leaf = name(&fresh.destination)?;
        let journal = self.link_path(&fresh.destination);
        let journal_dir = self.root.join("links");
        let current = self.link_record(&fresh.destination)?;
        if classify(&parent, leaf, &fresh.source, current.as_ref())? != fresh.status {
            return Err(invalid("destination changed"));
        }
        // Bind the second opened parent to the actual previewed parent and
        // recheck the complete fingerprint before making any mutation.
        let observed = stat(&parent, leaf)?.map(|s| (stat_identity(&s), s.st_mode));
        let mut check = fresh.clone();
        check.fingerprint.clear();
        if hash(&serde_json::to_vec(&(
            &check,
            identity(&parent)?,
            observed,
            &current,
        ))?) != fresh.fingerprint
        {
            return Err(invalid("destination changed after preview"));
        }
        if let Some(mut current) = current {
            if current.reasons.insert(fresh.reason) {
                atomic_json(&journal_dir, &journal, &current, true)?;
            }
            return Ok(Status::Managed);
        }
        let mut record = LinkRecord {
            project: fresh.project,
            destination: fresh.destination,
            artifact: fresh.artifact,
            source: fresh.source,
            reasons: BTreeSet::from([fresh.reason]),
            parent_identity: identity(&parent)?,
            link_identity: None,
            tree_sha256: Some(fresh.tree_sha256),
        };
        atomic_json(&journal_dir, &journal, &record, false)?;
        // Atomic non-replacing symlink creation; never delete a competing path.
        // If creation fails, leave the pending record for explicit recovery.
        symlinkat(&record.source, &parent, name(&record.destination)?)
            .map_err(std::io::Error::from)?;
        record.link_identity = exact_link(&parent, name(&record.destination)?, &record.source)?;
        if record.link_identity.is_none() {
            return Err(invalid("link changed during install; recovery required"));
        }
        let reopened = target_dir(&record.project, &approved.target)?;
        if identity(&reopened)? != record.parent_identity
            || exact_link(&reopened, name(&record.destination)?, &record.source)?
                != record.link_identity
        {
            return Err(invalid("target moved during install; recovery required"));
        }
        rustix::fs::fsync(&parent).map_err(std::io::Error::from)?;
        atomic_json(&journal_dir, &journal, &record, true)?;
        Ok(Status::Managed)
    }

    /// List recorded links including detached and unresolved records.
    /// No directory is opened through a symbolic link and no path is removed.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid journal records or excessive record count.
    pub fn audit_links(&self, state: &State) -> Result<Vec<LinkAudit>, Error> {
        let dir = self.root.join("links");
        let fd = directory(&dir)?;
        let mut result = Vec::new();
        for entry in std::fs::read_dir(&dir)? {
            let entry = entry?;
            if result.len() >= 4096 {
                return Err(invalid("journal entry limit"));
            }
            let path = entry.path();
            if path.extension().is_none_or(|e| e != "json") {
                continue;
            }
            let r: LinkRecord = serde_json::from_slice(&read(&fd, name(&path)?, MAX_RECORD)?)?;
            if self.link_path(&r.destination) != path || !r.destination.starts_with(&r.project) {
                return Err(invalid("journal identity mismatch"));
            }
            let parent = r
                .destination
                .parent()
                .ok_or_else(|| invalid("journal parent"))?;
            let status = if self.get(&r.artifact).is_ok()
                && r.source == self.object_dir(&r.artifact).join("skill")
            {
                directory(parent)
                    .and_then(|p| classify(&p, name(&r.destination)?, &r.source, Some(&r)))
                    .unwrap_or(Status::RecoveryRequired)
            } else {
                Status::RecoveryRequired
            };
            result.push(LinkAudit {
                artifact: r.artifact,
                detached: !state.projects.contains_key(&r.project),
                project: r.project,
                destination: r.destination,
                reasons: r.reasons,
                status,
            });
        }
        result.sort_by(|a, b| a.destination.cmp(&b.destination));
        Ok(result)
    }
}
