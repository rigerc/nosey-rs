//! CLI commands for target settings and effective inventory.

use anyhow::{Context, Result};
use clap::Subcommand;
use nosey_core::{
    State,
    library::Library,
    targets::{AgentTarget, InstallScope, TargetConfig},
};
use std::path::{Path, PathBuf};

/// Target settings and inventory commands.
#[derive(Subcommand)]
pub enum Command {
    /// Audit managed, pending and detached link records without changing them.
    AuditLinks { library: PathBuf },

    /// Replace the desired target set for all projects and global installs.
    SetTargets {
        /// Enable the built-in shared agent location.
        #[arg(long)]
        agents: bool,
        /// Enable the built-in Claude location.
        #[arg(long)]
        claude: bool,
        /// Enable a custom project-relative target (repeatable).
        #[arg(long)]
        custom: Vec<PathBuf>,
    },
    /// Show configured target entries and effective skill coverage as JSON.
    Inventory {
        library: PathBuf,
        /// Inspect this registered project. Omit to inspect global targets.
        #[arg(long, required_unless_present = "home")]
        project: Option<PathBuf>,
        /// Use this absolute home for global targets or inherited project coverage.
        #[arg(long)]
        home: Option<PathBuf>,
    },
}

/// Execute a target command against the selected central state.
pub fn run(command: Command, state: &mut State, state_path: &Path) -> Result<()> {
    match command {
        Command::AuditLinks { library } => {
            let library = Library::open(state, &library)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&library.audit_links(state)?)?
            );
        }

        Command::SetTargets {
            agents,
            claude,
            custom,
        } => {
            let agents = [(AgentTarget::Agents, agents), (AgentTarget::Claude, claude)]
                .into_iter()
                .filter_map(|(target, enabled)| enabled.then_some(target))
                .collect();
            state.set_targets(TargetConfig {
                agents,
                custom: custom.into_iter().collect(),
            })?;
            state.save(state_path)?;
        }
        Command::Inventory {
            library,
            project,
            home,
        } => {
            let scope = match project {
                Some(root) => InstallScope::Project { root },
                None => InstallScope::Global {
                    home: home.clone().context("global inventory requires --home")?,
                },
            };
            let library = Library::open_initialized(state, &library)?;
            let inventory = library.inventory(state, &scope, home.as_deref())?;
            println!("{}", serde_json::to_string_pretty(&inventory)?);
        }
    }
    Ok(())
}
