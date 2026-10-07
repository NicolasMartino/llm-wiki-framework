use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Result, bail};

pub fn refuse_framework_collision(path: &Path) -> Result<()> {
    let mut artifacts: Vec<PathBuf> = [
        "wiki",
        "raw",
        ".llm_wiki",
        "CLAUDE.md",
        "project_guidelines.md",
    ]
    .into_iter()
    .map(|artifact| path.join(artifact))
    .filter(|artifact| artifact.exists())
    .collect();
    // Matched by name in the folder listing: a case-insensitive file system
    // says both spellings exist when either does.
    let names: BTreeSet<String> = fs::read_dir(path)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    artifacts.extend(
        ["AGENTS.md", "AGENTS.MD"]
            .into_iter()
            .filter(|name| names.contains(*name))
            .map(|name| path.join(name)),
    );
    if !artifacts.is_empty() {
        let listed = artifacts
            .iter()
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>()
            .join(", ");
        bail!("refusing to initialize over existing framework artifacts: {listed}");
    }
    Ok(())
}
