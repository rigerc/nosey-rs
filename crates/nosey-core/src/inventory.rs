//! Read-only inventory of configured target entries and installation coverage.

use crate::{
    Error, State,
    changes::{LinkAudit, Status},
    library::{Library, manifest::manifest},
    library_io::{descend, directory, invalid, read, stat},
    targets::InstallScope,
};
use rustix::fs::{Dir, FileType};
use serde::Serialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    io,
    os::unix::ffi::OsStrExt,
    path::{Path, PathBuf},
};

const MAX_TARGET_ENTRIES: usize = 10_000;
const MAX_MANIFEST_BYTES: usize = 1024 * 1024;

/// Observed type of an unmanaged entry.
#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EntryKind {
    /// Directory.
    Directory,
    /// Symbolic link, observed without following it.
    Symlink,
    /// Regular file.
    File,
    /// Other filesystem object.
    Other,
}

/// An unmanaged entry, with no journal ownership inferred.
#[derive(Debug, Serialize)]
pub struct UnmanagedEntry {
    /// Absolute path of the entry.
    pub path: PathBuf,
    /// Filesystem type.
    pub kind: EntryKind,
}

/// All observations for one library artifact.
#[derive(Debug, Default, Serialize)]
pub struct ManagedGroup {
    /// Recorded links, including those needing recovery.
    pub links: Vec<LinkAudit>,
    /// Enabled target paths covered by a verified project or global link.
    pub covered_targets: BTreeSet<PathBuf>,
    /// At least one, but fewer than all, enabled targets are covered.
    pub partial: bool,
}

/// Effective skill inventory for a single scope.
#[derive(Debug, Serialize)]
pub struct Inventory {
    /// Absolute paths for every enabled target in this scope.
    pub targets: BTreeSet<PathBuf>,
    /// Managed groups keyed by canonical library artifact identity.
    pub managed: BTreeMap<String, ManagedGroup>,
    /// Unmanaged entries grouped by manifest name.
    pub unmanaged: BTreeMap<String, Vec<UnmanagedEntry>>,
    /// Unmanaged entries without a readable, valid manifest name.
    pub unidentified: Vec<UnmanagedEntry>,
    /// Journal destinations outside the enabled targets for this scope.
    pub orphaned: BTreeSet<PathBuf>,
}

impl Library {
    /// Inspect configured targets and link journals without changing them.
    /// A supplied home contributes built-in global coverage to a project.
    ///
    /// # Errors
    /// Rejects unknown project roots, redirected targets, and invalid journals.
    pub fn inventory(
        &self,
        state: &State,
        scope: &InstallScope,
        home: Option<&Path>,
    ) -> Result<Inventory, Error> {
        let (root, global) = match scope {
            InstallScope::Global { home } => (home.clone(), true),
            InstallScope::Project { root } => {
                let root = root.canonicalize()?;
                if !state.projects.contains_key(&root) {
                    return Err(invalid("project not registered"));
                }
                (root, false)
            }
        };
        directory(&root)?;
        let targets = state
            .targets()
            .paths(global)
            .into_iter()
            .map(|path| root.join(path))
            .collect();
        let mut inventory = Inventory {
            targets,
            managed: BTreeMap::new(),
            unmanaged: BTreeMap::new(),
            unidentified: Vec::new(),
            orphaned: BTreeSet::new(),
        };
        let global_paths = state.targets().paths(true);
        let mut owned = BTreeSet::new();
        for link in self.audit_links(state)? {
            let inherited = !global && link.global && home == Some(link.project.as_path());
            if !inherited && (link.global != global || link.project != root) {
                continue;
            }
            let parent = link
                .destination
                .parent()
                .ok_or_else(|| invalid("missing journal parent"))?;
            let relative = parent
                .strip_prefix(&link.project)
                .map_err(|_| invalid("journal parent outside scope"))?;
            let target = root.join(relative);
            let enabled = inventory.targets.contains(&target)
                && (!inherited || global_paths.contains(relative));
            if !enabled {
                inventory.orphaned.insert(link.destination.clone());
            }
            let group = inventory.managed.entry(link.artifact.clone()).or_default();
            if link.status == Status::Managed {
                owned.insert(link.destination.clone());
                if enabled {
                    group.covered_targets.insert(target);
                }
            }
            group.links.push(link);
        }
        for group in inventory.managed.values_mut() {
            group.partial = !group.covered_targets.is_empty()
                && group.covered_targets.len() < inventory.targets.len();
        }
        for target in &inventory.targets {
            for (name, entry) in entries(target, &owned)? {
                if let Some(name) = name {
                    inventory.unmanaged.entry(name).or_default().push(entry);
                } else {
                    inventory.unidentified.push(entry);
                }
            }
        }
        Ok(inventory)
    }
}

/// Find unmanaged symlinks with exact, verified library provenance.
pub(crate) fn unmanaged_library_links(
    library: &Library,
    target: &Path,
    owned: &BTreeSet<PathBuf>,
    sources: &BTreeMap<PathBuf, &String>,
) -> Result<BTreeSet<String>, Error> {
    let observations = entries(target, owned)?;
    let mut matches = BTreeSet::new();
    for (_, entry) in observations {
        if !matches!(entry.kind, EntryKind::Symlink) {
            continue;
        }
        let parent = directory(target)?;
        let leaf = entry
            .path
            .file_name()
            .ok_or_else(|| invalid("missing link name"))?;
        let destination =
            rustix::fs::readlinkat(&parent, leaf, Vec::new()).map_err(io::Error::from)?;
        let source = Path::new(std::ffi::OsStr::from_bytes(destination.to_bytes()));
        if let Some(&id) = sources.get(source)
            && library.get(id).is_ok()
        {
            matches.insert(id.clone());
        }
    }
    Ok(matches)
}

fn entries(
    target: &Path,
    owned: &BTreeSet<PathBuf>,
) -> Result<Vec<(Option<String>, UnmanagedEntry)>, Error> {
    let parent = match directory(target) {
        Ok(parent) => parent,
        Err(Error::Io(err)) if err.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(err),
    };
    let mut entries = Vec::new();
    for (index, item) in Dir::read_from(&parent)
        .map_err(io::Error::from)?
        .enumerate()
    {
        if index >= MAX_TARGET_ENTRIES {
            return Err(invalid("target enumeration limit"));
        }
        let item = item.map_err(io::Error::from)?;
        let leaf = Path::new(std::ffi::OsStr::from_bytes(item.file_name().to_bytes()));
        let path = target.join(leaf);
        if leaf == Path::new(".") || leaf == Path::new("..") || owned.contains(&path) {
            continue;
        }
        let Some(meta) = stat(&parent, leaf)? else {
            continue;
        };
        let kind = match FileType::from_raw_mode(meta.st_mode) {
            FileType::Directory => EntryKind::Directory,
            FileType::Symlink => EntryKind::Symlink,
            FileType::RegularFile => EntryKind::File,
            _ => EntryKind::Other,
        };
        let name = if matches!(kind, EntryKind::Directory) {
            descend(&parent, leaf)
                .and_then(|dir| read(&dir, Path::new("SKILL.md"), MAX_MANIFEST_BYTES))
                .and_then(|bytes| manifest(&bytes))
                .ok()
        } else {
            None
        };
        entries.push((name, UnmanagedEntry { path, kind }));
    }
    entries.sort_by(|a, b| a.1.path.cmp(&b.1.path));
    Ok(entries)
}
