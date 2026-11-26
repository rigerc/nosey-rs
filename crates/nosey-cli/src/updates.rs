//! Explicit snapshot-based review and update policy commands.

use anyhow::{Context, Result, ensure};
use clap::{Args, Subcommand, ValueEnum};
use nosey_core::{
    State, UpdatePolicy,
    library::{BundleSource, Library},
    plan::{ChangeOutcome, ChangeRequest},
};
use nosey_registry::{CatalogIndex, RegistryInput, SafeTransport, load_snapshot};
use std::path::{Path, PathBuf};

#[derive(Args)]
pub struct Snapshots {
    library: PathBuf,
    catalog: PathBuf,
    #[arg(long)]
    registry_id: String,
    /// Additional snapshots in descending priority, as REGISTRY=PATH.
    #[arg(long, value_name = "REGISTRY=PATH")]
    additional_catalog: Vec<String>,
}

impl Snapshots {
    fn index(&self) -> Result<CatalogIndex> {
        let mut inputs = Vec::new();
        let first = (self.registry_id.as_str(), self.catalog.as_path());
        inputs.push(read_input(first, 0)?);
        for (offset, spec) in self.additional_catalog.iter().enumerate() {
            let (registry, path) = spec.split_once('=').context("expected REGISTRY=PATH")?;
            ensure!(
                !registry.is_empty() && inputs.iter().all(|input| input.id != registry),
                "registry IDs must be nonempty and unique"
            );
            inputs.push(read_input(
                (registry, Path::new(path)),
                u32::try_from(offset + 1)?,
            )?);
        }
        Ok(CatalogIndex::build(inputs, "")?)
    }
}

fn read_input((id, path): (&str, &Path), priority: u32) -> Result<RegistryInput> {
    Ok(RegistryInput {
        id: id.into(),
        priority,
        approved: true,
        enabled: true,
        catalog: load_snapshot(path)?.catalog,
    })
}

#[derive(Clone, Copy, ValueEnum)]
pub enum Policy {
    Manual,
    Notify,
    Automatic,
}

#[derive(Subcommand)]
pub enum Command {
    /// Report available catalog updates and apply eligible updates under automatic policy.
    CheckUpdates(Snapshots),
    /// Review the selected remote bundle replacement.
    PreviewRemoteUpdate {
        #[command(flatten)]
        input: Snapshots,
        artifact: String,
    },
    /// Apply the exact reviewed remote bundle replacement.
    ApplyRemoteUpdate {
        #[command(flatten)]
        input: Snapshots,
        artifact: String,
        #[arg(long)]
        approve: String,
    },
    /// Set the global update policy for shared library content.
    SetUpdatePolicy { policy: Policy },
    /// Report current source availability from the supplied snapshots.
    RefreshAvailability(Snapshots),
}

pub fn run(command: Command, state: &mut State, state_path: &Path) -> Result<()> {
    match command {
        Command::SetUpdatePolicy { policy } => {
            state.update_policy = match policy {
                Policy::Manual => UpdatePolicy::Manual,
                Policy::Notify => UpdatePolicy::Notify,
                Policy::Automatic => UpdatePolicy::Automatic,
            };
            state.save(state_path)?;
            if state.update_policy == UpdatePolicy::Automatic {
                eprintln!(
                    "Automatic updates replace the shared copy used by every existing project and global link."
                );
            }
            print_json(&state.update_policy)
        }
        Command::RefreshAvailability(input) => {
            let index = input.index()?;
            let library = Library::open_initialized(state, &input.library)?;
            print_json(&library.refresh_availability(&index)?)
        }
        Command::CheckUpdates(input) => {
            let index = input.index()?;
            let library = Library::open_initialized(state, &input.library)?;
            let updates = library.available_updates(&index)?;
            let outcomes = if state.update_policy == UpdatePolicy::Automatic && !updates.is_empty()
            {
                library.apply_automatic_updates(state, &index, &SafeTransport::new()?)?
            } else {
                Vec::new()
            };
            print_json(&serde_json::json!({
                "policy": state.update_policy,
                "updates": updates,
                "outcomes": outcomes,
            }))
        }
        Command::PreviewRemoteUpdate { input, artifact } => review(state, &input, &artifact, None),
        Command::ApplyRemoteUpdate {
            input,
            artifact,
            approve,
        } => review(state, &input, &artifact, Some(&approve)),
    }
}

fn review(state: &State, input: &Snapshots, artifact: &str, approval: Option<&str>) -> Result<()> {
    let index = input.index()?;
    let selected = index
        .skills
        .get(artifact)
        .context("selected skill is missing from the catalog")?;
    let source = BundleSource::from_selected(selected)?;
    let library = Library::open_initialized(state, &input.library)?;
    let plan = library.plan(state, &[ChangeRequest::UpdateRemoteLibrarySkill { source }])?;
    let Some(approval) = approval else {
        return print_json(&plan);
    };
    ensure!(
        plan.fingerprint == approval,
        "stale approval; review the remote update again"
    );
    let outcomes = library.apply_plan(state, &plan)?;
    print_json(&outcomes)?;
    ensure!(
        outcomes
            .iter()
            .all(|item| item.outcome == ChangeOutcome::Applied),
        "remote update did not complete"
    );
    Ok(())
}

fn print_json(value: &impl serde::Serialize) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}
