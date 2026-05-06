use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use chrono::Utc;

pub fn copy_initial_sources(project_root: &Path, sources: &[PathBuf]) -> Result<Option<PathBuf>> {
    if sources.is_empty() {
        return Ok(None);
    }
    let base = project_root.join("raw/initial");
    let dest_dir = if sources.len() == 1 {
        base
    } else {
        base.join(Utc::now().format("%Y-%m-%dT%H%M%SZ").to_string())
    };
    fs::create_dir_all(&dest_dir)
        .with_context(|| format!("failed to create {}", dest_dir.display()))?;

    let mut manifest = String::from("# Initial Sources Manifest\n\n");
    manifest.push_str(&format!("- Copied At: {}\n", Utc::now().to_rfc3339()));
    manifest.push_str("- Sources:\n");
    for source in sources {
        if !source.exists() {
            bail!("initial source does not exist: {}", source.display());
        }
        let canonical = source
            .canonicalize()
            .with_context(|| format!("failed to resolve {}", source.display()))?;
        let file_name = source
            .file_name()
            .context("initial source path has no file name")?;
        let dest = dest_dir.join(file_name);
        if source.is_dir() {
            copy_dir(source, &dest)?;
        } else {
            fs::copy(source, &dest).with_context(|| {
                format!("failed to copy {} to {}", source.display(), dest.display())
            })?;
        }
        manifest.push_str(&format!(
            "  - `{}` -> `{}`\n",
            canonical.display(),
            dest.strip_prefix(project_root).unwrap_or(&dest).display()
        ));
    }
    fs::write(dest_dir.join("manifest.md"), manifest)
        .with_context(|| format!("failed to write {}", dest_dir.join("manifest.md").display()))?;
    Ok(Some(dest_dir))
}

fn copy_dir(source: &Path, dest: &Path) -> Result<()> {
    fs::create_dir_all(dest).with_context(|| format!("failed to create {}", dest.display()))?;
    for entry in
        fs::read_dir(source).with_context(|| format!("failed to read {}", source.display()))?
    {
        let entry = entry?;
        let entry_dest = dest.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir(&entry.path(), &entry_dest)?;
        } else {
            fs::copy(entry.path(), &entry_dest).with_context(|| {
                format!(
                    "failed to copy {} to {}",
                    entry.path().display(),
                    entry_dest.display()
                )
            })?;
        }
    }
    Ok(())
}
