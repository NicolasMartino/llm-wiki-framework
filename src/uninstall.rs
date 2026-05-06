use std::fs;

use anyhow::{Context, Result};

use crate::manifest::Manifest;
use crate::manifest::hash::sha256_hex;
use crate::paths::Paths;

pub fn run() -> Result<()> {
    let paths = Paths::from_env()?;
    let manifest_path = paths.manifest();
    let Some(manifest) = Manifest::read(&manifest_path)? else {
        return Ok(());
    };

    for entry in &manifest.skills {
        if entry.path.exists() {
            let current = sha256_hex(&fs::read(&entry.path)?);
            if current != entry.hash {
                anyhow::bail!(
                    "refusing to uninstall drifted file {}; restore it or move it aside first",
                    entry.path.display()
                );
            }
        }
    }

    for entry in manifest.skills.iter().rev() {
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
