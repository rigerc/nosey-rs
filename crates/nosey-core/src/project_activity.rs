// SPDX-License-Identifier: MIT AND Apache-2.0
//! Project activity ordering adapted from Kitter v0.2.6, src/config.rs.
//! Upstream: https://github.com/what1f/kitter/tree/19711db044aabfa9846a65aaee693276d65af5c2
//! See licenses/kitter-APACHE-2.0.txt and NOTICE.

use crate::{Error, State};
use std::{
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

impl State {
    /// Return registered projects, newest explicit activity first.
    ///
    /// Older prototype state without activity remains readable. Equal or absent
    /// timestamps are resolved by canonical path for deterministic ordering.
    #[must_use]
    pub fn project_paths(&self) -> Vec<PathBuf> {
        let mut paths: Vec<_> = self.projects.keys().cloned().collect();
        paths.sort_by(|left, right| {
            self.project_activity
                .get(right)
                .cmp(&self.project_activity.get(left))
                .then_with(|| left.cmp(right))
        });
        paths
    }

    /// Forget a registered directory without inspecting or changing its links.
    ///
    /// An exact stored absolute path is accepted even when a project has been
    /// deleted. A different existing path is resolved through canonicalization.
    ///
    /// # Errors
    ///
    /// Returns an error for an unregistered or inaccessible project path.
    pub fn unregister(&mut self, path: &Path) -> Result<PathBuf, Error> {
        let root = if self.projects.contains_key(path) {
            path.to_path_buf()
        } else {
            path.canonicalize()?
        };
        if self.projects.remove(&root).is_none() {
            return Err(Error::Invalid("project not registered".into()));
        }
        self.project_activity.remove(&root);
        Ok(root)
    }

    pub(crate) fn touch_project(&mut self, root: &Path) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .ok()
            .and_then(|duration| u64::try_from(duration.as_millis()).ok())
            .unwrap_or_default();
        let next = self.project_activity.values().copied().max()
            .unwrap_or_default().saturating_add(1);
        self.project_activity.insert(root.to_path_buf(), now.max(next));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn registers_once_orders_and_forgets_without_mutating_project() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let a = temp.path().join("a");
        let b = temp.path().join("b");
        fs::create_dir(&a)?;
        fs::create_dir(&b)?;
        fs::write(a.join("canary"), "keep")?;
        let mut state = State::default();
        let a = state.register(&a)?;
        let b = state.register(&b)?;
        assert_eq!(state.register(&a)?, a);
        assert_eq!(state.project_paths(), vec![b.clone(), a.clone()]);
        assert_eq!(state.unregister(&a)?, a);
        assert_eq!(fs::read_to_string(a.join("canary"))?, "keep");
        assert_eq!(state.project_paths(), vec![b.clone()]);
        assert!(!state.project_activity.contains_key(&a));
        fs::remove_dir(&b)?;
        assert_eq!(state.unregister(&b)?, b);
        assert!(state.project_paths().is_empty());
        Ok(())
    }

    #[test]
    fn old_state_without_activity_is_readable() -> Result<(), serde_json::Error> {
        let state: State = serde_json::from_str(r#"{"projects":{}}"#)?;
        assert!(state.project_activity.is_empty());
        Ok(())
    }
}
