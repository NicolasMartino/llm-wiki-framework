mod backup_policy;
mod cli;
mod doctor;
mod eval;
mod headroom;
mod init;
mod install;
mod instance;
mod legacy_skills;
mod manifest;
mod mcp;
mod mcp_config;
mod mcp_wiring;
mod path_guidance;
mod paths;
mod payload_integrity;
mod registry;
mod search;
mod search_models;
mod search_profile;
mod status;
#[cfg(test)]
mod test_env;
mod uninstall;
mod wiki_read;

use std::env;
use std::ffi::OsStr;
use std::io::IsTerminal;

use anyhow::{Context, Result, anyhow};
use clap::{CommandFactory, FromArgMatches};
use cli::{Cli, CliContext, Command, DIAGNOSTIC_TARGET};
use tracing_subscriber::EnvFilter;

fn main() -> Result<()> {
    let cli = parse_cli();
    init_tracing(cli.verbose)?;
    search::gguf_runtime::set_verbose_stderr(cli.verbose);
    let context = CliContext::new(cli.verbose);
    match &cli.command {
        Command::Install(args) => install::run(args, &context),
        Command::Init(args) => init::run(args, &context),
        Command::Headroom(args) => headroom::run(args, &context),
        Command::Mcp(args) => mcp::run(args, &context),
        Command::Read(args) => wiki_read::run_cli(args, &context),
        Command::Eval(args) => eval::run(args, &context),
        Command::Register(args) => registry::register(args, &context),
        Command::Forget(args) => registry::forget(args, &context),
        Command::Projects(args) => registry::projects(args, &context),
        Command::Index(args) => search::commands::index(args, &context),
        Command::IndexAll(args) => search::commands::index_all(args, &context),
        Command::Search(args) => search::commands::search(args, &context),
        Command::SearchAll(args) => search::commands::search_all(args, &context),
        Command::Path => path_guidance::run(&context),
        Command::Status => status::run(&context),
        Command::Doctor(args) => doctor::run(args, &context),
        Command::Uninstall(args) => uninstall::run(args, &context),
    }
}

fn parse_cli() -> Cli {
    let matches = Cli::command()
        .name(instance::binary_stem())
        .bin_name(instance::binary_stem())
        .get_matches();
    Cli::from_arg_matches(&matches).unwrap_or_else(|err| err.exit())
}

fn init_tracing(verbose: bool) -> Result<()> {
    if !verbose && env::var_os("RUST_LOG").is_none() {
        return Ok(());
    }

    let filter = tracing_filter(verbose)?;
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .with_ansi(use_ansi_stderr())
        .without_time()
        .with_level(false)
        .with_target(false)
        .try_init()
        .map_err(|error| anyhow!("initialize tracing subscriber: {error}"))
}

fn tracing_filter(verbose: bool) -> Result<EnvFilter> {
    if env::var_os("RUST_LOG").is_some() {
        let filter = EnvFilter::try_from_default_env().context("parse RUST_LOG")?;
        if verbose {
            return Ok(filter.add_directive(format!("{DIAGNOSTIC_TARGET}=info").parse()?));
        }
        return Ok(filter);
    }

    Ok(EnvFilter::new(format!("{DIAGNOSTIC_TARGET}=info")))
}

fn use_ansi_stderr() -> bool {
    if env::var_os("NO_COLOR").is_some() {
        return false;
    }
    if let Some(value) = env::var_os("CLICOLOR_FORCE") {
        return value != OsStr::new("0");
    }
    std::io::stderr().is_terminal()
}
