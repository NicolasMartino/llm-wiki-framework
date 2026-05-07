use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::embed;
use crate::manifest::Manifest;
use crate::manifest::hash::sha256_hex;
use crate::paths::{Paths, managed_binary_name};
use crate::registry::{ProjectRegistry, RegisteredProject};
use crate::search::adapter::{BackendState, SearchBackend, SearchMode};
use crate::search::project::discover_from_cwd;
use crate::search::qmd_rs::QmdRsBackend;
use crate::skill_render::managed_binary_invocation;

pub fn run() -> Result<()> {
    let paths = Paths::from_env()?;
    let manifest = Manifest::read(&paths.manifest())?;
    let manifest_paths: HashSet<PathBuf> = manifest
        .as_ref()
        .map(|manifest| {
            manifest
                .skills
                .iter()
                .map(|entry| entry.path.clone())
                .collect()
        })
        .unwrap_or_default();

    let mut findings = Vec::new();
    if let Some(manifest) = &manifest {
        if !manifest.binary.path.exists() {
            findings.push(format!(
                "Missing managed binary: {}",
                manifest.binary.path.display()
            ));
        } else {
            let current = sha256_hex(&fs::read(&manifest.binary.path)?);
            if current != manifest.binary.hash {
                findings.push(format!(
                    "Drifted managed binary: {} (run `llm-wiki install` to refresh)",
                    manifest.binary.path.display()
                ));
            }
        }
        if let Some(path_binary) = find_on_path(managed_binary_name())
            && !same_path(&path_binary, &manifest.binary.path)
            && path_binary.exists()
            && manifest.binary.path.exists()
        {
            let path_hash = sha256_hex(&fs::read(&path_binary)?);
            if path_hash != manifest.binary.hash {
                findings.push(format!(
                    "PATH llm-wiki differs from managed binary: {} (rerun `llm-wiki install` after upgrading)",
                    path_binary.display()
                ));
            }
        }
        let expected_binary = managed_binary_invocation(&manifest.binary.path);
        for entry in &manifest.skills {
            if !entry.path.exists() {
                findings.push(format!("Missing manifest file: {}", entry.path.display()));
            } else {
                let bytes = fs::read(&entry.path)?;
                let current = sha256_hex(&bytes);
                if current != entry.hash {
                    findings.push(format!(
                        "Drifted manifest file: {} (run `llm-wiki install --force` to replace)",
                        entry.path.display()
                    ));
                }
                let contents = String::from_utf8_lossy(&bytes);
                if contents.contains("{llm_wiki_binary}") || contents.contains("`llm-wiki init ") {
                    findings.push(format!(
                        "Installed skill does not use managed binary path: {} (run `llm-wiki install --force` to replace)",
                        entry.path.display()
                    ));
                } else if contents.contains(" init <path>") && !contents.contains(&expected_binary)
                {
                    findings.push(format!(
                        "Installed skill init command does not target managed binary: {}",
                        entry.path.display()
                    ));
                }
            }
        }
    }

    let partial = paths.partial_install();
    if partial.exists() {
        findings.push(format!(
            "Stale partial install marker: {}",
            partial.display()
        ));
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

    println!("Install:");
    if findings.is_empty() {
        println!("No llm-wiki install issues found.");
    } else {
        for finding in findings {
            println!("{finding}");
        }
    }
    print_project_search_diagnostics(&paths)?;
    Ok(())
}

fn print_project_search_diagnostics(paths: &Paths) -> Result<()> {
    let registry_path = paths.project_registry();
    let registry = ProjectRegistry::read(&registry_path)?;

    println!();
    println!("Registry:");
    if registry_path.exists() {
        println!(
            "Project registry: {} ({} projects)",
            registry_path.display(),
            registry.projects.len()
        );
    } else {
        println!("Project registry missing: {}", registry_path.display());
    }
    let missing_roots = registry
        .projects
        .iter()
        .filter(|project| !project.root.exists())
        .collect::<Vec<_>>();
    if !missing_roots.is_empty() {
        for project in missing_roots {
            println!(
                "Registered project root missing: {} ({})",
                project.id,
                project.root.display()
            );
        }
    }

    println!();
    println!("Current project:");
    let Some(discovered) = discover_from_cwd()? else {
        println!("No current wiki project detected; project search checks skipped.");
        return Ok(());
    };

    println!(
        "Detected wiki project: {}",
        discovered.project_root.display()
    );
    let registered = registry.project_by_root(&discovered.project_root);
    if let Some(project) = registered {
        println!("Registered project ID: {}", project.id);
    } else {
        println!(
            "Current project is not registered; run `llm-wiki register {}`",
            discovered.project_root.display()
        );
    }

    let store_path = registered
        .map(|project| paths.qmd_rs_store_path(&project.id))
        .unwrap_or_else(|| paths.qmd_rs_store_path(&discovered.project_key));
    let wiki_root = registered
        .map(RegisteredProject::wiki_root)
        .unwrap_or(discovered.wiki_root);
    let backend = QmdRsBackend::new();
    let status = backend.doctor(&store_path, &wiki_root, SearchMode::Fts)?;

    println!();
    println!("Search index:");
    match status.state {
        BackendState::Ready => {
            println!(
                "qmd-rs FTS index ready: {} files at {}",
                status.indexed_files,
                status.store_path.display()
            );
        }
        BackendState::Stale => {
            println!(
                "qmd-rs FTS index stale: {} (rebuild the search index)",
                status.store_path.display()
            );
        }
        BackendState::Missing => {
            println!(
                "qmd-rs FTS index missing: {} (build the search index)",
                status.store_path.display()
            );
        }
        BackendState::Corrupt => {
            println!(
                "qmd-rs FTS index corrupt: {} (rebuild the search index)",
                status.store_path.display()
            );
        }
        BackendState::SchemaMismatch => {
            println!(
                "qmd-rs FTS index schema mismatch: {} (rebuild the search index)",
                status.store_path.display()
            );
        }
        BackendState::FeatureDisabled => {
            println!(
                "{}",
                status
                    .message
                    .as_deref()
                    .unwrap_or("qmd-rs backend feature is disabled")
            );
        }
    }

    println!();
    println!("Semantic models:");
    println!(
        "Semantic search models are not checked until qmd-rs semantic mode is enabled; model cache: {}",
        paths.model_cache().display()
    );

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
    let marker = std::env::var("LLM_WIKI_LEGACY_SYMLINK_MARKER")
        .unwrap_or_else(|_| "software_project_management".to_string());
    Ok(target.to_string_lossy().contains(&marker))
}

fn find_on_path(binary_name: &str) -> Option<PathBuf> {
    let path = env::var_os("PATH")?;
    for dir in env::split_paths(&path) {
        let candidate = dir.join(binary_name);
        if candidate.is_file() {
            return Some(candidate);
        }
        #[cfg(windows)]
        {
            let pathext = env::var_os("PATHEXT")
                .map(|value| value.to_string_lossy().to_string())
                .unwrap_or_else(|| ".EXE;.BAT;.CMD".to_string());
            for ext in pathext.split(';') {
                let ext = ext.trim();
                let ext = ext.strip_prefix('.').unwrap_or(ext);
                let candidate = dir.join(format!("{binary_name}.{ext}"));
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }
    }
    None
}

fn same_path(left: &Path, right: &Path) -> bool {
    match (fs::canonicalize(left), fs::canonicalize(right)) {
        (Ok(left), Ok(right)) => left == right,
        _ => left == right,
    }
}
