//! Previewed maintenance of existing link journals.

use super::{
    Library, LinkRecord, atomic_json, directory, exact_link, hash, identity, invalid, name, stat,
    stat_identity,
};
use crate::Error;
use rustix::fs::{AtFlags, fsync, unlinkat};
use serde::Serialize;
use std::{
    os::fd::OwnedFd,
    path::{Path, PathBuf},
};

/// Change proposed for a link journal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LinkAction {
    /// The observed journal and destination cannot be changed safely.
    Blocked,
    /// Drop a record whose destination is absent.
    DropRecord,
    /// Record the observed identity of the expected pending symlink.
    ConfirmLink,
}

/// A reviewed link maintenance operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LinkPreview {
    /// Exact absolute destination from the journal.
    pub destination: PathBuf,
    /// Proposed operation based on the current observation.
    pub action: LinkAction,
    /// Explanation of the proposed operation or refusal.
    pub detail: &'static str,
    /// Fingerprint of the operation, journal, artifact and destination observation.
    pub fingerprint: String,
}

impl Library {
    /// Inspect an existing journal to determine whether it can be recovered.
    ///
    /// # Errors
    ///
    /// Returns an error if the journal cannot be found or read.
    pub fn preview_recover(&self, destination: &Path) -> Result<LinkPreview, Error> {
        self.recovery(destination).map(|(preview, _)| preview)
    }

    /// Recover the journal after fresh revalidation of the approved preview.
    ///
    /// # Errors
    ///
    /// Rejects stale or blocked previews and reports journal I/O failures.
    pub fn apply_recover(&self, approved: &LinkPreview) -> Result<LinkAction, Error> {
        let (fresh, record) = self.recovery(&approved.destination)?;
        if &fresh != approved {
            return Err(invalid("stale recovery approval; review again"));
        }
        match fresh.action {
            LinkAction::Blocked => return Err(invalid(fresh.detail)),
            LinkAction::DropRecord => self.drop_record(&record.destination)?,
            LinkAction::ConfirmLink => {
                self.write_record(&record)?;
            }
        }
        Ok(fresh.action)
    }

    fn recovery(&self, destination: &Path) -> Result<(LinkPreview, LinkRecord), Error> {
        let mut record = self.maintenance_record(destination)?;
        let leaf = name(destination)?;
        let parent = destination
            .parent()
            .ok_or_else(|| invalid("missing destination parent"))?;
        let artifact = self.get(&record.artifact).ok();
        let parent = directory(parent).ok();
        let parent_identity = parent.as_ref().map(identity).transpose()?;
        let metadata = parent
            .as_ref()
            .map(|fd| stat(fd, leaf))
            .transpose()?
            .flatten();
        let observed = parent
            .as_ref()
            .map(|fd| exact_link(fd, leaf, &record.source))
            .transpose()?
            .flatten();
        let (action, detail) = if !artifact.as_ref().is_some_and(|skill| {
            record.source == self.object_dir(&record.artifact).join("skill")
                && leaf == Path::new(&skill.name)
                && record
                    .tree_sha256
                    .as_ref()
                    .is_none_or(|digest| digest == &skill.source.tree_sha256)
        }) {
            (LinkAction::Blocked, "artifact changed or unavailable")
        } else if parent_identity != Some(record.parent_identity) {
            (
                LinkAction::Blocked,
                "destination parent changed or unavailable",
            )
        } else if metadata.is_none() && observed.is_none() {
            (LinkAction::DropRecord, "destination is absent")
        } else if record.link_identity.is_none()
            && record.tree_sha256.is_some()
            && observed.is_some()
            && metadata.as_ref().map(stat_identity) == observed
        {
            (
                LinkAction::ConfirmLink,
                "expected pending symlink is present",
            )
        } else {
            (
                LinkAction::Blocked,
                "destination is occupied or ownership cannot be recovered",
            )
        };
        let fingerprint = hash(&serde_json::to_vec(&(
            "recover",
            action,
            detail,
            &record,
            artifact,
            parent_identity,
            observed,
            metadata.map(|s| (stat_identity(&s), s.st_mode, s.st_ctime, s.st_ctime_nsec)),
        ))?);
        if action == LinkAction::ConfirmLink {
            record.link_identity = observed;
        }
        Ok((
            LinkPreview {
                destination: destination.into(),
                action,
                detail,
                fingerprint,
            },
            record,
        ))
    }

    fn maintenance_record(&self, destination: &Path) -> Result<LinkRecord, Error> {
        let record = self
            .link_record(destination)?
            .ok_or_else(|| invalid("no link journal for destination"))?;
        let relative = destination
            .strip_prefix(&record.project)
            .map_err(|_| invalid("journal project mismatch"))?;
        crate::library_io::relative(relative)?;
        if !record.project.is_absolute()
            || record.project.starts_with(&self.root)
            || self.root.starts_with(&record.project)
            || record.reasons.is_empty()
        {
            return Err(invalid("invalid journal project or reasons"));
        }
        Ok(record)
    }

    fn write_record(&self, record: &LinkRecord) -> Result<(), Error> {
        atomic_json(
            &self.root.join("links"),
            &self.link_path(&record.destination),
            record,
            true,
        )
    }

    fn drop_record(&self, destination: &Path) -> Result<(), Error> {
        let journal = directory(&self.root.join("links"))?;
        remove_entry(&journal, name(&self.link_path(destination))?)
    }
}

fn remove_entry(parent: &OwnedFd, leaf: &Path) -> Result<(), Error> {
    unlinkat(parent, leaf, AtFlags::empty()).map_err(std::io::Error::from)?;
    fsync(parent).map_err(std::io::Error::from)?;
    Ok(())
}
