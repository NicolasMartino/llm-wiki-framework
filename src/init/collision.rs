use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use anyhow::{Result, bail};

/// `names` is the folder's listing, spelled as on disk.
pub fn refuse_framework_collision(path: &Path, names: &BTreeSet<String>) -> Result<()> {
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
    // `exists()` finds any spelling on a case-insensitive file system and
    // only the exact one elsewhere; the listing names the file as spelled.
    if path.join("AGENTS.md").exists() || names.contains("AGENTS.MD") {
        let spelled = names
            .iter()
            .find(|name| name.eq_ignore_ascii_case("AGENTS.md"))
            .map_or("AGENTS.md", String::as_str);
        artifacts.push(path.join(spelled));
    }
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
