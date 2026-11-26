// SPDX-License-Identifier: MIT AND Apache-2.0
//! Central active-copy library, inspired by Kitter src/library.rs.
//! Explicitly local fixtures only; no catalog trust is inferred from a path.
//! Upstream: https://github.com/what1f/kitter/tree/19711db044aabfa9846a65aaee693276d65af5c2

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
};
use walkdir::WalkDir;

const MAX_FILE: usize = 8 * 1024 * 1024;
const MAX_TREE: usize = 64 * 1024 * 1024;
const MAX_ENTRIES: usize = 4096;
pub(crate) const MAX_RECORD: usize = 1024 * 1024;
const FIXTURE_PREFIX: &str = "urn:nosey:fixture:";
/// Owner-only directory mode required for library storage.
const PRIVATE_MODE: u32 = 0o700;

/// Explicitly local, unverified fixture source. Never a trusted catalog entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalSource {
    /// Directory selected by the caller.
    pub path: PathBuf,
    /// Local content fingerprint; not a publisher signature or SDK digest.
    pub tree_sha256: String,
}

/// Metadata for one active downloaded/local copy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkillRecord {
    /// Fixture-only canonical identifier.
    pub id: String,
    /// Validated Agent Skill manifest name.
    pub name: String,
    /// Local source provenance.
    pub source: LocalSource,
}

/// Private central library. All cooperating operations hold its exclusive lock.
pub struct Library {
    pub(crate) root: PathBuf,
    _lock: File,
}

pub(crate) fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub(crate) fn validate_id(id: &str) -> Result<(), Error> {
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
        let lock = File::from(
            openat(
                &fd,
                "library.lock",
                OFlags::RDWR
                    | OFlags::CREATE
                    | OFlags::NOFOLLOW
                    | OFlags::NONBLOCK
                    | OFlags::CLOEXEC,
                Mode::RUSR | Mode::WUSR,
            )
            .map_err(std::io::Error::from)?,
        );
        if !lock.metadata()?.is_file() {
            return Err(invalid("invalid library lock"));
        }
        lock.try_lock()
            .map_err(|_| invalid("library busy or lock unavailable"))?;
        child_dir(&root, "objects")?;
        child_dir(&root, "links")?;
        Ok(Self { root, _lock: lock })
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
        validate_id(id)?;
        let dir = self.object_dir(id);
        let fd = directory(&dir)?;
        let record: SkillRecord =
            serde_json::from_slice(&read(&fd, Path::new("record.json"), MAX_RECORD)?)?;
        if record.id != id {
            return Err(invalid("library identity mismatch"));
        }
        let (digest, name) = tree(&dir.join("skill"), None)?;
        if digest != record.source.tree_sha256 || name != record.name {
            return Err(invalid("library content changed"));
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
        validate_id(id)?;
        if fs::symlink_metadata(source)?.file_type().is_symlink() {
            return Err(invalid("linked source root"));
        }
        let source = source.canonicalize()?;
        if source.starts_with(&self.root) || self.root.starts_with(&source) {
            return Err(invalid("source overlaps library"));
        }
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
        };
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
        atomic_json(
            stage.path(),
            &stage.path().join("record.json"),
            &record,
            false,
        )?;
        if tree(&skill, None)? != (record.source.tree_sha256.clone(), record.name.clone()) {
            return Err(invalid("staged content mismatch"));
        }
        // Atomic no-replace publication; drop the empty temp path after rename.
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
            rustix::fs::RenameFlags::NOREPLACE,
        )
        .map_err(std::io::Error::from)?;
        File::open(&objects)?.sync_all()?;
        Ok(record)
    }
}

fn manifest(bytes: &[u8]) -> Result<String, Error> {
    let text = std::str::from_utf8(bytes).map_err(|_| invalid("manifest is not UTF-8"))?;
    let mut lines = text.lines();
    if lines.next() != Some("---") {
        return Err(invalid("missing skill frontmatter"));
    }
    let mut header = String::new();
    let mut closed = false;
    for line in lines {
        if line == "---" {
            closed = true;
            break;
        }
        header.push_str(line);
        header.push('\n');
        if header.len() > 16 * 1024 {
            return Err(invalid("frontmatter limit"));
        }
    }
    if !closed || header.contains(['&', '*', '!', '%']) {
        return Err(invalid("unsupported skill frontmatter"));
    }
    let doc: serde_yaml::Value =
        serde_yaml::from_str(&header).map_err(|_| invalid("invalid skill frontmatter"))?;
    let name = doc
        .get("name")
        .and_then(serde_yaml::Value::as_str)
        .ok_or_else(|| invalid("skill name missing"))?;
    if name.is_empty()
        || name.len() > 64
        || name.starts_with('-')
        || name.ends_with('-')
        || name.contains("--")
        || !name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
    {
        return Err(invalid("invalid skill name"));
    }
    let description = doc
        .get("description")
        .and_then(serde_yaml::Value::as_str)
        .ok_or_else(|| invalid("skill description missing"))?;
    if description.trim().is_empty()
        || description.chars().count() > 1024
        || description.chars().any(|c| c.is_control() && c != '\n')
    {
        return Err(invalid("invalid skill description"));
    }
    Ok(name.into())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn imports_once_and_rejects_mutation_or_symlink() -> Result<(), Box<dyn std::error::Error>> {
        let central = tempfile::Builder::new()
            .permissions(fs::Permissions::from_mode(0o700))
            .tempdir()?;
        let source = tempfile::tempdir()?;
        fs::write(
            source.path().join("SKILL.md"),
            "---\nname: demo\ndescription: Fixture\n---\nDo not execute.\n",
        )?;
        let library = Library::open(&State::default(), central.path())?;
        let first = library.import_fixture("urn:nosey:fixture:demo", source.path())?;
        assert_eq!(library.import_fixture(&first.id, source.path())?, first);
        assert_eq!(
            fs::metadata(library.object_dir(&first.id))?
                .permissions()
                .mode()
                & 0o777,
            PRIVATE_MODE
        );
        fs::write(source.path().join("script.sh"), "new")?;
        assert!(library.import_fixture(&first.id, source.path()).is_err());
        std::os::unix::fs::symlink("script.sh", source.path().join("link"))?;
        assert!(
            library
                .import_fixture("urn:nosey:fixture:other", source.path())
                .is_err()
        );
        assert_eq!(library.get(&first.id)?, first);
        assert!(Library::open(&State::default(), central.path()).is_err());
        Ok(())
    }
}
