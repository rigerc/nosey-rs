//! Staged retrieval and extraction of complete Agent Skill bundles.

use super::{
    Acquisition, Availability, BundleSource, FIXTURE_PREFIX, Library, LocalSource, MAX_ENTRIES,
    MAX_FILE, MAX_TREE, PRIVATE_MODE, SkillRecord, atomic_json, hash, tree, validate_artifact_id,
};
use crate::{
    Error,
    library_io::{invalid, relative},
};
use nosey_registry::Fetch;
use serde::Serialize;
use std::{
    collections::BTreeSet,
    fs::{self, File},
    io::{self, Cursor, Read},
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};
use zip::ZipArchive;

const TYPE_MASK: u32 = 0o170_000;
const REGULAR: u32 = 0o100_000;
const DIRECTORY: u32 = 0o040_000;

#[cfg(test)]
mod tests;

/// Reviewed source and the state of its active library copy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AcquisitionPreview {
    /// Source to acquire.
    pub source: BundleSource,
    /// Existing verified copy, when it can be reused.
    pub cached: Option<SkillRecord>,
    /// Fingerprint over the source and observed library state.
    pub fingerprint: String,
}

impl Library {
    /// Inspect the active-copy state and validate an acquisition without fetching.
    ///
    /// # Errors
    /// Rejects invalid source metadata, corrupt content and conflicting active copies.
    pub fn preview_acquire(&self, source: &BundleSource) -> Result<AcquisitionPreview, Error> {
        source.validate().map_err(|e| invalid(&e.to_string()))?;
        validate_artifact_id(&source.artifact)?;
        if source.artifact.starts_with(FIXTURE_PREFIX) {
            return Err(invalid(
                "fixture identifiers are reserved for local content",
            ));
        }
        let cached = match fs::symlink_metadata(self.object_dir(&source.artifact)) {
            Ok(_) => {
                let record = self.get(&source.artifact)?;
                if record.acquisition != provenance(source)
                    || record.registry.as_ref() != Some(&source.registry)
                    || record.revision != source.revision
                {
                    return Err(invalid("existing artifact requires explicit update"));
                }
                Some(record)
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => None,
            Err(e) => return Err(e.into()),
        };
        Ok(AcquisitionPreview {
            source: source.clone(),
            fingerprint: hash(&serde_json::to_vec(&(source, &cached))?),
            cached,
        })
    }

    /// Acquire an explicitly selected bundle, reusing an intact matching copy.
    ///
    /// # Errors
    /// Rejects invalid source, fetch, digest, archive, manifest or publication.
    pub fn acquire_bundle(
        &self,
        source: &BundleSource,
        fetch: &impl Fetch,
    ) -> Result<SkillRecord, Error> {
        self.acquire_named_bundle(source, None, fetch)
    }

    pub(crate) fn acquire_named_bundle(
        &self,
        source: &BundleSource,
        name: Option<&str>,
        fetch: &impl Fetch,
    ) -> Result<SkillRecord, Error> {
        let _mutation = self.lock_mutation()?;
        if let Some(record) = self.preview_acquire(source)?.cached {
            check_name(name, &record.name)?;
            return Ok(record);
        }
        let (stage, record) = self.stage_bundle(source, name, fetch)?;
        self.publish_stage(
            &stage,
            &self.object_dir(&source.artifact),
            rustix::fs::RenameFlags::NOREPLACE,
        )?;
        Ok(record)
    }

    pub(super) fn stage_bundle(
        &self,
        source: &BundleSource,
        name: Option<&str>,
        fetch: &impl Fetch,
    ) -> Result<(tempfile::TempDir, SkillRecord), Error> {
        let bytes = source
            .fetch_verified(fetch)
            .map_err(|e| invalid(&e.to_string()))?;
        let stage = tempfile::Builder::new()
            .permissions(fs::Permissions::from_mode(PRIVATE_MODE))
            .tempdir_in(self.root.join("objects"))?;
        let skill = stage.path().join("skill");
        fs::create_dir(&skill)?;
        extract(&bytes, &skill)?;
        let (tree_sha256, actual_name) = tree(&skill, None)?;
        check_name(name, &actual_name)?;
        let record = SkillRecord {
            id: source.artifact.clone(),
            name: actual_name,
            source: LocalSource {
                path: PathBuf::new(),
                tree_sha256,
            },
            registry: Some(source.registry.clone()),
            revision: source.revision.clone(),
            acquisition: provenance(source),
            availability: Availability::Available,
        };
        atomic_json(
            stage.path(),
            &stage.path().join("record.json"),
            &record,
            false,
        )?;
        Ok((stage, record))
    }
}

fn provenance(source: &BundleSource) -> Acquisition {
    Acquisition::Bundle {
        url: source.url.clone(),
        media_type: source.media_type.clone(),
        digest: source.digest.clone(),
    }
}

fn extract(bytes: &[u8], root: &Path) -> Result<(), Error> {
    let mut archive =
        ZipArchive::new(Cursor::new(bytes)).map_err(|_| invalid("invalid skill ZIP"))?;
    if archive.len() > MAX_ENTRIES {
        return Err(invalid("skill ZIP entry limit"));
    }
    archive
        .validate_header_consistency()
        .map_err(|_| invalid("inconsistent skill ZIP headers"))?;
    let mut paths = BTreeSet::new();
    let mut entries = BTreeSet::new();
    let mut remaining = MAX_TREE as u64;
    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|_| invalid("invalid ZIP entry"))?;
        let raw = std::str::from_utf8(entry.name_raw())
            .map_err(|_| invalid("invalid ZIP path encoding"))?;
        let path = raw.strip_suffix('/').unwrap_or(raw);
        if path.contains(['\\', ':']) || path.split('/').any(|part| matches!(part, "" | "." | ".."))
        {
            return Err(invalid("unsafe ZIP path"));
        }
        let path = relative(Path::new(path))?;
        if !entries.insert(path.clone()) {
            return Err(invalid("duplicate ZIP path"));
        }
        for ancestor in path.ancestors().filter(|p| !p.as_os_str().is_empty()) {
            paths.insert(ancestor.to_path_buf());
            if paths.len() > MAX_ENTRIES {
                return Err(invalid("skill ZIP entry limit"));
            }
        }
        let destination = root.join(path);
        let directory = entry.is_dir();
        if entry.unix_mode().is_some_and(|mode| {
            let kind = mode & TYPE_MASK;
            kind != 0 && kind != if directory { DIRECTORY } else { REGULAR }
        }) {
            return Err(invalid("skill ZIP contains a link or special entry"));
        }
        if directory {
            if entry.size() != 0 {
                return Err(invalid("nonempty ZIP directory entry"));
            }
            fs::create_dir_all(&destination)?;
        } else {
            let limit = remaining.min(MAX_FILE as u64);
            if entry.size() > limit {
                return Err(invalid("skill ZIP size limit"));
            }
            let parent = destination
                .parent()
                .ok_or_else(|| invalid("missing ZIP entry parent"))?;
            fs::create_dir_all(parent)?;
            let mut file = File::create_new(destination)?;
            let declared = entry.size();
            let copied = io::copy(&mut entry.by_ref().take(limit + 1), &mut file)?;
            if copied > limit || copied != declared {
                return Err(invalid("skill ZIP size mismatch"));
            }
            remaining -= copied;
            file.sync_all()?;
        }
    }
    Ok(())
}

fn check_name(expected: Option<&str>, actual: &str) -> Result<(), Error> {
    if expected.is_some_and(|name| name != actual) {
        return Err(invalid(
            "manifest name differs from the reviewed install name",
        ));
    }
    Ok(())
}
