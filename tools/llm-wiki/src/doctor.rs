use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::embed;
use crate::manifest::Manifest;
use crate::manifest::hash::sha256_hex;
use crate::paths::Paths;

pub fn run() -> Result<()> {
    let paths = Paths::from_env()?;
    let manifest = Manifest::read(&paths.manifest())?;
    let manifest_paths: HashSet<PathBuf> = manifest
        .as_ref()
        .map(|manifest| {
            manifest
                .files
                .iter()
                .map(|entry| entry.path.clone())
                .collect()
        })
        .unwrap_or_default();

    let mut findings = Vec::new();
    if let Some(manifest) = &manifest {
        for entry in &manifest.files {
            if !entry.path.exists() {
                findings.push(format!("Missing manifest file: {}", entry.path.display()));
            } else {
                let current = sha256_hex(&fs::read(&entry.path)?);
                if current != entry.sha256 {
                    findings.push(format!(
                        "Drifted manifest file: {} (run `llm-wiki install --force` to replace)",
                        entry.path.display()
                    ));
                }
            }
        }
    }

    for asset in embed::SKILLS {
        for path in [
            paths.claude_skill(asset.name),
            paths.codex_skill(asset.name),
            paths.codex_config(asset.name),
        ] {
            if is_legacy_symlink(&path)? {
                findings.push(format!(
                    "Legacy symlink residue: {} (remove it or run `llm-wiki install --force`)",
                    path.display()
                ));
            } else if path.exists() && !manifest_paths.contains(&path) {
                findings.push(format!(
                    "Unknown framework-shaped file: {} (run `llm-wiki install --force` to own it)",
                    path.display()
                ));
            }
        }
    }

    if findings.is_empty() {
        println!("No llm-wiki install issues found.");
    } else {
        for finding in findings {
            println!("{finding}");
        }
    }
    Ok(())
}

fn is_legacy_symlink(path: &Path) -> Result<bool> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(_) => return Ok(false),
    };
    if !metadata.file_type().is_symlink() {
        return Ok(false);
    }
    let target = fs::read_link(path)?;
    Ok(target
        .to_string_lossy()
        .contains("software_project_management"))
}
