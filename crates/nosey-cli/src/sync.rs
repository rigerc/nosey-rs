//! Desired preset settings and reviewed synchronization commands.

use anyhow::{Result, ensure};
use clap::{Args, Subcommand};
use nosey_core::{
    State,
    library::Library,
    presets::{Preset, PresetApplication},
    targets::InstallScope,
};
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

/// Select the project or global scope for a preset application.
#[derive(Args, Clone)]
pub struct Scope {
    /// Use this registered project root.
    #[arg(long, required_unless_present = "home", conflicts_with = "home")]
    project: Option<PathBuf>,
    /// Use supported global locations under this absolute home.
    #[arg(long, required_unless_present = "project")]
    home: Option<PathBuf>,
}

impl Scope {
    fn install_scope(self) -> Result<InstallScope> {
        match (self.project, self.home) {
            (Some(root), None) => Ok(InstallScope::Project { root }),
            (None, Some(home)) => Ok(InstallScope::Global { home }),
            _ => anyhow::bail!("select exactly one scope"),
        }
    }
}

/// Preset and target synchronization subcommands.
#[derive(Subcommand)]
pub enum Command {
    /// Define a preset or replace its full membership.
    SetPreset {
        id: String,
        name: String,
        /// Include this library artifact ID (repeatable).
        #[arg(long)]
        member: Vec<String>,
    },
    /// Add or remove members from a linked preset.
    EditPreset {
        id: String,
        /// Add this artifact ID (repeatable).
        #[arg(long)]
        add: Vec<String>,
        /// Remove this artifact ID (repeatable).
        #[arg(long)]
        remove: Vec<String>,
    },
    /// Record desired application and show its pending preset changes.
    ApplyPreset {
        library: PathBuf,
        id: String,
        #[command(flatten)]
        scope: Scope,
    },
    /// Remove desired application and show its pending preset changes.
    UnapplyPreset {
        library: PathBuf,
        id: String,
        #[command(flatten)]
        scope: Scope,
    },
    /// Review target synchronization or apply its reviewed fingerprint.
    SyncTargets {
        library: PathBuf,
        #[arg(long)]
        approve: Option<String>,
    },
    /// Review preset synchronization or apply its reviewed fingerprint.
    SyncPresets {
        library: PathBuf,
        #[arg(long)]
        approve: Option<String>,
    },
}

/// Update desired settings or review and apply a synchronization plan.
pub fn run(command: Command, state: &mut State, state_path: &Path) -> Result<()> {
    match command {
        Command::SetPreset { id, name, member } => {
            state.set_preset(Preset {
                id,
                name,
                members: member.into_iter().collect(),
            })?;
            state.save(state_path)?;
        }
        Command::EditPreset { id, add, remove } => {
            let mut preset = state
                .preset(&id)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("unknown preset {id}"))?;
            let removals: BTreeSet<_> = remove.into_iter().collect();
            ensure!(
                !add.iter().any(|id| removals.contains(id)),
                "cannot add and remove the same preset member"
            );
            preset.members.retain(|id| !removals.contains(id));
            preset.members.extend(add);
            state.set_preset(preset)?;
            state.save(state_path)?;
        }
        Command::ApplyPreset {
            ref library,
            ref id,
            scope: ref selected,
        }
        | Command::UnapplyPreset {
            ref library,
            ref id,
            scope: ref selected,
        } => {
            let enabled = matches!(command, Command::ApplyPreset { .. });
            state.set_preset_application(
                PresetApplication {
                    preset: id.clone(),
                    scope: selected.clone().install_scope()?,
                },
                enabled,
            )?;
            let library = Library::open(state, library)?;
            let plan = library.plan_preset_sync(state)?;
            state.save(state_path)?;
            println!("{}", serde_json::to_string_pretty(&plan)?);
        }
        Command::SyncTargets { library, approve } => {
            let library = Library::open(state, &library)?;
            let plan = library.plan_target_sync(state)?;
            super::plan::review_or_apply(&library, state, &plan, approve.as_deref())?;
        }
        Command::SyncPresets { library, approve } => {
            let library = Library::open(state, &library)?;
            let plan = library.plan_preset_sync(state)?;
            super::plan::review_or_apply(&library, state, &plan, approve.as_deref())?;
        }
    }
    Ok(())
}
