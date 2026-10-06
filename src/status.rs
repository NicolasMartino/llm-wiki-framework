use std::fs;
use std::path::Path;

use anyhow::Result;

use crate::instance;
use crate::legacy_skills;
use crate::manifest::Manifest;
use crate::manifest::hash::sha256_hex;
use crate::mcp_config;
use crate::paths::Paths;

pub fn run(context: &crate::cli::CliContext) -> Result<()> {
    context.diagnostic("command: status");
    let paths = Paths::from_env()?;
    let manifest_path = paths.manifest();
    context.diagnostic(format!("managed home: {}", paths.managed_home().display()));
    context.diagnostic(format!(
        "managed binary: {}",
        paths.managed_binary().display()
    ));
    context.diagnostic(format!("manifest: {}", manifest_path.display()));

    // A corrupt manifest should be reported, not aborted on: render the parse
    // failure as status output and finish cleanly with the remaining checks.
    let manifest = match Manifest::read(&manifest_path) {
        Ok(manifest) => manifest,
        Err(error) => {
            context.diagnostic(format!("manifest parse failed: {error:#}"));
            println!("{} {}", instance::binary_stem(), env!("CARGO_PKG_VERSION"));
            println!("status: manifest.json failed to parse: {error:#}");
            println!("managed home: {}", paths.managed_home().display());
            print_legacy_skill_warnings(&paths);
            return Ok(());
        }
    };
    let Some(manifest) = manifest else {
        context.diagnostic("manifest state: missing");
        println!("{} {}", instance::binary_stem(), env!("CARGO_PKG_VERSION"));
        println!("status: not installed");
        println!("managed home: {}", paths.managed_home().display());
        print_legacy_skill_warnings(&paths);
        return Ok(());
    };

    context.diagnostic("manifest state: present");
    let file_count = manifest.skills.len() + manifest.assets.len();
    context.diagnostic(format!("manifest files: {file_count}"));

    println!("{} {}", instance::binary_stem(), env!("CARGO_PKG_VERSION"));
    println!("version: {}", manifest.binary.version);
    println!("installed at: {}", manifest.installed_at);
    println!("managed binary: {}", manifest.binary.path.display());
    if let Some(poman) = &manifest.poman {
        println!("managed poman: {}", poman.path.display());
    }
    println!(
        "mcp server startup: {} ({})",
        mcp_config::SERVER_STARTUP,
        mcp_config::server_start_command(&manifest.binary.path)
    );
    println!("manifest files: {file_count}");
    println!("files: {file_count}");

    for entry in &manifest.skills {
        print_file_status("skill", &entry.path, &entry.hash, context)?;
    }
    for asset in &manifest.assets {
        print_file_status("asset", &asset.path, &asset.hash, context)?;
    }

    print_legacy_skill_warnings(&paths);
    Ok(())
}

fn print_legacy_skill_warnings(paths: &Paths) {
    for dir in legacy_skills::existing_legacy_skill_dirs(paths) {
        println!(
            "Warning: legacy generated skill directory remains: {}",
            dir.display()
        );
    }
}

fn print_file_status(
    label: &str,
    path: &Path,
    expected_hash: &str,
    context: &crate::cli::CliContext,
) -> Result<()> {
    if !path.exists() {
        context.diagnostic(format!(
            "manifest file status: {} -> missing",
            path.display()
        ));
        println!("Missing {label}: {}", path.display());
        return Ok(());
    }

    let current = match fs::read(path) {
        Ok(bytes) => sha256_hex(&bytes),
        Err(error) => {
            context.diagnostic(format!(
                "manifest file status: {} -> unreadable",
                path.display()
            ));
            println!("Unreadable {label}: {} ({error})", path.display());
            return Ok(());
        }
    };
    if current == expected_hash {
        context.diagnostic(format!("manifest file status: {} -> ok", path.display()));
        println!("OK: {label}: {}", path.display());
    } else {
        context.diagnostic(format!(
            "manifest file status: {} -> drifted",
            path.display()
        ));
        println!("Drifted {label}: {}", path.display());
    }
    Ok(())
}
