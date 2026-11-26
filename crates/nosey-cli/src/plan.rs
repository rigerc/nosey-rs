//! CLI entry points for shared change plans.

use anyhow::{Result, ensure};
use clap::{Args, Subcommand};
use nosey_core::{
    State,
    library::Library,
    plan::{ChangeOutcome, ChangePlan, ChangeRequest, load_requests},
};
use nosey_registry::{CatalogIndex, RegistryInput, load_snapshot};
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

#[derive(Args)]
pub struct CatalogSelection {
    /// Review advisory metadata from this local catalog snapshot.
    #[arg(long, requires = "registry_id")]
    catalog: Option<PathBuf>,
    /// Identity of the selected registry.
    #[arg(long, requires = "catalog")]
    registry_id: Option<String>,
}

impl CatalogSelection {
    fn load(self) -> Result<Option<CatalogIndex>> {
        let (Some(path), Some(id)) = (self.catalog, self.registry_id) else {
            return Ok(None);
        };
        Ok(Some(CatalogIndex::build(
            vec![RegistryInput {
                id,
                priority: 0,
                approved: true,
                enabled: true,
                catalog: load_snapshot(&path)?.catalog,
            }],
            "",
        )?))
    }
}

#[derive(Subcommand)]
/// Change plan subcommands flattened into the main CLI.
pub enum Command {
    /// Review a batch of changes from a JSON request file without applying them.
    Plan {
        library: PathBuf,
        requests: PathBuf,
        /// Restrict the plan to these change IDs (repeatable).
        #[arg(long)]
        only: Vec<String>,
        #[command(flatten)]
        catalog: CatalogSelection,
    },
    /// Apply exactly the reviewed plan identified by its fingerprint.
    ApplyPlan {
        library: PathBuf,
        requests: PathBuf,
        /// Restrict the plan to these change IDs (repeatable).
        #[arg(long)]
        only: Vec<String>,
        #[command(flatten)]
        catalog: CatalogSelection,
        #[arg(long)]
        approve: String,
    },
}

fn select(
    library: &Library,
    state: &State,
    requests: &Path,
    only: &[String],
) -> Result<Vec<ChangeRequest>> {
    let requests = load_requests(requests)?;
    if only.is_empty() {
        return Ok(requests);
    }
    let wanted: BTreeSet<&str> = only.iter().map(String::as_str).collect();
    let plan = library.plan(state, &requests)?;
    let selected: Vec<_> = plan
        .changes
        .into_iter()
        .filter(|c| wanted.contains(c.id.as_str()))
        .map(|c| c.request)
        .collect();
    ensure!(
        selected.len() == wanted.len(),
        "unknown change ID in --only"
    );
    Ok(selected)
}

/// Run a change plan command against the loaded project state.
pub fn run(command: Command, state: &State) -> Result<()> {
    match command {
        Command::Plan {
            library,
            requests,
            only,
            catalog,
        } => {
            let library = Library::open(state, &library)?;
            let requests = select(&library, state, &requests, &only)?;
            let catalog = catalog.load()?;
            println!(
                "{}",
                serde_json::to_string_pretty(&library.plan_with_catalog(
                    state,
                    &requests,
                    catalog.as_ref()
                )?)?
            );
        }
        Command::ApplyPlan {
            library,
            requests,
            only,
            catalog,
            approve,
        } => {
            let library = Library::open(state, &library)?;
            let requests = select(&library, state, &requests, &only)?;
            let catalog = catalog.load()?;
            let plan = library.plan_with_catalog(state, &requests, catalog.as_ref())?;
            review_or_apply_with_catalog(&library, state, &plan, Some(&approve), catalog.as_ref())?;
        }
    }
    Ok(())
}

/// Print a plan, or verify its approval and print the results.
pub fn review_or_apply(
    library: &Library,
    state: &State,
    plan: &ChangePlan,
    approve: Option<&str>,
) -> Result<()> {
    review_or_apply_with_catalog(library, state, plan, approve, None)
}

fn review_or_apply_with_catalog(
    library: &Library,
    state: &State,
    plan: &ChangePlan,
    approve: Option<&str>,
    catalog: Option<&CatalogIndex>,
) -> Result<()> {
    let Some(approve) = approve else {
        println!("{}", serde_json::to_string_pretty(plan)?);
        return Ok(());
    };
    ensure!(
        plan.fingerprint == approve,
        "stale approval; review the plan again"
    );
    let results = library.apply_plan_with_catalog(state, plan, catalog)?;
    println!("{}", serde_json::to_string_pretty(&results)?);
    let unresolved = results
        .iter()
        .filter(|result| {
            matches!(
                result.outcome,
                ChangeOutcome::Stale | ChangeOutcome::Failed { .. }
            )
        })
        .count();
    ensure!(
        unresolved == 0,
        "{unresolved} change(s) stale or failed; plan again to retry"
    );
    Ok(())
}
