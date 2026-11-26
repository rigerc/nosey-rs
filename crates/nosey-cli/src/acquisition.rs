//! Fingerprinted acquisition from a user-selected catalog snapshot.

use anyhow::{Context, Result, ensure};
use clap::{Args, Subcommand};
use nosey_core::{
    State,
    library::{BundleSource, GitSource, Library},
    plan::{AcquisitionSource, ChangeOutcome, ChangeRequest},
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

#[derive(Args)]
pub struct GitSelection {
    library: PathBuf,
    artifact: String,
    #[arg(long)]
    repo: String,
    #[arg(long)]
    path: PathBuf,
    #[arg(long)]
    revision: String,
}

#[derive(Subcommand)]
pub enum Command {
    /// Review an Agent Skill bundle in a catalog snapshot without downloading it.
    PreviewAcquire(Selection),
    /// Review an explicitly approved HTTPS Git repository and pinned skill subtree.
    PreviewAcquireGit(GitSelection),
    /// Acquire the exact Git commit and subtree from the approved plan.
    ApplyAcquireGit {
        #[command(flatten)]
        selection: GitSelection,
        #[arg(long)]
        approve: String,
    },
    /// Acquire exactly the bundle in the approved acquisition plan.
    ApplyAcquire {
        #[command(flatten)]
        selection: Selection,
        #[arg(long)]
        approve: String,
    },
}

pub fn run(command: Command, state: &State) -> Result<()> {
    let (library, source, approval) = match command {
        Command::PreviewAcquire(selection) => (
            selection.library.clone(),
            catalog_source(&selection)?.into(),
            None,
        ),
        Command::ApplyAcquire { selection, approve } => (
            selection.library.clone(),
            catalog_source(&selection)?.into(),
            Some(approve),
        ),
        Command::PreviewAcquireGit(selection) => git_source(selection, None),
        Command::ApplyAcquireGit { selection, approve } => git_source(selection, Some(approve)),
    };
    let request = ChangeRequest::AcquireLibrarySkill {
        source,
        install_name: None,
    };
    let library = Library::open(state, &library)?;
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
            "skill acquisition did not complete"
        );
    } else {
        println!("{}", serde_json::to_string_pretty(&plan)?);
    }
    Ok(())
}

fn catalog_source(selection: &Selection) -> Result<BundleSource> {
    let snapshot = load_snapshot(&selection.catalog)?;
    // The caller explicitly selects the snapshot and registry here. The
    // fingerprint binds that choice and its selected artifact before retrieval.
    let index = CatalogIndex::build(
        vec![RegistryInput {
            id: selection.registry_id.clone(),
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
        .context("Agent Skill entry not found")?;
    Ok(BundleSource::from_selected(selected)?)
}

fn git_source(
    selection: GitSelection,
    approval: Option<String>,
) -> (PathBuf, AcquisitionSource, Option<String>) {
    let source = GitSource {
        artifact: selection.artifact,
        repo: selection.repo,
        path: selection.path,
        revision: selection.revision,
    };
    (selection.library, AcquisitionSource::Git(source), approval)
}
