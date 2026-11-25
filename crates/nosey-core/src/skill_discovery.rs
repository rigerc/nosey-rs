// SPDX-License-Identifier: MIT AND Apache-2.0
//! Native read-only installation discovery adapted from Kitter v0.2.6
//! src/effective_skills/scanner.rs (direct_children) and src/directory_link.rs.
//! Upstream: https://github.com/what1f/kitter/tree/19711db044aabfa9846a65aaee693276d65af5c2
//! See licenses/kitter-APACHE-2.0.txt and NOTICE.

use crate::{Error, State};
use serde::Serialize;
use std::{
    fs, io,
    path::{Component, Path, PathBuf},
};

const MAX_ENTRIES: usize = 10_000;
const MAX_TARGET_DEPTH: usize = 32;
const MANIFEST: &str = "SKILL.md";

/// A candidate installation, not proof of ownership or canonical identity.
#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Candidate {
    /// An ordinary directory containing a regular manifest.
    Manifest {
        /// Project-relative path of the manifest.
        path: PathBuf,
    },
    /// A symbolic link, not followed or classified as managed.
    UnverifiedLink {
        /// Project-relative path of the link.
        path: PathBuf,
        /// Link target as stored on disk, not resolved or opened.
        target: PathBuf,
    },
}

/// Direct candidates in one explicitly selected project-relative target.
#[derive(Debug, Serialize)]
pub struct TargetAudit {
    /// Target relative to the registered project.
    pub target: PathBuf,
    /// Deterministically ordered direct candidates.
    pub candidates: Vec<Candidate>,
}

/// Inspect a registered project's explicit target without modifying anything.
///
/// Adapted from Kitter's direct-child discovery: check the root manifest and
/// immediate children, then sort. Unlike Kitter, do not follow linked roots
/// or linked skills, open manifest contents, infer ownership, or infer IDs.
/// An absent target is empty; access errors or budget exhaustion are errors
/// rather than misleading complete observations.
///
/// # Errors
///
/// Rejects unregistered projects, escaping or symlinked target components,
/// non-directory targets, inaccessible entries and excessive target depth or
/// enumeration size. Intended for normal local trees, not concurrent hostile
/// filesystem mutation (metadata checks are not race-proof).
pub fn inspect_target(
    state: &State,
    project: &Path,
    target: &Path,
) -> Result<TargetAudit, Error> {
    let root = project.canonicalize()?;
    if !state.projects.contains_key(&root) {
        return Err(Error::Invalid("project not registered".into()));
    }
    let mut dir = root.clone();
    let mut relative = PathBuf::new();
    let mut absent = false;
    let mut depth = 0;
    for component in target.components() {
        let Component::Normal(part) = component else {
            return Err(Error::Invalid("target must be a relative descendant".into()));
        };
        depth += 1;
        if depth > MAX_TARGET_DEPTH {
            return Err(Error::Invalid("target depth limit".into()));
        }
        relative.push(part);
        dir.push(part);
        if absent {
            continue;
        }
        match metadata(&dir)? {
            None => absent = true,
            Some(m) if m.is_dir() && !m.file_type().is_symlink() => {}
            Some(_) => return Err(Error::Invalid("target component is not a real directory".into())),
        }
    }
    if depth == 0 {
        return Err(Error::Invalid("target must be a relative descendant".into()));
    }
    let mut audit = TargetAudit { target: relative.clone(), candidates: Vec::new() };
    if absent {
        return Ok(audit);
    }
    inspect_manifest(&dir, &relative, &mut audit.candidates)?;
    let mut entries = Vec::new();
    for entry in fs::read_dir(&dir)? {
        if entries.len() >= MAX_ENTRIES {
            return Err(Error::Invalid("target enumeration limit".into()));
        }
        entries.push(entry?);
    }
    entries.sort_by_key(fs::DirEntry::file_name);
    for entry in entries {
        let path = entry.path();
        let relative_path = relative.join(entry.file_name());
        let kind = entry.file_type()?;
        if kind.is_symlink() {
            audit.candidates.push(Candidate::UnverifiedLink {
                path: relative_path,
                target: fs::read_link(path)?,
            });
        } else if kind.is_dir() {
            inspect_manifest(&path, &relative_path, &mut audit.candidates)?;
        }
    }
    Ok(audit)
}

fn metadata(path: &Path) -> io::Result<Option<fs::Metadata>> {
    match fs::symlink_metadata(path) {
        Ok(m) => Ok(Some(m)),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e),
    }
}

fn inspect_manifest(
    dir: &Path,
    relative: &Path,
    candidates: &mut Vec<Candidate>,
) -> io::Result<()> {
    let path = dir.join(MANIFEST);
    if let Some(m) = metadata(&path)? {
        if m.is_file() {
            candidates.push(Candidate::Manifest { path: relative.join(MANIFEST) });
        }
        // A linked manifest is not read or considered a regular manifest.
        // Its containing directory is not asserted to be a skill.
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discovers_direct_manifests_and_not_nested_groups() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let target = temp.path().join("skills");
        fs::create_dir_all(target.join("a"))?;
        fs::create_dir_all(target.join("group/nested"))?;
        fs::write(target.join("a/SKILL.md"), "untouched")?;
        fs::write(target.join("group/nested/SKILL.md"), "ignored")?;
        let mut state = State::default();
        let project = state.register(temp.path())?;
        let result = inspect_target(&state, &project, Path::new("skills"))?;
        assert_eq!(result.candidates, vec![Candidate::Manifest { path: "skills/a/SKILL.md".into() }]);
        assert_eq!(fs::read_to_string(target.join("a/SKILL.md"))?, "untouched");
        assert!(inspect_target(&state, &project, Path::new("missing")).is_ok());
        assert!(inspect_target(&state, &project, Path::new("missing/../escape")).is_err());
        assert!(inspect_target(&state, &project, Path::new("/tmp")).is_err());
        assert!(inspect_target(&State::default(), &project, Path::new("skills")).is_err());
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn reports_links_without_following_or_claiming_ownership() -> Result<(), Box<dyn std::error::Error>> {
        use std::os::unix::fs::symlink;
        let temp = tempfile::tempdir()?;
        let outside = tempfile::tempdir()?;
        fs::write(outside.path().join(MANIFEST), "outside")?;
        let target = temp.path().join("skills");
        fs::create_dir(&target)?;
        symlink(outside.path(), target.join("linked"))?;
        symlink("../missing", target.join("broken"))?;
        symlink(outside.path(), temp.path().join("linked-root"))?;
        let mut state = State::default();
        let project = state.register(temp.path())?;
        let result = inspect_target(&state, &project, Path::new("skills"))?;
        assert_eq!(result.candidates, vec![
            Candidate::UnverifiedLink { path: "skills/broken".into(), target: "../missing".into() },
            Candidate::UnverifiedLink { path: "skills/linked".into(), target: outside.path().to_path_buf() },
        ]);
        assert!(inspect_target(&state, &project, Path::new("linked-root")).is_err());
        assert_eq!(fs::read_to_string(outside.path().join(MANIFEST))?, "outside");
        Ok(())
    }
}
