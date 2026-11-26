//! Standalone headless compiler for the restricted development authoring subset.
#![forbid(unsafe_code)]
use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
#[derive(Parser)]
struct Args {
    #[command(subcommand)]
    command: Command,
}
#[derive(Subcommand)]
enum Command {
    Validate {
        #[arg(long)]
        registry: PathBuf,
    },
    Compile {
        #[arg(long)]
        registry: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
}
fn main() -> Result<()> {
    match Args::parse().command {
        Command::Validate { registry } => {
            nosey_registry::compile(&registry)?;
            println!("SDK validation passed (not trust verification)");
        }
        Command::Compile { registry, output } => {
            let bytes = nosey_registry::compile(&registry)?;
            nosey_registry::save_snapshot(&output, &bytes)?;
        }
    }
    Ok(())
}
