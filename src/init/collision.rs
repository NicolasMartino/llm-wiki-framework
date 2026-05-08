use std::path::{Path, PathBuf};

use anyhow::{Result, bail};

pub fn refuse_framework_collision(path: &Path) -> Result<()> {
    let artifacts: Vec<PathBuf> = [
        "wiki",
        "raw",
        ".llm_wiki",
        "AGENTS.md",
        "CLAUDE.md",
        "project_guidelines.md",
    ]
    .into_iter()
    .map(|artifact| path.join(artifact))
    .filter(|artifact| artifact.exists())
    .collect();
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
