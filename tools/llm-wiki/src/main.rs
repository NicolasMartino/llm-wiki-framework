mod build;
mod cli;
mod embed;
mod install;
mod manifest;
mod paths;
mod uninstall;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Command};

fn main() -> Result<()> {
    let _ = embed::PROJECT_GUIDELINES_TEMPLATE.len() + embed::CLAUDE_TEMPLATE.len();
    let cli = Cli::parse();
    match &cli.command {
        Command::Build(args) => build::run(args),
        Command::Install(args) => install::run(args.force),
        Command::Uninstall => uninstall::run(),
    }
}
