//! Development-only headless slice; production connectivity/trust orchestration remains a gate.
#![forbid(unsafe_code)]

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use nosey_core::{State, skill_discovery::inspect_target};
#[cfg(target_os = "linux")]
use nosey_core::{
    changes::{InstallPreview, InstallReason},
    library::Library,
};
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
    /// Stage an explicitly selected local fixture skill (not a trusted registry entry).
    #[cfg(target_os = "linux")]
    ImportFixture {
        library: PathBuf,
        id: String,
        source: PathBuf,
        #[arg(long)]
        approve_fixture: bool,
    },
    /// Preview a link from a local fixture into an existing target.
    #[cfg(target_os = "linux")]
    PreviewInstall {
        library: PathBuf,
        root: PathBuf,
        target: PathBuf,
        id: String,
    },
    /// Apply only the exact previously displayed fingerprint.
    #[cfg(target_os = "linux")]
    ApplyInstall {
        library: PathBuf,
        root: PathBuf,
        target: PathBuf,
        id: String,
        #[arg(long)]
        approve: String,
    },
    /// Audit managed, pending and detached link records without changing them.
    #[cfg(target_os = "linux")]
    AuditLinks { library: PathBuf },
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
        #[cfg(target_os = "linux")]
        Command::ImportFixture {
            library,
            id,
            source,
            approve_fixture,
        } => {
            anyhow::ensure!(approve_fixture, "explicit --approve-fixture required");
            let library = Library::open(&state, &library)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&library.import_fixture(&id, &source)?)?
            );
        }
        #[cfg(target_os = "linux")]
        Command::PreviewInstall {
            library,
            root,
            target,
            id,
        } => {
            let library = Library::open(&state, &library)?;
            let preview = library.preview(&state, &root, &target, &id, InstallReason::Manual)?;
            println!("{}", serde_json::to_string_pretty(&preview)?);
        }
        #[cfg(target_os = "linux")]
        Command::ApplyInstall {
            library,
            root,
            target,
            id,
            approve,
        } => {
            let library = Library::open(&state, &library)?;
            let preview: InstallPreview =
                library.preview(&state, &root, &target, &id, InstallReason::Manual)?;
            anyhow::ensure!(
                preview.fingerprint == approve,
                "stale approval; review preview again"
            );
            println!(
                "{}",
                serde_json::to_string_pretty(&library.apply(&state, &preview)?)?
            );
        }
        #[cfg(target_os = "linux")]
        Command::AuditLinks { library } => {
            let library = Library::open(&state, &library)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&library.audit_links(&state)?)?
            );
        }
        Command::ScanFixture {
            root,
            catalog,
            registry_id,
            extension,
            approve_fixture,
            mcp,
        } => {
            anyhow::ensure!(approve_fixture, "explicit --approve-fixture required");
            scan_fixture(
                &mut state,
                &args.state,
                &root,
                &catalog,
                registry_id,
                &extension,
                mcp,
            )?;
        }
    }
    Ok(())
}

fn scan_fixture(
    state: &mut State,
    state_path: &std::path::Path,
    root: &std::path::Path,
    catalog: &std::path::Path,
    registry_id: String,
    extension: &str,
    mcp: bool,
) -> Result<()> {
    let snapshot = load_snapshot(catalog)?;
    let index = CatalogIndex::build(
        vec![RegistryInput {
            id: registry_id,
            priority: 0,
            approved: true,
            enabled: true,
            catalog: snapshot.catalog,
        }],
        extension,
    )?;
    let root = root.canonicalize()?;
    let settings = Settings {
        mcp,
        ..Settings::default()
    };
    let attempt = state
        .scan(&root, &index, &settings, &AtomicBool::new(false))
        .cloned();
    state.save(state_path)?;
    let scan = attempt?;
    let project = state
        .projects
        .get(&root)
        .context("registered project disappeared")?;
    // Explicit synthetic fixture mode is not a production online session.
    let evaluation = project.reevaluate(
        &index,
        &settings,
        true,
        &BTreeSet::new(),
        &BTreeMap::new(),
        &BTreeSet::new(),
    )?;
    println!("{}", serde_json::to_string_pretty(&(scan, evaluation))?);
    Ok(())
}
