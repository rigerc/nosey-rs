//! Read-only audit of link journals, including detached and unresolved records.

use super::{InstallReason, LinkRecord, Status, classify, name};
use crate::{
    Error, State,
    library::{Library, MAX_RECORD},
    library_io::{directory, invalid, read},
};
use serde::Serialize;
use std::{collections::BTreeSet, path::PathBuf};

const MAX_JOURNAL_ENTRIES: usize = 4096;

/// Connection between a journal and its recorded destination.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum JournalConnection {
    /// The journal never recorded a confirmed inode.
    Pending,
    /// The destination still matches the recorded symlink.
    Connected,
    /// Confirmed ownership can no longer be established.
    Disconnected,
}

/// Read-only ownership and retained-reference report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LinkAudit {
    /// Artifact identifier.
    pub artifact: String,
    /// Original registered project path, even after unregister.
    pub project: PathBuf,
    /// Destination path.
    pub destination: PathBuf,
    /// Source path recorded in the journal.
    pub source: PathBuf,
    /// Independent retained reasons.
    pub reasons: BTreeSet<InstallReason>,
    /// True if project is no longer registered.
    pub detached: bool,
    /// Whether the original install used a global agent location.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub global: bool,
    /// Ownership evidence at the recorded destination.
    pub connection: JournalConnection,
    /// The recorded artifact is missing or fails verification.
    pub artifact_unavailable: bool,
    /// Current observed ownership status.
    pub status: Status,
}

impl Library {
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
        for (index, entry) in std::fs::read_dir(&dir)?.enumerate() {
            if index >= MAX_JOURNAL_ENTRIES {
                return Err(invalid("journal entry limit"));
            }
            let entry = entry?;
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
            let artifact_unavailable = self.get(&r.artifact).is_err();
            let observed = directory(parent)
                .and_then(|p| classify(&p, name(&r.destination)?, &r.source, Some(&r)))
                .unwrap_or(Status::RecoveryRequired);
            let status =
                if artifact_unavailable || r.source != self.object_dir(&r.artifact).join("skill") {
                    Status::RecoveryRequired
                } else {
                    observed
                };
            result.push(LinkAudit {
                artifact: r.artifact,
                detached: !r.global && !state.projects.contains_key(&r.project),
                global: r.global,
                project: r.project,
                destination: r.destination,
                source: r.source,
                reasons: r.reasons,
                connection: if r.link_identity.is_none() {
                    JournalConnection::Pending
                } else if observed == Status::Managed {
                    JournalConnection::Connected
                } else {
                    JournalConnection::Disconnected
                },
                artifact_unavailable,
                status,
            });
        }
        result.sort_by(|a, b| a.destination.cmp(&b.destination));
        Ok(result)
    }
}
