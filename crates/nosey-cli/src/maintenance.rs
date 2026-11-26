//! CLI entry points for previewed library maintenance.

use anyhow::Result;
use clap::Subcommand;
use nosey_core::{State, changes::InstallReason, library::Library};
use std::path::PathBuf;

#[derive(Subcommand)]
/// Library maintenance subcommands flattened into the main CLI.
pub enum Command {
    /// Show how an interrupted link journal can be recovered.
    PreviewRecover {
        library: PathBuf,
        destination: PathBuf,
    },
    /// Recover a link journal using an approved preview.
    ApplyRecover {
        library: PathBuf,
        destination: PathBuf,
        #[arg(long)]
        approve: String,
    },
    /// Preview removal of one install reason.
    PreviewUnlink {
        library: PathBuf,
        destination: PathBuf,
        #[arg(long, default_value = "manual", value_parser = parse_install_reason)]
        reason: InstallReason,
    },
    /// Remove one reason using an approved preview.
    ApplyUnlink {
        library: PathBuf,
        destination: PathBuf,
        #[arg(long, default_value = "manual", value_parser = parse_install_reason)]
        reason: InstallReason,
        #[arg(long)]
        approve: String,
    },
    /// Preview replacement of an existing fixture object.
    PreviewUpdate {
        library: PathBuf,
        id: String,
        source: PathBuf,
    },
    /// Atomically replace a fixture using an approved preview.
    ApplyUpdate {
        library: PathBuf,
        id: String,
        source: PathBuf,
        #[arg(long)]
        approve: String,
    },
}

fn parse_install_reason(value: &str) -> Result<InstallReason, String> {
    match value {
        "manual" => Ok(InstallReason::Manual),
        "recommendation" => Ok(InstallReason::Recommendation),
        _ => value
            .strip_prefix("preset:")
            .filter(|id| !id.is_empty())
            .map(|id| InstallReason::Preset(id.into()))
            .ok_or_else(|| "expected manual, recommendation or preset:<id>".into()),
    }
}

/// Run a maintenance command against the loaded project state.
pub fn run(command: Command, state: &State) -> Result<()> {
    match command {
        Command::PreviewRecover {
            library,
            destination,
        } => {
            let library = Library::open(state, &library)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&library.preview_recover(&destination)?)?
            );
        }
        Command::ApplyRecover {
            library,
            destination,
            approve,
        } => {
            let library = Library::open(state, &library)?;
            let preview = library.preview_recover(&destination)?;
            anyhow::ensure!(
                preview.fingerprint == approve,
                "stale approval; review preview again"
            );
            println!(
                "{}",
                serde_json::to_string_pretty(&library.apply_recover(&preview)?)?
            );
        }
        Command::PreviewUnlink {
            library,
            destination,
            reason,
        } => {
            let library = Library::open(state, &library)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&library.preview_unlink(&destination, &reason)?)?
            );
        }
        Command::ApplyUnlink {
            library,
            destination,
            reason,
            approve,
        } => {
            let library = Library::open(state, &library)?;
            let preview = library.preview_unlink(&destination, &reason)?;
            anyhow::ensure!(
                preview.fingerprint == approve,
                "stale approval; review preview again"
            );
            println!(
                "{}",
                serde_json::to_string_pretty(&library.apply_unlink(&preview)?)?
            );
        }
        Command::PreviewUpdate {
            library,
            id,
            source,
        } => {
            let library = Library::open(state, &library)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&library.preview_update(&id, &source)?)?
            );
        }
        Command::ApplyUpdate {
            library,
            id,
            source,
            approve,
        } => {
            let library = Library::open(state, &library)?;
            let preview = library.preview_update(&id, &source)?;
            anyhow::ensure!(
                preview.fingerprint == approve,
                "stale approval; review preview again"
            );
            println!(
                "{}",
                serde_json::to_string_pretty(&library.apply_update(&preview)?)?
            );
        }
    }
    Ok(())
}
