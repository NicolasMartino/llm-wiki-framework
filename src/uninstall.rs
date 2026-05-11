use std::fs;

use anyhow::{Context, Result};

use crate::manifest::Manifest;
use crate::manifest::hash::sha256_hex;
use crate::paths::Paths;

pub fn run(include_binary: bool, context: &crate::cli::CliContext) -> Result<()> {
    context.diagnostic("command: uninstall");
    context.diagnostic(format!("include managed binary: {include_binary}"));
    let paths = Paths::from_env()?;
    let manifest_path = paths.manifest();
    context.diagnostic(format!("manifest: {}", manifest_path.display()));
    let Some(manifest) = Manifest::read(&manifest_path)? else {
        context.diagnostic("manifest state: missing");
        return Ok(());
    };
    context.diagnostic("manifest state: present");
    context.diagnostic(format!(
        "managed binary: {}",
        manifest.binary.path.display()
    ));

    for entry in &manifest.skills {
        context.diagnostic(format!("consider file: {}", entry.path.display()));
        if entry.path.exists() {
            let current = sha256_hex(&fs::read(&entry.path)?);
            if current != entry.hash {
                context.diagnostic(format!("drift check: {} -> drifted", entry.path.display()));
                anyhow::bail!(
                    "refusing to uninstall drifted file {}; restore it or move it aside first",
                    entry.path.display()
                );
            }
            context.diagnostic(format!("drift check: {} -> ok", entry.path.display()));
        } else {
            context.diagnostic(format!("drift check: {} -> missing", entry.path.display()));
        }
    }

    for entry in manifest.skills.iter().rev() {
        if entry.path.exists() {
            context.diagnostic(format!("remove file: {}", entry.path.display()));
            fs::remove_file(&entry.path)
                .with_context(|| format!("failed to remove {}", entry.path.display()))?;
        }
    }
    if manifest_path.exists() {
        context.diagnostic(format!("remove manifest: {}", manifest_path.display()));
        fs::remove_file(&manifest_path)
            .with_context(|| format!("failed to remove {}", manifest_path.display()))?;
    }
    if include_binary && manifest.binary.path.exists() {
        context.diagnostic(format!(
            "remove managed binary: {}",
            manifest.binary.path.display()
        ));
        fs::remove_file(&manifest.binary.path)
            .with_context(|| format!("failed to remove {}", manifest.binary.path.display()))?;
    }
    Ok(())
}
