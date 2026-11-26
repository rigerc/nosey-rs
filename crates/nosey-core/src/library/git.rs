//! Pinned Git acquisition without a working-tree checkout.

mod process;
#[cfg(test)]
mod tests;

use self::process::Git;
use super::{
    Acquisition, Availability, FIXTURE_PREFIX, Library, LocalSource, MAX_ENTRIES, MAX_FILE,
    MAX_TREE, PRIVATE_MODE, SkillRecord, atomic_json, hash, tree, validate_artifact_id,
};
use crate::{
    Error,
    library_io::{invalid, relative},
};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Write as _,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};

const SHA_LEN: usize = 40;
const MAX_METADATA: usize = 4096;
const MAX_LISTING: usize = 4 * 1024 * 1024;

/// Explicitly approved source of a skill at a pinned Git commit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GitSource {
    /// Canonical library artifact identity.
    pub artifact: String,
    /// HTTPS repository URL without credentials.
    pub repo: String,
    /// Relative subtree path, or `.` for the repository root.
    pub path: PathBuf,
    /// Full 40-hex commit ID.
    pub revision: String,
}

#[derive(Clone, Copy)]
enum GitProtocol {
    Https,
    #[cfg(test)]
    File {
        staging_limit: u64,
    },
}

/// Git acquisition source and its observed active copy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GitAcquisitionPreview {
    /// Approved source.
    pub source: GitSource,
    /// Intact active copy matching the source, if present.
    pub cached: Option<SkillRecord>,
    /// Approval fingerprint covering the source and active copy.
    pub fingerprint: String,
}

impl GitSource {
    fn validate(&self, protocol: GitProtocol) -> Result<(), Error> {
        validate_artifact_id(&self.artifact)?;
        if self.artifact.starts_with(FIXTURE_PREFIX)
            || self.revision.len() != SHA_LEN
            || !self.revision.bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(invalid(
                "Git acquisition requires a non-fixture ID and a full commit SHA",
            ));
        }
        if self.path != Path::new(".") {
            relative(&self.path)?;
            if self.path.as_os_str().len() > MAX_METADATA {
                return Err(invalid("Git subtree path exceeds its limit"));
            }
        }
        if self.repo.len() > MAX_METADATA
            || self
                .repo
                .chars()
                .any(|c| c.is_whitespace() || c.is_control())
            || self.repo.contains(['?', '#', '\\'])
        {
            return Err(invalid("invalid Git repository URL"));
        }
        match protocol {
            GitProtocol::Https => {
                let url = url::Url::parse(&self.repo)
                    .map_err(|_| invalid("invalid Git repository URL"))?;
                if !self.repo.starts_with("https://") || url.authority().contains('@') {
                    return Err(invalid("Git repository must use HTTPS without credentials"));
                }
                nosey_registry::validate_url(&url).map_err(|e| invalid(&e))?;
            }
            #[cfg(test)]
            GitProtocol::File { .. } => {
                if !self.repo.starts_with("file:///") {
                    return Err(invalid("test Git repository requires an absolute file URL"));
                }
            }
        }
        Ok(())
    }

    fn provenance(&self) -> Acquisition {
        Acquisition::Git {
            repo: self.repo.clone(),
            path: self.path.clone(),
            revision: self.revision.to_ascii_lowercase(),
        }
    }
}

impl Library {
    /// Review a pinned Git source without fetching.
    ///
    /// # Errors
    /// Rejects invalid metadata and corrupt or conflicting active copies.
    pub fn preview_acquire_git(&self, source: &GitSource) -> Result<GitAcquisitionPreview, Error> {
        self.preview_pinned_git(source, GitProtocol::Https)
    }

    fn preview_pinned_git(
        &self,
        source: &GitSource,
        protocol: GitProtocol,
    ) -> Result<GitAcquisitionPreview, Error> {
        source.validate(protocol)?;
        let cached = match fs::symlink_metadata(self.object_dir(&source.artifact)) {
            Ok(_) => {
                let record = self.get(&source.artifact)?;
                if record.acquisition != source.provenance()
                    || record.revision.as_deref()
                        != Some(source.revision.to_ascii_lowercase().as_str())
                {
                    return Err(invalid("existing artifact requires explicit update"));
                }
                Some(record)
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(error.into()),
        };
        Ok(GitAcquisitionPreview {
            source: source.clone(),
            fingerprint: hash(&serde_json::to_vec(&(source, &cached))?),
            cached,
        })
    }

    /// Acquire the approved pinned Git subtree, reusing an intact matching copy.
    ///
    /// # Errors
    /// Rejects invalid sources, fetch failures and invalid or unpublishable skill trees.
    pub fn acquire_git(&self, source: &GitSource) -> Result<SkillRecord, Error> {
        self.acquire_named_git(source, None)
    }

    pub(crate) fn acquire_named_git(
        &self,
        source: &GitSource,
        name: Option<&str>,
    ) -> Result<SkillRecord, Error> {
        self.acquire_pinned_git(source, name, GitProtocol::Https)
    }

    fn acquire_pinned_git(
        &self,
        source: &GitSource,
        name: Option<&str>,
        protocol: GitProtocol,
    ) -> Result<SkillRecord, Error> {
        let _mutation = self.lock_mutation()?;
        if let Some(record) = self.preview_pinned_git(source, protocol)?.cached {
            super::acquisition::check_name(name, &record.name)?;
            return Ok(record);
        }
        let objects = self.root.join("objects");
        let stage = tempfile::Builder::new()
            .permissions(fs::Permissions::from_mode(PRIVATE_MODE))
            .tempdir_in(&objects)?;
        let skill = stage.path().join("skill");
        fs::create_dir(&skill)?;
        let git_stage = tempfile::Builder::new()
            .permissions(fs::Permissions::from_mode(PRIVATE_MODE))
            .tempdir_in(&objects)?;
        Git::new(git_stage.path(), protocol)?.extract(source, &skill)?;
        let (tree_sha256, actual_name) = tree(&skill, None)?;
        super::acquisition::check_name(name, &actual_name)?;
        let record = SkillRecord {
            id: source.artifact.clone(),
            name: actual_name,
            source: LocalSource {
                path: PathBuf::new(),
                tree_sha256,
            },
            registry: None,
            revision: Some(source.revision.to_ascii_lowercase()),
            acquisition: source.provenance(),
            availability: Availability::Available,
        };
        atomic_json(
            stage.path(),
            &stage.path().join("record.json"),
            &record,
            false,
        )?;
        self.publish_stage(
            &stage,
            &self.object_dir(&source.artifact),
            rustix::fs::RenameFlags::NOREPLACE,
        )?;
        Ok(record)
    }
}

impl Git<'_> {
    fn extract(&mut self, source: &GitSource, root: &Path) -> Result<(), Error> {
        self.run(
            &["init", "--bare", "--template=", "--object-format=sha1"],
            MAX_METADATA,
        )?;
        let repo = match self.protocol() {
            GitProtocol::Https => url::Url::parse(&source.repo)
                .map_err(|_| invalid("invalid Git repository URL"))?
                .to_string(),
            #[cfg(test)]
            GitProtocol::File { .. } => source.repo.clone(),
        };
        self.prepare_remote(&repo)?;
        let revision = source.revision.to_ascii_lowercase();
        self.run(
            &[
                "fetch",
                "--depth=1",
                "--no-tags",
                "--no-recurse-submodules",
                "--filter=blob:none",
                "origin",
                &revision,
            ],
            0,
        )?;
        let actual = self.run(
            &["rev-parse", "--verify", "FETCH_HEAD^{commit}"],
            SHA_LEN + 1,
        )?;
        if actual.strip_suffix(b"\n") != Some(revision.as_bytes()) {
            return Err(invalid(
                "fetched Git commit differs from the pinned revision",
            ));
        }
        let subtree = if source.path == Path::new(".") {
            "FETCH_HEAD^{tree}".into()
        } else {
            format!("FETCH_HEAD:{}", relative(&source.path)?.display())
        };
        let listing = self.run(&["ls-tree", "-r", "-t", "-z", &subtree], MAX_LISTING)?;
        let mut remaining = MAX_TREE;
        for (index, entry) in listing
            .strip_suffix(b"\0")
            .ok_or_else(|| invalid("empty or incomplete Git subtree"))?
            .split(|byte| *byte == 0)
            .enumerate()
        {
            if index >= MAX_ENTRIES {
                return Err(invalid("Git tree entry limit"));
            }
            let entry =
                std::str::from_utf8(entry).map_err(|_| invalid("Git tree path is not UTF-8"))?;
            let (metadata, path) = entry
                .split_once('\t')
                .ok_or_else(|| invalid("invalid Git tree entry"))?;
            let destination = root.join(relative(Path::new(path))?);
            let (mode, id) = metadata
                .rsplit_once(' ')
                .ok_or_else(|| invalid("invalid Git tree metadata"))?;
            if id.len() != SHA_LEN || !id.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                return Err(invalid("invalid Git tree object ID"));
            }
            match mode {
                "040000 tree" => fs::create_dir(destination)?,
                "100644 blob" | "100755 blob" => {
                    let bytes = self.run(&["cat-file", "blob", id], remaining.min(MAX_FILE))?;
                    remaining -= bytes.len();
                    let mut file = fs::File::create_new(destination)?;
                    file.write_all(&bytes)?;
                    file.sync_all()?;
                }
                _ => {
                    return Err(invalid(
                        "Git skill contains a symlink, submodule or special entry",
                    ));
                }
            }
        }
        Ok(())
    }
}
