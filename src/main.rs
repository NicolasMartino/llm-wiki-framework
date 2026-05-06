mod build;
mod cli;
mod doctor;
mod embed;
mod init;
mod install;
mod manifest;
mod path_guidance;
mod paths;
mod skill_render;
mod status;
mod uninstall;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Command};

fn main() -> Result<()> {
    let cli = Cli::parse();
    match &cli.command {
        Command::Build(args) => build::run(args),
        Command::Install(args) => install::run(args.force, !args.skip_path_guidance),
        Command::Init(args) => init::run(args),
        Command::Path => path_guidance::run(),
        Command::Status => status::run(),
        Command::Doctor => doctor::run(),
        Command::Uninstall(args) => uninstall::run(args.include_binary),
    }
}
