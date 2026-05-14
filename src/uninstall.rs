use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};

use crate::cli::{CliContext, UninstallArgs};
use crate::manifest::Manifest;
use crate::manifest::hash::sha256_hex;
use crate::paths::Paths;
use crate::search_profile::SearchConfig;

pub fn run(args: &UninstallArgs, context: &CliContext) -> Result<()> {
    if args.search_artifacts {
        return cleanup_search_artifacts(args.force, context);
    }
    full_uninstall(args.include_binary, context)
}

fn full_uninstall(include_binary: bool, context: &CliContext) -> Result<()> {
    context.diagnostic("command: uninstall");
    context.diagnostic(format!("include managed binary: {include_binary}"));
    let paths = Paths::from_env()?;
    let manifest_path = paths.manifest();
    context.diagnostic(format!("manifest: {}", manifest_path.display()));
    let manifest = Manifest::read(&manifest_path)?;
    let Some(manifest) = manifest.as_ref() else {
        context.diagnostic("manifest state: missing");
        remove_global_runtime_state(&paths, include_binary, None, context)?;
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
    remove_global_runtime_state(&paths, include_binary, Some(&manifest.binary.path), context)?;
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
            "refusing to remove search artifacts while LLM search is enabled; disable LLM search first or rerun `llm-wiki uninstall --search-artifacts --force`"
        );
    }

    remove_search_artifacts(&paths, context)?;
    println!("Removed LLM search artifacts.");
    Ok(())
}

fn remove_global_runtime_state(
    paths: &Paths,
    include_binary: bool,
    manifest_binary: Option<&Path>,
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

    if include_binary {
        let binary = manifest_binary
            .map(Path::to_path_buf)
            .unwrap_or_else(|| paths.managed_binary());
        remove_file_if_exists(&binary, context)?;
        remove_empty_dir(&paths.managed_bin_dir(), context)?;
    }
    remove_empty_dir(&paths.managed_home(), context)?;
    remove_empty_dir(&paths.data_home(), context)?;
    Ok(())
}

fn remove_search_artifacts(paths: &Paths, context: &CliContext) -> Result<()> {
    remove_dir_all_if_exists(&paths.managed_model_root(), context)?;
    remove_file_if_exists(&paths.accepted_licenses(), context)?;
    remove_file_if_exists(&paths.search_thresholds(), context)?;
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
