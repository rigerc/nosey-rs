//! CLI entry points for shared change plans.

use anyhow::{Result, ensure};
use clap::Subcommand;
use nosey_core::{
    State,
    library::Library,
    plan::{ChangeOutcome, ChangePlan, ChangeRequest, load_requests},
};
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

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
    },
    /// Apply exactly the reviewed plan identified by its fingerprint.
    ApplyPlan {
        library: PathBuf,
        requests: PathBuf,
        /// Restrict the plan to these change IDs (repeatable).
        #[arg(long)]
        only: Vec<String>,
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
        } => {
            let library = Library::open(state, &library)?;
            let requests = select(&library, state, &requests, &only)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&library.plan(state, &requests)?)?
            );
        }
        Command::ApplyPlan {
            library,
            requests,
            only,
            approve,
        } => {
            let library = Library::open(state, &library)?;
            let requests = select(&library, state, &requests, &only)?;
            let plan = library.plan(state, &requests)?;
            review_or_apply(&library, state, &plan, Some(&approve))?;
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
    let Some(approve) = approve else {
        println!("{}", serde_json::to_string_pretty(plan)?);
        return Ok(());
    };
    ensure!(
        plan.fingerprint == approve,
        "stale approval; review the plan again"
    );
    let results = library.apply_plan(state, plan)?;
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
