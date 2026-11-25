//! Development-only headless slice; production connectivity/trust orchestration remains a gate.
#![forbid(unsafe_code)]
use anyhow::{Context,Result};
use clap::{Parser,Subcommand};
use nosey_core::State;
use nosey_registry::{CatalogIndex,RegistryInput,load_snapshot};
use nosey_scanner::Settings;
use std::{collections::{BTreeMap,BTreeSet},path::PathBuf,sync::atomic::AtomicBool};
#[derive(Parser)]
#[command(about="Nosey Phase 3 development slice (local synthetic catalogs only)")]
struct Args{#[arg(long)]state:PathBuf,#[command(subcommand)]command:Command}
#[derive(Subcommand)]
enum Command{Register{root:PathBuf},List,ScanFixture{root:PathBuf,#[arg(long)]catalog:PathBuf,#[arg(long)]registry_id:String,#[arg(long)]extension:String,#[arg(long)]approve_fixture:bool}}
fn main()->Result<()>{let args=Args::parse();let mut state=State::load(&args.state)?;match args.command{Command::Register{root}=>{let root=state.register(&root)?;state.save(&args.state)?;println!("{}",root.display());},Command::List=>{for p in state.projects.keys(){println!("{}",p.display());}},Command::ScanFixture{root,catalog,registry_id,extension,approve_fixture}=>{anyhow::ensure!(approve_fixture,"explicit --approve-fixture required");let snapshot=load_snapshot(&catalog)?;let index=CatalogIndex::build(vec![RegistryInput{id:registry_id,priority:0,approved:true,enabled:true,catalog:snapshot.catalog}],&extension)?;let root=root.canonicalize()?;let scan=state.scan(&root,&index,&Settings::default(),&AtomicBool::new(false))?.clone();let project=state.projects.get(&root).context("registered project disappeared")?;let evaluation=nosey_recommendation::evaluate(&index,&scan,&project.dismissed,&BTreeSet::new(),&BTreeMap::new());state.save(&args.state)?;println!("{}",serde_json::to_string_pretty(&(scan,evaluation))?);}}Ok(())}
