use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

pub const DIAGNOSTIC_TARGET: &str = "llm_wiki_cli";

#[derive(Debug, Parser)]
#[command(name = "llm-wiki", version, about = "LLM Wiki framework tooling")]
pub struct Cli {
    #[arg(short = 'v', long = "verbose", global = true)]
    pub verbose: bool,
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CliContext {
    pub verbose: bool,
}

impl CliContext {
    pub fn new(verbose: bool) -> Self {
        Self { verbose }
    }

    pub fn diagnostic(&self, message: impl AsRef<str>) {
        if self.verbose {
            tracing::info!(target: DIAGNOSTIC_TARGET, "{}", message.as_ref());
        }
    }
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Build(BuildArgs),
    Install(InstallArgs),
    Init(InitArgs),
    Eval(EvalArgs),
    Register(RegisterArgs),
    Forget(ForgetArgs),
    Projects(ProjectsArgs),
    Index(IndexArgs),
    IndexAll(IndexAllArgs),
    Search(SearchArgs),
    SearchAll(SearchAllArgs),
    Path,
    Status,
    Doctor,
    Uninstall(UninstallArgs),
}

#[derive(Debug, clap::Args)]
pub struct EvalArgs {
    #[command(subcommand)]
    pub command: EvalCommand,
}

#[derive(Debug, Subcommand)]
pub enum EvalCommand {
    Run(EvalRunArgs),
    Calibrate(EvalCalibrateArgs),
}

#[derive(Clone, Debug, clap::Args)]
pub struct EvalRunArgs {
    #[arg(long, default_value = "wiki/evals/natural-language-search.eval.md")]
    pub eval_page: PathBuf,
    #[arg(long)]
    pub project: Option<String>,
    #[arg(long)]
    pub project_root: Option<PathBuf>,
    #[arg(long = "candidate-profile")]
    pub candidate_profiles: Vec<String>,
    #[arg(long)]
    pub embedding_model: Option<String>,
    #[arg(long)]
    pub query_expansion_model: Option<String>,
    #[arg(long)]
    pub reranker_model: Option<String>,
    #[arg(long)]
    pub candidate_name: Option<String>,
    #[arg(long)]
    pub output_dir: Option<PathBuf>,
    #[arg(long, default_value_t = 10)]
    pub limit: usize,
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub format: OutputFormat,
    #[arg(long)]
    pub rerank: bool,
    #[arg(long)]
    pub time_budget_warn_ms: Option<u64>,
}

#[derive(Debug, clap::Args)]
pub struct EvalCalibrateArgs {
    #[arg(long)]
    pub run_report: Option<PathBuf>,
    #[command(flatten)]
    pub run: EvalRunArgs,
    #[arg(long)]
    pub select_candidate: Option<String>,
    #[arg(long)]
    pub apply: bool,
    #[arg(long)]
    pub apply_profile: Option<String>,
    #[arg(long)]
    pub record: bool,
    #[arg(long)]
    pub export_raw_data: bool,
    #[arg(long)]
    pub raw_data_dir: Option<PathBuf>,
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
    #[arg(long)]
    pub configure_search: bool,
    #[arg(long)]
    pub disable_llm_search: bool,
}

#[derive(Debug, clap::Args)]
pub struct UninstallArgs {
    #[arg(long)]
    pub include_binary: bool,
    #[arg(long, conflicts_with = "include_binary")]
    pub search_artifacts: bool,
    #[arg(long, requires = "search_artifacts", conflicts_with = "include_binary")]
    pub force: bool,
}

#[derive(Debug, clap::Args)]
pub struct RegisterArgs {
    #[arg(long)]
    pub update: Option<String>,
    pub path: Option<PathBuf>,
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

#[derive(Debug, clap::Args)]
pub struct IndexArgs {
    #[arg(long)]
    pub project: Option<String>,
    #[arg(long)]
    pub force: bool,
}

#[derive(Debug, clap::Args)]
pub struct IndexAllArgs {
    #[arg(long)]
    pub force: bool,
}

#[derive(Debug, clap::Args)]
pub struct SearchArgs {
    pub query: String,
    #[arg(long, value_enum, default_value_t = SearchModeArg::Auto)]
    pub mode: SearchModeArg,
    #[arg(long)]
    pub project: Option<String>,
    #[arg(long = "class")]
    pub document_class: Option<String>,
    #[arg(long)]
    pub status: Option<String>,
    #[arg(long, default_value_t = 10)]
    pub limit: usize,
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub format: OutputFormat,
    #[arg(long)]
    pub allow_lexical_fallback: bool,
    #[arg(long)]
    pub rerank: bool,
}

#[derive(Debug, clap::Args)]
pub struct SearchAllArgs {
    pub query: String,
    #[arg(long, value_enum, default_value_t = SearchModeArg::Auto)]
    pub mode: SearchModeArg,
    #[arg(long)]
    pub include: Vec<String>,
    #[arg(long)]
    pub exclude: Vec<String>,
    #[arg(long = "class")]
    pub document_class: Option<String>,
    #[arg(long)]
    pub status: Option<String>,
    #[arg(long, default_value_t = 10)]
    pub limit: usize,
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub format: OutputFormat,
    #[arg(long)]
    pub allow_lexical_fallback: bool,
    #[arg(long)]
    pub rerank: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum SearchModeArg {
    Auto,
    Lexical,
    Semantic,
    Hybrid,
}

impl SearchModeArg {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Lexical => "lexical",
            Self::Semantic => "semantic",
            Self::Hybrid => "hybrid",
        }
    }
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
    #[arg(long)]
    pub blueprint: Option<String>,
    #[arg(long = "pack")]
    pub packs: Vec<String>,
    #[arg(long = "type", hide = true)]
    pub project_type: Option<String>,
    #[arg(long, hide = true)]
    pub scale: Option<String>,
    #[arg(long = "initial-sources")]
    pub initial_sources: Vec<PathBuf>,
}
