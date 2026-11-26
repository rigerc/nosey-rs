// SPDX-License-Identifier: MIT AND Apache-2.0
//! Install preview/revalidation adapted from Kitter src/project/install.rs.
//!
//! Unlike Kitter, no occupied path is replaced, and no unmanaged link adopted.
//! Pending journal records survive an interrupted apply.

use crate::presets::MAX_PRESET_ID;

mod audit;
mod maintenance;
pub use audit::{JournalConnection, LinkAudit};
pub use maintenance::{LinkAction, LinkPreview};

use crate::{
    Error, State,
    library::{Library, MAX_RECORD, atomic_json, hash},
    library_io::{descend, directory, identity, invalid, read, relative, stat, stat_identity},
    targets::InstallScope,
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
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    global: bool,
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

/// Explicit read-only preview for a single installation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InstallPreview {
    /// Registered project or supplied home directory.
    pub project: PathBuf,
    /// Whether this preview installs into a global agent location.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub global: bool,
    /// Explicit existing project-relative target directory.
    pub target: PathBuf,
    /// Canonical artifact identity.
    pub artifact: String,
    /// Exact destination path.
    pub destination: PathBuf,
    /// Active central skill directory.
    pub source: PathBuf,
    /// Local tree digest when the artifact is already available.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub tree_sha256: String,
    /// Independent reason requested.
    pub reason: InstallReason,
    /// Current destination status.
    pub status: Status,
    /// Fingerprint of complete preview state, supplied for explicit apply.
    pub fingerprint: String,
}

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
        let skill = self.get(artifact)?;
        self.preview_skill(
            state,
            project,
            target,
            artifact,
            reason,
            (&skill.name, &skill.source.tree_sha256),
        )
    }

    pub(crate) fn preview_skill(
        &self,
        state: &State,
        project: &Path,
        target: &Path,
        artifact: &str,
        reason: InstallReason,
        skill: (&str, &str),
    ) -> Result<InstallPreview, Error> {
        let project = project.canonicalize()?;
        if !state.projects.contains_key(&project)
            || project.starts_with(&self.root)
            || self.root.starts_with(&project)
        {
            return Err(invalid("unregistered or overlapping project"));
        }
        self.preview_at(project, false, target, artifact, reason, skill)
    }

    pub(crate) fn preview_scoped(
        &self,
        state: &State,
        scope: &InstallScope,
        target: &Path,
        artifact: &str,
        reason: InstallReason,
    ) -> Result<InstallPreview, Error> {
        let skill = self.get(artifact)?;
        self.preview_scoped_skill(
            state,
            scope,
            target,
            artifact,
            reason,
            (&skill.name, &skill.source.tree_sha256),
        )
    }

    /// Scoped preview for an artifact that may still be pending acquisition.
    pub(crate) fn preview_scoped_skill(
        &self,
        state: &State,
        scope: &InstallScope,
        target: &Path,
        artifact: &str,
        reason: InstallReason,
        skill: (&str, &str),
    ) -> Result<InstallPreview, Error> {
        let global = matches!(scope, InstallScope::Global { .. });
        if !state.targets().paths(global).contains(target) {
            return Err(invalid("target is not enabled for this scope"));
        }
        match scope {
            InstallScope::Project { root } => {
                self.preview_skill(state, root, target, artifact, reason, skill)
            }
            InstallScope::Global { home } => {
                let destination = home.join(target);
                if destination.starts_with(&self.root) || self.root.starts_with(&destination) {
                    return Err(invalid("global target overlaps the library"));
                }
                self.preview_at(home.clone(), true, target, artifact, reason, skill)
            }
        }
    }

    fn preview_at(
        &self,
        project: PathBuf,
        global: bool,
        target: &Path,
        artifact: &str,
        reason: InstallReason,
        skill: (&str, &str),
    ) -> Result<InstallPreview, Error> {
        if let InstallReason::Preset(id) = &reason
            && (id.is_empty() || id.len() > MAX_PRESET_ID || id.chars().any(char::is_control))
        {
            return Err(invalid("invalid preset reason"));
        }
        let target = relative(target)?;
        let (skill_name, digest) = skill;
        crate::library::skill_name(skill_name)?;
        let parent = target_dir(&project, &target)?;
        let destination = project.join(&target).join(skill_name);
        let source = self.object_dir(artifact).join("skill");
        let record = self.link_record(&destination)?;
        let status = classify(&parent, Path::new(skill_name), &source, record.as_ref())?;
        let status = if record
            .as_ref()
            .is_some_and(|r| r.artifact != artifact || r.project != project || r.global != global)
        {
            Status::RecoveryRequired
        } else {
            status
        };
        let mut preview = InstallPreview {
            project,
            global,
            target,
            artifact: artifact.into(),
            destination,
            source,
            tree_sha256: digest.into(),
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
        self.apply_install(state, approved, None)
    }

    pub(crate) fn apply_install(
        &self,
        state: &State,
        approved: &InstallPreview,
        bundle: Option<&crate::library::BundleSource>,
    ) -> Result<Status, Error> {
        let _mutation = self.lock_mutation()?;
        self.apply_install_locked(state, approved, bundle)
    }

    /// Install while the caller already holds the library mutation lock.
    pub(crate) fn apply_install_locked(
        &self,
        state: &State,
        approved: &InstallPreview,
        bundle: Option<&crate::library::BundleSource>,
    ) -> Result<Status, Error> {
        let skill = match bundle {
            Some(source) => self
                .preview_acquire(source)?
                .cached
                .ok_or_else(|| invalid("required bundle acquisition did not succeed"))?,
            None => self.get(&approved.artifact)?,
        };
        // A pending install binds the representation digest in its acquisition
        // detail. Its local tree fingerprint is only known after extraction.
        let digest = bundle.map_or(skill.source.tree_sha256.as_str(), |_| "");
        let fresh = if approved.global {
            self.preview_scoped_skill(
                state,
                &InstallScope::Global {
                    home: approved.project.clone(),
                },
                &approved.target,
                &approved.artifact,
                approved.reason.clone(),
                (&skill.name, digest),
            )?
        } else {
            self.preview_skill(
                state,
                &approved.project,
                &approved.target,
                &approved.artifact,
                approved.reason.clone(),
                (&skill.name, digest),
            )?
        };
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
            global: fresh.global,
            destination: fresh.destination,
            artifact: fresh.artifact,
            source: fresh.source,
            reasons: BTreeSet::from([fresh.reason]),
            parent_identity: identity(&parent)?,
            link_identity: None,
            tree_sha256: Some(skill.source.tree_sha256),
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
}
