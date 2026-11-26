//! Cleanup preview and apply commands.

use anyhow::Result;
use clap::Subcommand;
use nosey_core::{State, library::Library};
use std::path::PathBuf;

/// Read-only cleanup reviews and fingerprint-approved removal.
#[derive(Subcommand)]
pub enum Command {
    /// Inventory unused objects and unresolved journal references.
    CleanupReport { library: PathBuf },
    /// Review optional cleanup for an exact stored source path.
    PreviewRemovedSource { library: PathBuf, source: PathBuf },
    /// Review an unused library copy for removal.
    PreviewRemoveUnused {
        library: PathBuf,
        id: String,
        #[arg(
            long,
            help = "Attest that unmanaged references to this object were checked and none remain"
        )]
        external_references_checked: bool,
    },
    /// Remove an unused library copy using its approved fingerprint.
    ApplyRemoveUnused {
        library: PathBuf,
        id: String,
        #[arg(
            long,
            help = "Attest that unmanaged references to this object were checked and none remain"
        )]
        external_references_checked: bool,
        #[arg(long)]
        approve: String,
    },
}

/// Run a cleanup command against the loaded project state.
pub(super) fn run(command: Command, state: &State) -> Result<()> {
    match command {
        Command::CleanupReport { library } => {
            let library = Library::open(state, &library)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&library.cleanup_report(state)?)?
            );
        }
        Command::PreviewRemovedSource { library, source } => {
            let library = Library::open(state, &library)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&library.preview_removed_source(state, &source)?)?
            );
        }
        Command::PreviewRemoveUnused {
            library,
            id,
            external_references_checked,
        } => {
            let library = Library::open(state, &library)?;
            let preview = library.preview_remove_unused(state, &id, external_references_checked)?;
            println!("{}", serde_json::to_string_pretty(&preview)?);
        }
        Command::ApplyRemoveUnused {
            library,
            id,
            external_references_checked,
            approve,
        } => {
            let library = Library::open(state, &library)?;
            let preview = library.preview_remove_unused(state, &id, external_references_checked)?;
            anyhow::ensure!(
                preview.fingerprint == approve,
                "stale approval; review preview again"
            );
            library.apply_remove_unused(state, &preview)?;
            println!("removed {id}");
        }
    }
    Ok(())
}
