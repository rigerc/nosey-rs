//! Fingerprinted acquisition from a user-selected catalog snapshot.

use anyhow::{Context, Result, ensure};
use clap::{Args, Subcommand};
use nosey_core::{
    State,
    library::{BundleSource, Library},
    plan::{ChangeOutcome, ChangeRequest},
};
use nosey_registry::{CatalogIndex, RegistryInput, load_snapshot};
use std::path::PathBuf;

#[derive(Args)]
pub struct Selection {
    library: PathBuf,
    catalog: PathBuf,
    #[arg(long)]
    registry_id: String,
    artifact: String,
}

#[derive(Subcommand)]
pub enum Command {
    /// Review an Agent Skill bundle in a catalog snapshot without downloading it.
    PreviewAcquire(Selection),
    /// Acquire exactly the bundle in the approved acquisition plan.
    ApplyAcquire {
        #[command(flatten)]
        selection: Selection,
        #[arg(long)]
        approve: String,
    },
}

pub fn run(command: Command, state: &State) -> Result<()> {
    let (selection, approval) = match command {
        Command::PreviewAcquire(selection) => (selection, None),
        Command::ApplyAcquire { selection, approve } => (selection, Some(approve)),
    };
    let snapshot = load_snapshot(&selection.catalog)?;
    // The caller explicitly selects the snapshot and registry here. The
    // fingerprint binds that choice and its selected artifact before retrieval.
    let index = CatalogIndex::build(
        vec![RegistryInput {
            id: selection.registry_id,
            priority: 0,
            approved: true,
            enabled: true,
            catalog: snapshot.catalog,
        }],
        "",
    )?;
    let selected = index
        .skills
        .get(&selection.artifact)
        .context("Agent Skill entry not found; Git acquisition is unavailable")?;
    let request = ChangeRequest::AcquireLibrarySkill {
        source: BundleSource::from_selected(selected)?,
        install_name: None,
    };
    let library = Library::open(state, &selection.library)?;
    let plan = library.plan(state, &[request])?;
    if let Some(approval) = approval {
        ensure!(
            plan.fingerprint == approval,
            "stale approval; review the acquisition again"
        );
        let results = library.apply_plan(state, &plan)?;
        println!("{}", serde_json::to_string_pretty(&results)?);
        ensure!(
            results
                .iter()
                .all(|result| result.outcome == ChangeOutcome::Applied),
            "bundle acquisition did not complete"
        );
    } else {
        println!("{}", serde_json::to_string_pretty(&plan)?);
    }
    Ok(())
}
