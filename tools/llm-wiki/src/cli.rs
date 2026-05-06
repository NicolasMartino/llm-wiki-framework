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
