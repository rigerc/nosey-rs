//! Previewed maintenance of existing link journals.

use super::{
    InstallReason, Library, LinkRecord, atomic_json, directory, exact_link, hash, identity,
    invalid, name, stat, stat_identity,
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
    /// Remove one reason while keeping the link.
    RemoveReason,
    /// Remove the final reason, link and journal.
    RemoveLink,
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
    /// Reason selected for unlink, when applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<InstallReason>,
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
        let _mutation = self.lock_mutation()?;
        let (fresh, record) = self.recovery(&approved.destination)?;
        if &fresh != approved {
            return Err(invalid("stale recovery approval; review again"));
        }
        match fresh.action {
            LinkAction::Blocked | LinkAction::RemoveReason | LinkAction::RemoveLink => {
                return Err(invalid(fresh.detail));
            }
            LinkAction::DropRecord => self.drop_record(&record.destination)?,
            LinkAction::ConfirmLink => {
                let parent = directory(journal_parent(&record.destination)?)?;
                fsync(&parent).map_err(std::io::Error::from)?;
                if identity(&parent)? != record.parent_identity
                    || exact_link(&parent, name(&record.destination)?, &record.source)?
                        != record.link_identity
                {
                    return Err(invalid("destination changed during recovery"));
                }
                self.write_record(&record)?;
            }
        }
        Ok(fresh.action)
    }

    fn recovery(&self, destination: &Path) -> Result<(LinkPreview, LinkRecord), Error> {
        let mut record = self.maintenance_record(destination)?;
        let leaf = name(destination)?;
        let parent = journal_parent(destination)?;
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
                && (record.link_identity.is_some()
                    || record
                        .tree_sha256
                        .as_ref()
                        .is_none_or(|digest| digest == &skill.source.tree_sha256))
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
                reason: None,
                fingerprint,
            },
            record,
        ))
    }

    /// Preview removing one reason from an existing managed link.
    ///
    /// # Errors
    ///
    /// Returns an error for an unreadable or missing journal.
    pub fn preview_unlink(
        &self,
        destination: &Path,
        reason: &InstallReason,
    ) -> Result<LinkPreview, Error> {
        self.unlink_plan(destination, reason)
            .map(|(preview, _)| preview)
    }

    /// Remove the approved reason and, for the final reason, the verified symlink.
    ///
    /// # Errors
    ///
    /// Rejects stale approvals, missing reasons and changed destinations.
    pub fn apply_unlink(&self, approved: &LinkPreview) -> Result<LinkAction, Error> {
        let _mutation = self.lock_mutation()?;
        let reason = approved
            .reason
            .as_ref()
            .ok_or_else(|| invalid("unlink approval lacks an install reason"))?;
        let (fresh, mut record) = self.unlink_plan(&approved.destination, reason)?;
        if &fresh != approved {
            return Err(invalid("stale unlink approval; review again"));
        }
        match fresh.action {
            LinkAction::RemoveReason => {
                record.reasons.remove(reason);
                self.write_record(&record)?;
            }
            LinkAction::RemoveLink => {
                let parent = directory(journal_parent(&record.destination)?)?;
                let leaf = name(&record.destination)?;
                if identity(&parent)? != record.parent_identity
                    || exact_link(&parent, leaf, &record.source)? != record.link_identity
                {
                    return Err(invalid("destination changed before unlink"));
                }
                remove_entry(&parent, leaf)?;
                self.drop_record(&record.destination)?;
            }
            _ => return Err(invalid(fresh.detail)),
        }
        Ok(fresh.action)
    }

    fn unlink_plan(
        &self,
        destination: &Path,
        reason: &InstallReason,
    ) -> Result<(LinkPreview, LinkRecord), Error> {
        let record = self.maintenance_record(destination)?;
        let leaf = name(destination)?;
        let artifact = self.get(&record.artifact).ok();
        let parent = directory(journal_parent(destination)?).ok();
        let parent_identity = parent.as_ref().map(identity).transpose()?;
        let observed = parent
            .as_ref()
            .map(|fd| exact_link(fd, leaf, &record.source))
            .transpose()?
            .flatten();
        let metadata = parent
            .as_ref()
            .map(|fd| stat(fd, leaf))
            .transpose()?
            .flatten();
        let (action, detail) = if !artifact.as_ref().is_some_and(|skill| {
            record.source == self.object_dir(&record.artifact).join("skill")
                && leaf == Path::new(&skill.name)
        }) {
            (LinkAction::Blocked, "artifact invalid or unavailable")
        } else if !record.reasons.contains(reason) {
            (LinkAction::Blocked, "install reason is absent")
        } else if parent_identity != Some(record.parent_identity)
            || observed.is_none()
            || observed != record.link_identity
            || metadata.as_ref().map(stat_identity) != observed
        {
            (
                LinkAction::Blocked,
                "destination is no longer the managed link",
            )
        } else if record.reasons.len() > 1 {
            (
                LinkAction::RemoveReason,
                "other reasons still require the link",
            )
        } else {
            (
                LinkAction::RemoveLink,
                "removing the last reason removes the link",
            )
        };
        let fingerprint = hash(&serde_json::to_vec(&(
            "unlink",
            action,
            detail,
            reason,
            &record,
            artifact,
            parent_identity,
            observed,
            metadata.map(|s| (stat_identity(&s), s.st_mode, s.st_ctime, s.st_ctime_nsec)),
        ))?);
        Ok((
            LinkPreview {
                destination: destination.into(),
                action,
                detail,
                reason: Some(reason.clone()),
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
        let boundary = if record.global {
            destination
                .parent()
                .ok_or_else(|| invalid("missing global target"))?
        } else {
            &record.project
        };
        if !record.project.is_absolute()
            || boundary.starts_with(&self.root)
            || self.root.starts_with(boundary)
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

fn journal_parent(destination: &Path) -> Result<&Path, Error> {
    destination
        .parent()
        .ok_or_else(|| invalid("missing destination parent"))
}
