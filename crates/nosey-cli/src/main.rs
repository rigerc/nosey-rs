//! Development-only headless slice; production connectivity/trust orchestration remains a gate.
#![forbid(unsafe_code)]

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use nosey_core::{State, skill_discovery::inspect_target};
use nosey_registry::{CatalogIndex, RegistryInput, load_snapshot};
use nosey_scanner::Settings;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
    sync::atomic::AtomicBool,
};

#[derive(Parser)]
#[command(about = "Nosey development slice (local fixtures only)")]
struct Args {
    #[arg(long)]
    state: PathBuf,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Explicitly register a project.
    Register { root: PathBuf },
    /// Forget project state; never change installed links.
    Unregister { root: PathBuf },
    /// List registered projects.
    List,
    /// Inspect direct skill candidates in an explicit project-relative target.
    InspectTarget {
        root: PathBuf,
        #[arg(long)]
        target: PathBuf,
    },
    /// Run the synthetic local-registry development flow.
    ScanFixture {
        root: PathBuf,
        #[arg(long)]
        catalog: PathBuf,
        #[arg(long)]
        registry_id: String,
        #[arg(long)]
        extension: String,
        #[arg(long)]
        approve_fixture: bool,
        /// Enable passive MCP manifest inspection for the fixture.
        #[arg(long)]
        mcp: bool,
    },
}

fn main() -> Result<()> {
    let args = Args::parse();
    let mut state = State::load(&args.state)?;
    match args.command {
        Command::Register { root } => {
            let root = state.register(&root)?;
            state.save(&args.state)?;
            println!("{}", root.display());
        }
        Command::Unregister { root } => {
            let root = state.unregister(&root)?;
            state.save(&args.state)?;
            println!("{}", root.display());
        }
        Command::List => {
            for path in state.project_paths() {
                println!("{}", path.display());
            }
        }
        Command::InspectTarget { root, target } => {
            let audit = inspect_target(&state, &root, &target)?;
            println!("{}", serde_json::to_string_pretty(&audit)?);
        }
        Command::ScanFixture { root, catalog, registry_id, extension, approve_fixture, mcp } => {
            anyhow::ensure!(approve_fixture, "explicit --approve-fixture required");
            let snapshot = load_snapshot(&catalog)?;
            let index = CatalogIndex::build(
                vec![RegistryInput {
                    id: registry_id, priority: 0, approved: true,
                    enabled: true, catalog: snapshot.catalog,
                }],
                &extension,
            )?;
            let root = root.canonicalize()?;
            let settings = Settings { mcp, ..Settings::default() };
            let attempt = state.scan(&root, &index, &settings, &AtomicBool::new(false)).map(Clone::clone);
            state.save(&args.state)?;
            let scan = attempt?;
            let project = state.projects.get(&root).context("registered project disappeared")?;
            // Explicit synthetic fixture mode is not a production online session.
            let evaluation = project.reevaluate(&index, &settings, true,
                &BTreeSet::new(), &BTreeMap::new(), &BTreeSet::new())?;
            println!("{}", serde_json::to_string_pretty(&(scan, evaluation))?);
        }
    }
    Ok(())
}
