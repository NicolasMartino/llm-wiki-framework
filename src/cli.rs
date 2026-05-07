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
    Register(RegisterArgs),
    Forget(ForgetArgs),
    Projects(ProjectsArgs),
    Path,
    Status,
    Doctor,
    Uninstall(UninstallArgs),
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
    #[arg(long)]
    pub skip_path_guidance: bool,
}

#[derive(Debug, clap::Args)]
pub struct UninstallArgs {
    #[arg(long)]
    pub include_binary: bool,
}

#[derive(Debug, clap::Args)]
pub struct RegisterArgs {
    #[arg(long)]
    pub update: Option<String>,
    pub path: PathBuf,
    #[arg(long)]
    pub name: Option<String>,
    #[arg(long)]
    pub id: Option<String>,
}

#[derive(Debug, clap::Args)]
pub struct ForgetArgs {
    pub project_id: String,
    #[arg(long)]
    pub delete_cache: bool,
}

#[derive(Debug, clap::Args)]
pub struct ProjectsArgs {
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub format: OutputFormat,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum OutputFormat {
    Text,
    Json,
}

#[derive(Debug, clap::Args)]
pub struct InitArgs {
    pub path: PathBuf,
    #[arg(long)]
    pub no_register: bool,
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
