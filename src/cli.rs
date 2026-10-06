use std::ffi::OsString;
use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};

pub const DIAGNOSTIC_TARGET: &str = "llm_wiki_cli";

#[derive(Debug, Parser)]
#[command(
    name = env!("LLM_WIKI_COMPILED_BINARY_STEM"),
    version,
    about = "LLM Wiki framework tooling"
)]
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
    /// Install or refresh the managed binary, skills, and MCP config.
    Install(InstallArgs),
    /// Scaffold or update an LLM Wiki project in a directory.
    Init(InitArgs),
    /// Run Headroom with LLM Wiki MCP tools excluded from compression.
    Headroom(HeadroomArgs),
    /// Run MCP server subcommands (e.g. serve over stdio).
    Mcp(McpArgs),
    /// Read a wiki file with provenance-aware resolution.
    Read(ReadArgs),
    /// Run or calibrate natural-language search evaluations.
    Eval(EvalArgs),
    /// Register a project directory with the search registry.
    Register(RegisterArgs),
    /// Remove a project from the registry (optionally its cache).
    Forget(ForgetArgs),
    /// List registered projects and their index status.
    Projects(ProjectsArgs),
    /// Build or refresh the search index for one project.
    Index(IndexArgs),
    /// Build or refresh the search index for every project.
    IndexAll(IndexAllArgs),
    /// Search the current or a named project's wiki.
    Search(SearchArgs),
    /// Search across all registered projects.
    SearchAll(SearchAllArgs),
    /// Print the managed install and cache paths.
    Path,
    /// Show install status and managed-file integrity.
    Status,
    /// Diagnose the install, search profile, and indexes.
    Doctor(DoctorArgs),
    /// Remove the managed install (optionally the binary/caches).
    Uninstall(UninstallArgs),
}

#[derive(Debug, clap::Args)]
#[command(
    about = "Run the Headroom binary with llm-wiki MCP tools excluded from compression",
    long_about = "Export HEADROOM_EXCLUDE_TOOLS (*llm_wiki* plus all llm-wiki MCP tools in both namespaces) and HEADROOM_MCP_READ=off, then exec the Headroom binary with forwarded Headroom args untouched. Example: llm-wiki headroom -- wrap codex"
)]
pub struct HeadroomArgs {
    #[arg(
        long,
        value_name = "PATH",
        help = "Path to the headroom binary (default: search $PATH)"
    )]
    pub headroom_bin: Option<PathBuf>,
    #[arg(
        long,
        help = "Remove HEADROOM_MCP_READ instead of forcing it off",
        long_help = "Do not force HEADROOM_MCP_READ=off; remove it from the Headroom environment even if the parent set it. This is unsafe for wiki/raw provenance unless the session routes reads through llm-wiki MCP tools and avoids headroom_read."
    )]
    pub unsafe_mcp_read: bool,
    #[arg(
        required = true,
        trailing_var_arg = true,
        allow_hyphen_values = true,
        num_args = 1..,
        value_name = "HEADROOM_ARGS",
        help = "Arguments forwarded to the headroom binary (recommended form: `-- wrap codex`)"
    )]
    pub args: Vec<OsString>,
}

#[derive(Debug, clap::Args)]
pub struct McpArgs {
    #[command(subcommand)]
    pub command: McpCommand,
}

#[derive(Debug, Subcommand)]
pub enum McpCommand {
    Serve,
}

#[derive(Debug, clap::Args)]
pub struct ReadArgs {
    pub path: PathBuf,
    #[arg(long)]
    pub project: Option<String>,
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
#[command(group(
    clap::ArgGroup::new("search_posture")
        .args(["enable_llm_search", "disable_llm_search"])
))]
pub struct InstallArgs {
    #[arg(long)]
    pub force: bool,
    #[arg(long)]
    pub skip_path_guidance: bool,
    #[arg(long)]
    pub configure_search: bool,
    #[arg(long)]
    pub non_interactive: bool,
    #[arg(
        long,
        conflicts_with = "disable_llm_search",
        requires_all = ["non_interactive", "profile"]
    )]
    pub enable_llm_search: bool,
    #[arg(long, value_enum, requires = "enable_llm_search")]
    pub profile: Option<InstallSearchProfileArg>,
    #[arg(long, requires = "enable_llm_search")]
    pub confirm_model_downloads: bool,
    #[arg(long, requires = "enable_llm_search")]
    pub accept_profile_licenses: bool,
    #[arg(long)]
    pub disable_llm_search: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum InstallSearchProfileArg {
    Balanced,
}

impl InstallSearchProfileArg {
    pub const fn id(self) -> &'static str {
        match self {
            Self::Balanced => "balanced",
        }
    }
}

#[derive(Debug, clap::Args)]
pub struct UninstallArgs {
    /// Remove only the LLM search artifacts, leaving the rest installed.
    #[arg(long)]
    pub search_artifacts: bool,
    /// Only meaningful with `--search-artifacts`; permits removing absent or
    /// drifted search artifacts.
    #[arg(long)]
    pub force: bool,
}

#[derive(Args, Debug)]
pub struct DoctorArgs {}

#[derive(Debug, clap::Args)]
pub struct RegisterArgs {
    #[arg(long)]
    pub update: Option<String>,
    pub path: Option<PathBuf>,
    #[arg(long)]
    pub name: Option<String>,
    #[arg(long)]
    pub id: Option<String>,
    /// Skip wiring the project's host MCP config (`.mcp.json` / Codex).
    #[arg(long)]
    pub no_mcp: bool,
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
    #[arg(long)]
    pub compact: bool,
    #[arg(long)]
    pub page_size: Option<usize>,
    #[arg(long, default_value_t = 0)]
    pub offset: usize,
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
    #[arg(long)]
    pub compact: bool,
    #[arg(long)]
    pub page_size: Option<usize>,
    #[arg(long, default_value_t = 0)]
    pub offset: usize,
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
    /// Skip wiring the project's host MCP config (`.mcp.json` / Codex).
    #[arg(long)]
    pub no_mcp: bool,
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
