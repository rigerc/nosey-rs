//! Configured agent targets and explicit install scopes.

use crate::{Error, State};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    path::{Component, PathBuf},
};

const MAX_CUSTOM_TARGETS: usize = 64;
const MAX_TARGET_DEPTH: usize = 32;

/// A built-in agent location available both in projects and under a home directory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentTarget {
    /// Shared agent skills location.
    Agents,
    /// Claude skills location.
    Claude,
}

#[cfg(target_os = "linux")]
impl AgentTarget {
    pub(crate) fn path(self) -> &'static std::path::Path {
        std::path::Path::new(match self {
            Self::Agents => ".agents/skills",
            Self::Claude => ".claude/skills",
        })
    }
}

/// The root against which configured targets are resolved.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum InstallScope {
    /// Global agent locations.
    Global {
        /// Caller-supplied absolute home directory.
        home: PathBuf,
    },
    /// A registered project.
    Project {
        /// Registered project directory.
        root: PathBuf,
    },
}

/// Desired target settings shared by all registered projects.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TargetConfig {
    /// Enabled built-in agent targets.
    pub agents: BTreeSet<AgentTarget>,
    /// Additional project-relative targets.
    pub custom: BTreeSet<PathBuf>,
}

impl Default for TargetConfig {
    fn default() -> Self {
        Self {
            agents: BTreeSet::from([AgentTarget::Agents]),
            custom: BTreeSet::new(),
        }
    }
}

impl TargetConfig {
    #[cfg(target_os = "linux")]
    pub(crate) fn paths(&self, global: bool) -> BTreeSet<PathBuf> {
        let mut paths: BTreeSet<_> = self
            .agents
            .iter()
            .map(|agent| agent.path().into())
            .collect();
        if !global {
            paths.extend(self.custom.iter().cloned());
        }
        paths
    }

    pub(crate) fn validate(&self) -> Result<(), Error> {
        if self.custom.len() > MAX_CUSTOM_TARGETS {
            return Err(Error::Invalid("too many custom targets".into()));
        }
        for path in &self.custom {
            if path.as_os_str().is_empty()
                || path.components().count() > MAX_TARGET_DEPTH
                || path.components().any(|component| {
                    !matches!(component, Component::Normal(part)
                        if part.to_str().is_some_and(|text| !text.chars().any(char::is_control)))
                })
            {
                return Err(Error::Invalid("invalid custom target path".into()));
            }
        }
        Ok(())
    }
}

impl State {
    /// Read the target configuration inherited by all projects.
    #[must_use]
    pub const fn targets(&self) -> &TargetConfig {
        &self.targets
    }

    /// Replace desired targets without changing any installed links.
    ///
    /// # Errors
    /// Rejects invalid paths and redirected targets in currently registered projects.
    pub fn set_targets(&mut self, config: TargetConfig) -> Result<(), Error> {
        config.validate()?;
        for project in self.projects.keys() {
            for path in &config.custom {
                let mut current = project.clone();
                for part in path {
                    current.push(part);
                    match std::fs::symlink_metadata(&current) {
                        Ok(meta) if meta.is_dir() => {}
                        Ok(_) => {
                            return Err(Error::Invalid(
                                "custom target is not a real directory".into(),
                            ));
                        }
                        Err(err) if err.kind() == std::io::ErrorKind::NotFound => break,
                        Err(err) => return Err(err.into()),
                    }
                }
            }
        }
        self.targets = config;
        Ok(())
    }
}
