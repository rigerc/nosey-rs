// SPDX-License-Identifier: MIT AND Apache-2.0
//! Central active-copy library, inspired by Kitter src/library.rs.
//! Local fixtures and verified catalog artifacts.
//!
//! Upstream: <https://github.com/what1f/kitter/tree/19711db044aabfa9846a65aaee693276d65af5c2>

mod acquisition;
mod cleanup;
pub(crate) mod manifest;
pub use acquisition::AcquisitionPreview;
pub use nosey_registry::BundleSource;
mod update;
pub use cleanup::{CleanupReport, RemovePreview};
pub use update::UpdatePreview;

use manifest::manifest;
pub(crate) use manifest::skill_name;

use crate::library_io::{directory, invalid, read, relative};
use crate::{Error, State};
use rustix::fs::{Mode, OFlags, openat};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::Write,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    sync::{Mutex, MutexGuard},
};
use walkdir::WalkDir;

const MAX_FILE: usize = 8 * 1024 * 1024;
const MAX_TREE: usize = 64 * 1024 * 1024;
const MAX_ENTRIES: usize = 4096;
pub(crate) const MAX_RECORD: usize = 1024 * 1024;
const FIXTURE_PREFIX: &str = "urn:nosey:fixture:";
/// Owner-only directory mode required for library storage.
const PRIVATE_MODE: u32 = 0o700;

/// Local integrity metadata shared by fixtures and catalog artifacts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalSource {
    /// Directory selected for a fixture, empty for remote content.
    pub path: PathBuf,
    /// Local content fingerprint; not a publisher signature or SDK digest.
    pub tree_sha256: String,
}

/// Origin of a library skill.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Acquisition {
    /// Explicitly selected local content.
    #[default]
    Fixture,
    /// An exact Git revision of a skill subtree.
    Git {
        /// Approved repository URL.
        repo: String,
        /// Skill subtree relative to the repository root.
        path: PathBuf,
        /// Exact commit object ID.
        revision: String,
    },
    /// A digest-verified catalog representation.
    Bundle {
        /// Representation URL.
        url: String,
        /// SDK representation media type.
        media_type: String,
        /// Declared digest of the fetched bytes.
        digest: String,
    },
}

/// Last known source availability.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Availability {
    /// Source was available at acquisition.
    #[default]
    Available,
    /// Source has become unavailable.
    Unavailable,
}

/// Metadata for one active downloaded/local copy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkillRecord {
    /// Canonical artifact identifier.
    pub id: String,
    /// Validated Agent Skill manifest name.
    pub name: String,
    /// Local path and integrity metadata.
    pub source: LocalSource,
    /// Selected registry identity for catalog acquisitions.
    #[serde(default)]
    pub registry: Option<String>,
    /// How the active copy was acquired.
    #[serde(default)]
    pub acquisition: Acquisition,
    /// Catalog release version when supplied.
    #[serde(default)]
    pub revision: Option<String>,
    /// Last known availability of the source.
    #[serde(default)]
    pub availability: Availability,
}

/// Private central library. All cooperating operations hold its exclusive lock.
pub struct Library {
    pub(crate) root: PathBuf,
    _lock: File,
    mutation: Mutex<()>,
}

pub(crate) fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub(crate) fn validate_fixture_id(id: &str) -> Result<(), Error> {
    let Some(suffix) = id.strip_prefix(FIXTURE_PREFIX) else {
        return Err(invalid("only fixture identifiers are accepted"));
    };
    if suffix.is_empty()
        || suffix.len() > 128
        || !suffix
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
    {
        return Err(invalid("invalid fixture identifier"));
    }
    Ok(())
}

fn validate_artifact_id(id: &str) -> Result<(), Error> {
    const MAX_ARTIFACT_ID: usize = 4096;
    if id.starts_with(FIXTURE_PREFIX) {
        return validate_fixture_id(id);
    }
    if id.is_empty()
        || id.len() > MAX_ARTIFACT_ID
        || id.chars().any(char::is_whitespace)
        || id.chars().any(char::is_control)
    {
        return Err(invalid("invalid catalog identifier"));
    }
    Ok(())
}

pub(crate) fn atomic_json<T: Serialize>(
    dir: &Path,
    path: &Path,
    value: &T,
    replace: bool,
) -> Result<(), Error> {
    let bytes = serde_json::to_vec_pretty(value)?;
    if bytes.len() > MAX_RECORD {
        return Err(invalid("record size limit"));
    }
    let mut temp = tempfile::NamedTempFile::new_in(dir)?;
    temp.write_all(&bytes)?;
    temp.as_file().sync_all()?;
    if replace {
        temp.persist(path).map_err(|e| e.error)?;
    } else {
        temp.persist_noclobber(path).map_err(|e| e.error)?;
    }
    File::open(dir)?.sync_all()?;
    Ok(())
}

fn private_directory(path: &Path) -> Result<(), Error> {
    let meta = fs::symlink_metadata(path)?;
    if !meta.is_dir() || meta.permissions().mode() & !PRIVATE_MODE & 0o777 != 0 {
        return Err(invalid("library directory must be real and private (0700)"));
    }
    Ok(())
}

fn child_dir(root: &Path, name: &str) -> Result<(), Error> {
    use std::os::unix::fs::DirBuilderExt;
    let path = root.join(name);
    match fs::DirBuilder::new().mode(PRIVATE_MODE).create(&path) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(e) => return Err(e.into()),
    }
    private_directory(&path)
}

impl Library {
    /// Open an existing private central directory outside all registered projects.
    ///
    /// # Errors
    ///
    /// Rejects linked/nonprivate paths, overlapping projects and concurrent
    /// access by another cooperating Nosey library operation.
    pub fn open(state: &State, path: &Path) -> Result<Self, Error> {
        Self::open_existing(state, path, true)
    }

    /// Open an initialized library without creating storage or a lock file.
    ///
    /// # Errors
    /// Rejects missing, unsafe, uninitialized or locked library storage.
    pub fn open_initialized(state: &State, path: &Path) -> Result<Self, Error> {
        Self::open_existing(state, path, false)
    }

    fn open_existing(state: &State, path: &Path, initialize: bool) -> Result<Self, Error> {
        let root = path.canonicalize()?;
        if root == Path::new("/")
            || state
                .projects
                .keys()
                .any(|p| root.starts_with(p) || p.starts_with(&root))
        {
            return Err(invalid("library and project roots must not overlap"));
        }
        if fs::symlink_metadata(path)?.file_type().is_symlink() {
            return Err(invalid("linked library root"));
        }
        private_directory(&root)?;
        let fd = directory(&root)?;
        let mut flags = OFlags::RDWR | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC;
        if initialize {
            flags |= OFlags::CREATE;
        }
        let lock = File::from(
            openat(&fd, "library.lock", flags, Mode::RUSR | Mode::WUSR)
                .map_err(std::io::Error::from)?,
        );
        if !lock.metadata()?.is_file() {
            return Err(invalid("invalid library lock"));
        }
        lock.try_lock()
            .map_err(|_| invalid("library busy or lock unavailable"))?;
        for child in ["objects", "links"] {
            if initialize {
                child_dir(&root, child)?;
            } else {
                private_directory(&root.join(child))?;
            }
        }
        Ok(Self {
            root,
            _lock: lock,
            mutation: Mutex::new(()),
        })
    }

    pub(crate) fn lock_mutation(&self) -> Result<MutexGuard<'_, ()>, Error> {
        self.mutation
            .lock()
            .map_err(|_| invalid("library mutation lock poisoned"))
    }

    pub(crate) fn object_dir(&self, id: &str) -> PathBuf {
        self.root.join("objects").join(hash(id.as_bytes()))
    }

    /// Read a stored skill and verify its local tree fingerprint and manifest.
    ///
    /// # Errors
    ///
    /// Rejects missing, malformed, modified or mismatched library content.
    pub fn get(&self, id: &str) -> Result<SkillRecord, Error> {
        validate_artifact_id(id)?;
        let dir = self.object_dir(id);
        let record = verified_record(&dir)?;
        if record.id != id {
            return Err(invalid("library identity mismatch"));
        }
        Ok(record)
    }

    /// Stage and publish an explicitly selected local fixture skill.
    /// Reuse an existing intact copy only if the source and digest match.
    /// Never replaces a library copy or executes content.
    ///
    /// # Errors
    ///
    /// Rejects invalid manifest, unsafe tree, bounds, overlapping paths and
    /// attempts to change an existing artifact under the same identity.
    pub fn import_fixture(&self, id: &str, source: &Path) -> Result<SkillRecord, Error> {
        let _mutation = self.lock_mutation()?;
        let (stage, record) = self.stage_fixture(id, source)?;
        let destination = self.object_dir(id);
        match fs::symlink_metadata(&destination) {
            Ok(_) => {
                let current = self.get(id)?;
                if current != record {
                    return Err(invalid("existing artifact requires explicit update"));
                }
                return Ok(current);
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
        }
        self.publish_stage(&stage, &destination, rustix::fs::RenameFlags::NOREPLACE)?;
        Ok(record)
    }

    fn fixture_source(&self, source: &Path) -> Result<PathBuf, Error> {
        if fs::symlink_metadata(source)?.file_type().is_symlink() {
            return Err(invalid("linked source root"));
        }
        let source = source.canonicalize()?;
        if source.starts_with(&self.root) || self.root.starts_with(&source) {
            return Err(invalid("source overlaps library"));
        }
        Ok(source)
    }

    fn stage_fixture(
        &self,
        id: &str,
        source: &Path,
    ) -> Result<(tempfile::TempDir, SkillRecord), Error> {
        validate_fixture_id(id)?;
        let source = self.fixture_source(source)?;
        let objects = self.root.join("objects");
        // Published objects stay as private as the library root they live in.
        let stage = tempfile::Builder::new()
            .permissions(fs::Permissions::from_mode(PRIVATE_MODE))
            .tempdir_in(&objects)?;
        let skill = stage.path().join("skill");
        fs::create_dir(&skill)?;
        let (digest, name) = tree(&source, Some(&skill))?;
        let record = SkillRecord {
            id: id.into(),
            name,
            source: LocalSource {
                path: source,
                tree_sha256: digest,
            },
            registry: None,
            acquisition: Acquisition::Fixture,
            revision: None,
            availability: Availability::Available,
        };
        atomic_json(
            stage.path(),
            &stage.path().join("record.json"),
            &record,
            false,
        )?;
        if tree(&skill, None)? != (record.source.tree_sha256.clone(), record.name.clone()) {
            return Err(invalid("staged content mismatch"));
        }
        Ok((stage, record))
    }

    fn publish_stage(
        &self,
        stage: &tempfile::TempDir,
        destination: &Path,
        flags: rustix::fs::RenameFlags,
    ) -> Result<(), Error> {
        let objects = self.root.join("objects");
        let parent = directory(&objects)?;
        rustix::fs::renameat_with(
            &parent,
            stage
                .path()
                .file_name()
                .ok_or_else(|| invalid("stage name"))?,
            &parent,
            destination
                .file_name()
                .ok_or_else(|| invalid("object name"))?,
            flags,
        )
        .map_err(|error| {
            if flags == rustix::fs::RenameFlags::EXCHANGE
                && matches!(error, rustix::io::Errno::INVAL | rustix::io::Errno::NOSYS)
            {
                invalid("library update requires filesystem and kernel support for RENAME_EXCHANGE")
            } else {
                std::io::Error::from(error).into()
            }
        })?;
        rustix::fs::fsync(&parent).map_err(std::io::Error::from)?;
        Ok(())
    }
}

fn tree(source: &Path, copy: Option<&Path>) -> Result<(String, String), Error> {
    let root = directory(source)?;
    let mut digest = Sha256::new();
    let mut total = 0_usize;
    let mut name = None;
    for (i, entry) in WalkDir::new(source)
        .follow_links(false)
        .max_depth(33)
        .sort_by_file_name()
        .into_iter()
        .enumerate()
    {
        if i > MAX_ENTRIES {
            return Err(invalid("skill tree entry limit"));
        }
        let entry = entry.map_err(|_| invalid("skill tree walk failed"))?;
        if entry.depth() == 0 {
            continue;
        }
        if entry.depth() > 32 {
            return Err(invalid("skill tree depth limit"));
        }
        let path = relative(
            entry
                .path()
                .strip_prefix(source)
                .map_err(|_| invalid("skill tree escape"))?,
        )?;
        let label = path.to_str().ok_or_else(|| invalid("invalid skill path"))?;
        let kind = entry.file_type();
        digest.update((label.len() as u64).to_le_bytes());
        digest.update(label.as_bytes());
        if kind.is_dir() {
            // Ensure the listed directory is still a real rooted directory.
            crate::library_io::descend(&root, &path)?;
            digest.update([0]);
            if let Some(dest) = copy {
                fs::create_dir(dest.join(&path))?;
            }
        } else if kind.is_file() {
            let bytes = read(&root, &path, MAX_FILE.min(MAX_TREE.saturating_sub(total)))?;
            total = total.saturating_add(bytes.len());
            if total > MAX_TREE {
                return Err(invalid("skill tree byte limit"));
            }
            digest.update([1]);
            digest.update((bytes.len() as u64).to_le_bytes());
            digest.update(&bytes);
            if path == Path::new("SKILL.md") {
                name = Some(manifest(&bytes)?);
            }
            if let Some(dest) = copy {
                let mut file = File::create_new(dest.join(&path))?;
                file.write_all(&bytes)?;
                file.sync_all()?;
            }
        } else {
            return Err(invalid("skill tree contains link or special file"));
        }
    }
    Ok((
        format!("{:x}", digest.finalize()),
        name.ok_or_else(|| invalid("missing root SKILL.md"))?,
    ))
}

fn verified_record(dir: &Path) -> Result<SkillRecord, Error> {
    let fd = directory(dir)?;
    let record: SkillRecord =
        serde_json::from_slice(&read(&fd, Path::new("record.json"), MAX_RECORD)?)?;
    let (digest, name) = tree(&dir.join("skill"), None)?;
    if digest != record.source.tree_sha256 || name != record.name {
        return Err(invalid("library content changed"));
    }
    Ok(record)
}

#[cfg(test)]
mod tests;
