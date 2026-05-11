mod build;
mod cli;
mod doctor;
mod embed;
mod init;
mod install;
mod manifest;
mod path_guidance;
mod paths;
mod registry;
mod search;
mod search_profile;
mod skill_render;
mod status;
mod uninstall;

use std::env;
use std::ffi::OsStr;
use std::io::IsTerminal;

use anyhow::{Context, Result, anyhow};
use clap::Parser;
use cli::{Cli, CliContext, Command, DIAGNOSTIC_TARGET};
use tracing_subscriber::EnvFilter;

fn main() -> Result<()> {
    let cli = Cli::parse();
    init_tracing(cli.verbose)?;
    let context = CliContext::new(cli.verbose);
    match &cli.command {
        Command::Build(args) => build::run(args, &context),
        Command::Install(args) => install::run(args, &context),
        Command::Init(args) => init::run(args, &context),
        Command::Register(args) => registry::register(args, &context),
        Command::Forget(args) => registry::forget(args, &context),
        Command::Projects(args) => registry::projects(args, &context),
        Command::Index(args) => search::commands::index(args, &context),
        Command::IndexAll(args) => search::commands::index_all(args, &context),
        Command::Search(args) => search::commands::search(args, &context),
        Command::SearchAll(args) => search::commands::search_all(args, &context),
        Command::Path => path_guidance::run(&context),
        Command::Status => status::run(&context),
        Command::Doctor => doctor::run(&context),
        Command::Uninstall(args) => uninstall::run(args.include_binary, &context),
    }
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
