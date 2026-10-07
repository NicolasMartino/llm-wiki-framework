use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use crate::cli::{CliContext, UninstallArgs};
use crate::instance;
use crate::manifest::Manifest;
use crate::manifest::hash::sha256_hex;
use crate::mcp_config;
use crate::paths::Paths;
use crate::search_profile::SearchConfig;

pub fn run(args: &UninstallArgs, context: &CliContext) -> Result<()> {
    if args.search_artifacts {
        return cleanup_search_artifacts(args.force, context);
    }
    if args.force {
        bail!(
            "`--force` only applies to `{} uninstall --search-artifacts`",
            instance::binary_stem()
        );
    }
    full_uninstall(context)
}

/// Removes everything install wrote, both binaries included (the owner,
/// 2026-10-06: "uninstall uninstalls all").
fn full_uninstall(context: &CliContext) -> Result<()> {
    context.diagnostic("command: uninstall");
    let paths = Paths::from_env()?;
    let manifest_path = paths.manifest();
    context.diagnostic(format!("manifest: {}", manifest_path.display()));
    let manifest = Manifest::read(&manifest_path)?;
    let Some(manifest) = manifest.as_ref() else {
        context.diagnostic("manifest state: missing");
        // The Codex/Claude MCP configs are materialized independently of the
        // manifest, so clean them up even when the manifest is gone — otherwise
        // the host keeps trying to spawn the now-deleted managed binary.
        cleanup_codex_mcp_config(&paths, context)?;
        cleanup_staged_claude_mcp_config(&paths, context)?;
        remove_global_runtime_state(&paths, None, context)?;
        return Ok(());
    };
    context.diagnostic("manifest state: present");
    context.diagnostic(format!(
        "managed binary: {}",
        manifest.binary.path.display()
    ));

    for entry in &manifest.skills {
        validate_owned_file(&entry.path, &entry.hash, context)?;
    }
    for asset in &manifest.assets {
        validate_owned_file(&asset.path, &asset.hash, context)?;
    }

    cleanup_codex_mcp_config(&paths, context)?;

    let mut managed_dirs = BTreeSet::new();
    for entry in &manifest.skills {
        if let Some(parent) = entry.path.parent() {
            managed_dirs.insert(parent.to_path_buf());
        }
    }
    for asset in &manifest.assets {
        if let Some(parent) = asset.path.parent() {
            managed_dirs.insert(parent.to_path_buf());
        }
    }

    for entry in manifest.skills.iter().rev() {
        remove_owned_file(&entry.path, context)?;
    }
    for asset in manifest.assets.iter().rev() {
        remove_owned_file(&asset.path, context)?;
    }

    remove_empty_skill_dirs(managed_dirs, context)?;
    if manifest_path.exists() {
        fs::remove_file(&manifest_path)
            .with_context(|| format!("failed to remove {}", manifest_path.display()))?;
    }
    remove_global_runtime_state(&paths, Some(manifest), context)?;
    Ok(())
}

fn validate_owned_file(path: &Path, expected_hash: &str, context: &CliContext) -> Result<()> {
    context.diagnostic(format!("consider file: {}", path.display()));
    if path.exists() {
        let current = sha256_hex(&fs::read(path)?);
        if current != expected_hash {
            context.diagnostic(format!("drift check: {} -> drifted", path.display()));
            bail!(
                "refusing to uninstall drifted file {}; run install --force first or remove it manually",
                path.display()
            );
        }
        context.diagnostic(format!("drift check: {} -> ok", path.display()));
    } else {
        context.diagnostic(format!("drift check: {} -> missing", path.display()));
    }
    Ok(())
}

fn cleanup_codex_mcp_config(paths: &Paths, context: &CliContext) -> Result<()> {
    let codex_path = paths.codex_config_toml();

    // The install-time recovery snapshot must outlive the edit below: if the
    // rewrite fails, the snapshot is the user's only way back. Compute its path
    // now but remove it only after the config has been safely rewritten (or
    // confirmed to need no change).
    let mut backup = codex_path.clone().into_os_string();
    backup.push(".llm-wiki-backup");
    let backup_path = PathBuf::from(backup);

    let existing = match fs::read_to_string(&codex_path) {
        Ok(existing) => existing,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            remove_file_if_exists(&backup_path, context)?;
            return Ok(());
        }
        Err(error) => {
            return Err(error).with_context(|| format!("failed to read {}", codex_path.display()));
        }
    };

    let names = [instance::mcp_server_name(), mcp_config::poman_server_name()];
    let Some(updated) = mcp_config::remove_codex_servers(&existing, &names)? else {
        remove_file_if_exists(&backup_path, context)?;
        return Ok(());
    };

    if updated.trim().is_empty() {
        fs::remove_file(&codex_path)
            .with_context(|| format!("failed to remove {}", codex_path.display()))?;
        context.diagnostic(format!(
            "removed Codex MCP config: {}",
            codex_path.display()
        ));
        if let Some(parent) = codex_path.parent() {
            remove_empty_dir(parent, context)?;
        }
    } else {
        // Re-validate + write atomically so a failed edit leaves the original
        // config and its backup snapshot untouched.
        mcp_config::write_codex_config_atomic(&codex_path, &updated)?;
        context.diagnostic(format!(
            "updated Codex MCP config: {}",
            codex_path.display()
        ));
    }

    // The edit landed; the recovery snapshot is now safe to drop.
    remove_file_if_exists(&backup_path, context)?;
    Ok(())
}

fn cleanup_staged_claude_mcp_config(paths: &Paths, context: &CliContext) -> Result<()> {
    let path = paths.claude_project_mcp_config();
    if path.exists() {
        context.diagnostic(format!(
            "remove staged Claude MCP config: {}",
            path.display()
        ));
        fs::remove_file(&path).with_context(|| format!("failed to remove {}", path.display()))?;
    }
    if let Some(parent) = path.parent() {
        remove_empty_dir(parent, context)?;
    }
    Ok(())
}

fn remove_owned_file(path: &Path, context: &CliContext) -> Result<()> {
    if path.exists() {
        context.diagnostic(format!("remove file: {}", path.display()));
        fs::remove_file(path).with_context(|| format!("failed to remove {}", path.display()))?;
    }
    Ok(())
}

fn remove_empty_skill_dirs(dirs: BTreeSet<PathBuf>, context: &CliContext) -> Result<()> {
    let mut dirs: Vec<_> = dirs.into_iter().collect();
    dirs.sort_by(|a, b| {
        b.components()
            .count()
            .cmp(&a.components().count())
            .then_with(|| b.cmp(a))
    });

    for dir in dirs {
        match fs::remove_dir(&dir) {
            Ok(()) => context.diagnostic(format!("remove empty dir: {}", dir.display())),
            Err(err)
                if matches!(
                    err.kind(),
                    std::io::ErrorKind::NotFound | std::io::ErrorKind::DirectoryNotEmpty
                ) => {}
            Err(err) => {
                return Err(err)
                    .with_context(|| format!("failed to remove empty dir {}", dir.display()));
            }
        }
    }

    Ok(())
}

fn cleanup_search_artifacts(force: bool, context: &CliContext) -> Result<()> {
    context.diagnostic("command: uninstall --search-artifacts");
    context.diagnostic(format!("force: {force}"));
    let paths = Paths::from_env()?;
    context.diagnostic(format!(
        "search config: {}",
        paths.search_config().display()
    ));
    if llm_search_enabled(&paths)? && !force {
        bail!(
            "refusing to remove search artifacts while LLM search is enabled; disable LLM search first or rerun `{} uninstall --search-artifacts --force`",
            instance::binary_stem()
        );
    }

    remove_search_artifacts(&paths, context)?;
    println!("Removed LLM search artifacts.");
    Ok(())
}

fn remove_global_runtime_state(
    paths: &Paths,
    manifest: Option<&Manifest>,
    context: &CliContext,
) -> Result<()> {
    remove_file_if_exists(&paths.partial_install(), context)?;
    remove_dir_all_if_exists(&paths.managed_home().join("backups"), context)?;
    remove_file_if_exists(&paths.search_config(), context)?;
    remove_file_if_exists(&paths.external_dependencies(), context)?;
    remove_search_artifacts(paths, context)?;
    remove_dir_all_if_exists(&paths.managed_index_root(), context)?;
    remove_file_if_exists(&paths.project_registry(), context)?;
    remove_dir_all_if_exists(&paths.cache_home(), context)?;

    let (binary, poman) = match manifest {
        Some(manifest) => (
            manifest.binary.path.clone(),
            manifest.poman.as_ref().map(|poman| poman.path.clone()),
        ),
        // Without a manifest, the managed bin folder still holds only what
        // install put there.
        None => (paths.managed_binary(), Some(paths.managed_poman())),
    };
    remove_file_if_exists(&binary, context)?;
    if let Some(poman) = poman {
        remove_file_if_exists(&poman, context)?;
    }
    sweep_leaked_binary_temps(&paths.managed_bin_dir(), context);
    remove_empty_dir(&paths.managed_bin_dir(), context)?;
    remove_empty_dir(&paths.managed_home(), context)?;
    remove_empty_dir(&paths.data_home(), context)?;
    Ok(())
}

/// Best-effort removal of leaked atomic-replace temp files (`.llm-wiki-bin-*`)
/// so the managed bin dir can be pruned. A crash mid-install can orphan one and
/// otherwise leave the directory non-empty and unremovable.
fn sweep_leaked_binary_temps(bin_dir: &Path, context: &CliContext) {
    let Ok(entries) = fs::read_dir(bin_dir) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        if name.to_string_lossy().starts_with(".llm-wiki-bin-") {
            context.diagnostic(format!(
                "remove leaked install temp: {}",
                entry.path().display()
            ));
            let _ = fs::remove_file(entry.path());
        }
    }
}

fn remove_search_artifacts(paths: &Paths, context: &CliContext) -> Result<()> {
    remove_dir_all_if_exists(&paths.managed_model_root(), context)?;
    remove_file_if_exists(&paths.accepted_licenses(), context)?;
    remove_file_if_exists(&paths.search_thresholds(), context)?;
    remove_file_if_exists(&paths.search_runtime_probes(), context)?;
    let removed_sidecars = remove_semantic_sidecars(&paths.managed_index_root(), context)?;
    context.diagnostic(format!("semantic sidecars removed: {removed_sidecars}"));
    Ok(())
}

fn llm_search_enabled(paths: &Paths) -> Result<bool> {
    let Some(config) = SearchConfig::read(&paths.search_config())? else {
        return Ok(false);
    };
    Ok(config.project_default.llm_search_enabled || config.global_search.llm_search_enabled)
}

fn remove_semantic_sidecars(root: &Path, context: &CliContext) -> Result<usize> {
    let root_type = match fs::symlink_metadata(root) {
        Ok(metadata) => metadata.file_type(),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(0),
        Err(error) => {
            return Err(error).with_context(|| format!("failed to inspect {}", root.display()));
        }
    };
    if !root_type.is_dir() {
        return Ok(0);
    }
    let mut removed = 0;
    for entry in fs::read_dir(root).with_context(|| format!("failed to read {}", root.display()))? {
        let entry = entry.with_context(|| format!("failed to read {}", root.display()))?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .with_context(|| format!("failed to inspect {}", path.display()))?;
        if file_type.is_dir() {
            removed += remove_semantic_sidecars(&path, context)?;
            remove_empty_dir(&path, context)?;
        } else if is_semantic_sidecar(&path) {
            remove_file_entry(&path, context)?;
            removed += 1;
        }
    }
    Ok(removed)
}

fn is_semantic_sidecar(path: &Path) -> bool {
    matches!(
        path.file_name().and_then(|name| name.to_str()),
        Some("semantic-index.json" | "semantic-vectors.json")
    )
}

fn remove_file_if_exists(path: &Path, context: &CliContext) -> Result<()> {
    if path.exists() {
        context.diagnostic(format!("remove file: {}", path.display()));
        fs::remove_file(path).with_context(|| format!("failed to remove {}", path.display()))?;
    }
    Ok(())
}

fn remove_file_entry(path: &Path, context: &CliContext) -> Result<()> {
    context.diagnostic(format!("remove file: {}", path.display()));
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error).with_context(|| format!("failed to remove {}", path.display())),
    }
}

fn remove_dir_all_if_exists(path: &Path, context: &CliContext) -> Result<()> {
    if path.exists() {
        context.diagnostic(format!("remove dir: {}", path.display()));
        fs::remove_dir_all(path).with_context(|| format!("failed to remove {}", path.display()))?;
    }
    Ok(())
}

fn remove_empty_dir(path: &Path, context: &CliContext) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }
    match fs::remove_dir(path) {
        Ok(()) => {
            context.diagnostic(format!("remove empty dir: {}", path.display()));
            Ok(())
        }
        Err(error) if error.kind() == std::io::ErrorKind::DirectoryNotEmpty => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error).with_context(|| format!("failed to remove {}", path.display())),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::remove_semantic_sidecars;
    use crate::cli::CliContext;

    #[cfg(unix)]
    #[test]
    fn semantic_sidecar_cleanup_skips_symlinked_index_directories() {
        use std::os::unix::fs::symlink;

        let managed = TempDir::new().expect("managed");
        let outside = TempDir::new().expect("outside");
        let root = managed.path().join("indexes");
        let real_index = root.join("project");
        let outside_index = outside.path().join("external-index");

        fs::create_dir_all(&real_index).expect("real index");
        fs::create_dir_all(&outside_index).expect("outside index");
        fs::write(real_index.join("semantic-index.json"), "{}").expect("real semantic");
        fs::write(real_index.join("qmd-rs.sqlite"), "lexical").expect("lexical");
        fs::write(outside_index.join("semantic-index.json"), "{}").expect("outside semantic");
        fs::write(outside_index.join("semantic-vectors.json"), "{}").expect("outside vectors");
        symlink(&outside_index, root.join("linked-index")).expect("symlinked index");

        let removed =
            remove_semantic_sidecars(&root, &CliContext::new(false)).expect("cleanup sidecars");

        assert_eq!(removed, 1);
        assert!(!real_index.join("semantic-index.json").exists());
        assert!(real_index.join("qmd-rs.sqlite").exists());
        assert!(root.join("linked-index").exists());
        assert!(outside_index.join("semantic-index.json").exists());
        assert!(outside_index.join("semantic-vectors.json").exists());
    }
}
