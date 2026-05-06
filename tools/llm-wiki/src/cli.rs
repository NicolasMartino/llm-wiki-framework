use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
#[command(name = "llm-wiki", version, about = "LLM Wiki framework tooling")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Build(BuildArgs),
    Install(InstallArgs),
    Init(InitArgs),
    Uninstall,
}

#[derive(Debug, clap::Args)]
pub struct BuildArgs {
    #[arg(long, value_enum, default_value_t = BuildTarget::Both)]
    pub target: BuildTarget,
    #[arg(long, default_value = "./build")]
    pub out: PathBuf,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum BuildTarget {
    Claude,
    Codex,
    Both,
}

#[derive(Debug, clap::Args)]
pub struct InstallArgs {
    #[arg(long)]
    pub force: bool,
}

#[derive(Debug, clap::Args)]
pub struct InitArgs {
    pub path: PathBuf,
    #[arg(long)]
    pub non_interactive: bool,
    #[arg(long)]
    pub name: Option<String>,
    #[arg(long)]
    pub description: Option<String>,
    #[arg(long = "type")]
    pub project_type: Option<String>,
    #[arg(long)]
    pub scale: Option<String>,
    #[arg(long)]
    pub existing: bool,
    #[arg(long = "initial-sources")]
    pub initial_sources: Vec<PathBuf>,
}
