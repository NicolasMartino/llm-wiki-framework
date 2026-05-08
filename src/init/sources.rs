use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use chrono::Utc;

pub fn validate_initial_sources(sources: &[PathBuf]) -> Result<()> {
    for source in sources {
        if !source.exists() {
            bail!("initial source does not exist: {}", source.display());
        }
        source
            .canonicalize()
            .with_context(|| format!("failed to resolve {}", source.display()))?;
        source
            .file_name()
            .context("initial source path has no file name")?;
    }
    Ok(())
}

pub fn copy_initial_sources(project_root: &Path, sources: &[PathBuf]) -> Result<Option<PathBuf>> {
    if sources.is_empty() {
        return Ok(None);
    }
    validate_initial_sources(sources)?;

    let copied_at = Utc::now();
    let base = project_root.join("raw/initial");
    let dest_dir = base.join(copied_at.format("%Y-%m-%dT%H%M%SZ").to_string());
    let sources_dir = dest_dir.join("sources");
    fs::create_dir_all(&sources_dir)
        .with_context(|| format!("failed to create {}", dest_dir.display()))?;

    let mut manifest = String::from("# Initial Sources Manifest\n\n");
    manifest.push_str(&format!("- Copied At: {}\n", copied_at.to_rfc3339()));
    manifest.push_str("- Sources:\n");
    for source in sources {
        let canonical = source
            .canonicalize()
            .with_context(|| format!("failed to resolve {}", source.display()))?;
        let file_name = source
            .file_name()
            .context("initial source path has no file name")?;
        let dest = unique_dest(&sources_dir.join(file_name));
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

fn unique_dest(path: &Path) -> PathBuf {
    if !path.exists() {
        return path.to_path_buf();
    }

    let parent = path.parent().unwrap_or_else(|| Path::new(""));
    let stem = path
        .file_stem()
        .map(|value| value.to_string_lossy())
        .unwrap_or_default();
    let extension = path.extension().map(|value| value.to_string_lossy());

    for attempt in 2.. {
        let file_name = match &extension {
            Some(extension) => format!("{stem}-{attempt}.{extension}"),
            None => format!("{stem}-{attempt}"),
        };
        let candidate = parent.join(file_name);
        if !candidate.exists() {
            return candidate;
        }
    }
    unreachable!("unbounded retry loop always returns")
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
