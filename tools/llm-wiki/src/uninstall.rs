use std::fs;

use anyhow::{Context, Result};

use crate::manifest::Manifest;
use crate::paths::Paths;

pub fn run() -> Result<()> {
    let paths = Paths::from_env()?;
    let manifest_path = paths.manifest();
    let Some(manifest) = Manifest::read(&manifest_path)? else {
        return Ok(());
    };
    for entry in manifest.files.iter().rev() {
        if entry.path.exists() {
            fs::remove_file(&entry.path)
                .with_context(|| format!("failed to remove {}", entry.path.display()))?;
        }
    }
    if manifest_path.exists() {
        fs::remove_file(&manifest_path)
            .with_context(|| format!("failed to remove {}", manifest_path.display()))?;
    }
    Ok(())
}
