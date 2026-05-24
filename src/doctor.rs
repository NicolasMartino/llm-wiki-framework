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
use crate::search::semantic::{SemanticIndexMetadata, SemanticVectorIndex};
use crate::search_models::{AcceptedLicenses, ModelArtifacts, SearchThresholdStore};
use crate::search_profile::{ExternalDependencies, SearchConfig};
use crate::skill_render::{BINARY_MARKER, managed_binary_invocation};

pub fn run(context: &crate::cli::CliContext) -> Result<()> {
    context.diagnostic("command: doctor");
    let paths = Paths::from_env()?;
    context.diagnostic(format!("managed home: {}", paths.managed_home().display()));
    context.diagnostic(format!(
        "managed binary: {}",
        paths.managed_binary().display()
    ));
    context.diagnostic(format!("manifest: {}", paths.manifest().display()));
    context.diagnostic(format!(
        "partial marker: {}",
        paths.partial_install().display()
    ));
    let manifest = Manifest::read(&paths.manifest())?;
    context.diagnostic(format!(
        "manifest state: {}",
        if manifest.is_some() {
            "present"
        } else {
            "missing"
        }
    ));
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
            context.diagnostic(format!("PATH binary: {}", path_binary.display()));
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
                if contents.contains(BINARY_MARKER)
                    || (contents.contains("llm-wiki") && !contents.contains(&expected_binary))
                {
                    findings.push(format!(
                        "Installed skill does not use managed binary path: {} (run `llm-wiki install --force` to replace)",
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
    print_search_profile_diagnostics(&paths, context)?;
    print_project_search_diagnostics(&paths, context)?;
    Ok(())
}

fn print_search_profile_diagnostics(paths: &Paths, context: &crate::cli::CliContext) -> Result<()> {
    let search_config = paths.search_config();
    let external_dependencies = paths.external_dependencies();
    let accepted_licenses = paths.accepted_licenses();
    let model_artifacts = paths.model_artifacts();
    let search_thresholds = paths.search_thresholds();
    context.diagnostic(format!("search config: {}", search_config.display()));
    context.diagnostic(format!(
        "external dependencies: {}",
        external_dependencies.display()
    ));
    context.diagnostic(format!(
        "accepted licenses: {}",
        accepted_licenses.display()
    ));
    context.diagnostic(format!("model artifacts: {}", model_artifacts.display()));
    context.diagnostic(format!(
        "search thresholds: {}",
        search_thresholds.display()
    ));

    println!();
    println!("Search profile:");
    match SearchConfig::read(&search_config)? {
        Some(config) => {
            context.diagnostic(format!(
                "project default llm search: {}",
                config.project_default.llm_search_enabled
            ));
            context.diagnostic(format!(
                "global search llm search: {}",
                config.global_search.llm_search_enabled
            ));
            if config.project_default.llm_search_enabled || config.global_search.llm_search_enabled
            {
                println!("LLM search profile configured: {}", search_config.display());
            } else {
                println!("LLM search disabled: {}", search_config.display());
            }
        }
        None => {
            context.diagnostic("search config state: missing");
            println!(
                "LLM search profile missing: {} (run `llm-wiki install --configure-search`)",
                search_config.display()
            );
        }
    }

    match ExternalDependencies::read(&external_dependencies)? {
        Some(dependencies) => {
            context.diagnostic(format!(
                "external dependency records: {}",
                dependencies.dependencies.len()
            ));
            println!(
                "External dependency records: {} at {}",
                dependencies.dependencies.len(),
                external_dependencies.display()
            );
        }
        None => {
            context.diagnostic("external dependencies state: missing");
            println!(
                "External dependency records missing: {}",
                external_dependencies.display()
            );
        }
    }

    match AcceptedLicenses::read(&accepted_licenses)? {
        Some(licenses) => {
            context.diagnostic(format!(
                "accepted license records: {}",
                licenses.licenses.len()
            ));
            println!(
                "Accepted license records: {} at {}",
                licenses.licenses.len(),
                accepted_licenses.display()
            );
        }
        None => {
            context.diagnostic("accepted licenses state: missing");
            println!(
                "Accepted license records missing: {}",
                accepted_licenses.display()
            );
        }
    }

    match ModelArtifacts::read(&model_artifacts)? {
        Some(artifacts) => {
            context.diagnostic(format!(
                "model artifact records: {}",
                artifacts.artifacts.len()
            ));
            println!(
                "Model artifact records: {} at {}",
                artifacts.artifacts.len(),
                model_artifacts.display()
            );
        }
        None => {
            context.diagnostic("model artifacts state: missing");
            println!(
                "Model artifact records missing: {}",
                model_artifacts.display()
            );
        }
    }

    match SearchThresholdStore::read(&search_thresholds)? {
        Some(store) => {
            context.diagnostic("search thresholds state: configured");
            let first = store.thresholds().first();
            println!(
                "Search thresholds configured: records={} first_profile={} first_project={} at {}",
                store.thresholds().len(),
                first
                    .map(|thresholds| thresholds.profile.as_str())
                    .unwrap_or("<none>"),
                first
                    .and_then(|thresholds| thresholds.project_id.as_deref())
                    .unwrap_or("<unscoped>"),
                search_thresholds.display()
            );
        }
        None => {
            context.diagnostic("search thresholds state: missing");
            println!(
                "Search thresholds missing: {} (semantic/hybrid modes fail closed)",
                search_thresholds.display()
            );
        }
    }

    Ok(())
}

fn print_project_search_diagnostics(paths: &Paths, context: &crate::cli::CliContext) -> Result<()> {
    let registry_path = paths.project_registry();
    context.diagnostic(format!("registry: {}", registry_path.display()));
    context.diagnostic(format!("index root: {}", paths.index_root().display()));
    context.diagnostic(format!(
        "legacy index root: {}",
        paths.legacy_index_root().display()
    ));
    context.diagnostic(format!("model cache: {}", paths.model_cache().display()));
    context.diagnostic(format!(
        "managed index root: {}",
        paths.managed_index_root().display()
    ));
    context.diagnostic(format!(
        "managed model root: {}",
        paths.managed_model_root().display()
    ));
    let registry = ProjectRegistry::read(&registry_path)?;
    context.diagnostic(format!("registered projects: {}", registry.projects.len()));

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
        context.diagnostic("current project: <none>");
        println!("No current wiki project detected; project search checks skipped.");
        return Ok(());
    };
    context.diagnostic(format!(
        "current project: {}",
        discovered.project_root.display()
    ));

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

    let managed_store_path = registered
        .map(|project| paths.qmd_rs_store_path(&project.id))
        .unwrap_or_else(|| paths.qmd_rs_store_path(&discovered.project_key));
    let legacy_store_path = registered
        .map(|project| paths.legacy_qmd_rs_store_path(&project.id))
        .unwrap_or_else(|| paths.legacy_qmd_rs_store_path(&discovered.project_key));
    let using_legacy = !managed_store_path.exists() && legacy_store_path.exists();
    let store_path = if using_legacy {
        legacy_store_path.clone()
    } else {
        managed_store_path.clone()
    };
    let wiki_root = registered
        .map(RegisteredProject::wiki_root)
        .unwrap_or_else(|| discovered.wiki_root.clone());
    context.diagnostic(format!("wiki root: {}", wiki_root.display()));
    context.diagnostic(format!("index store: {}", store_path.display()));
    context.diagnostic(format!(
        "legacy index store: {}",
        legacy_store_path.display()
    ));
    let backend = QmdRsBackend::new();
    let status_project_id = registered
        .map(|project| project.id.as_str())
        .unwrap_or(discovered.project_key.as_str());
    let status = backend.doctor(status_project_id, &store_path, &wiki_root, SearchMode::Fts)?;
    context.diagnostic(format!(
        "search index status: {}, open_mode={}, indexed_files={}",
        backend_state_label(&status.state),
        status.open_mode.label(),
        status.indexed_files
    ));

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
        BackendState::Transient => {
            println!(
                "qmd-rs FTS index transient: {} (index publication is in progress; retry)",
                status.store_path.display()
            );
        }
        BackendState::PermissionDenied => {
            println!(
                "qmd-rs FTS index permission denied: {} (grant read access to the managed search cache)",
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
    }
    if legacy_store_path.exists() {
        println!(
            "Legacy qmd-rs cache present: {} (run `llm-wiki index` to migrate into {})",
            legacy_store_path.display(),
            paths.managed_index_root().display()
        );
    }

    let semantic_metadata_path = registered
        .map(|project| paths.semantic_index_metadata(&project.id))
        .unwrap_or_else(|| paths.semantic_index_metadata(&discovered.project_key));
    context.diagnostic(format!(
        "semantic index metadata: {}",
        semantic_metadata_path.display()
    ));
    match SemanticIndexMetadata::read(&semantic_metadata_path)? {
        Some(metadata) => println!(
            "Semantic index metadata: {} chunks at {}",
            metadata.chunks.len(),
            semantic_metadata_path.display()
        ),
        None => println!(
            "Semantic index metadata missing: {}",
            semantic_metadata_path.display()
        ),
    }
    let semantic_vector_path = registered
        .map(|project| paths.semantic_vector_index(&project.id))
        .unwrap_or_else(|| paths.semantic_vector_index(&discovered.project_key));
    context.diagnostic(format!(
        "semantic vector index: {}",
        semantic_vector_path.display()
    ));
    match SemanticVectorIndex::read(&semantic_vector_path)? {
        Some(index) => println!(
            "Semantic vector index: {} vectors at {}",
            index.vectors.len(),
            semantic_vector_path.display()
        ),
        None => println!(
            "Semantic vector index missing: {}",
            semantic_vector_path.display()
        ),
    }

    println!();
    println!("Semantic models:");
    println!(
        "Semantic search models are not checked until qmd-rs semantic mode is enabled; managed model root: {}; legacy model cache: {}",
        paths.managed_model_root().display(),
        paths.model_cache().display()
    );

    Ok(())
}

fn backend_state_label(state: &BackendState) -> &'static str {
    match state {
        BackendState::Ready => "ready",
        BackendState::Missing => "missing",
        BackendState::Stale => "stale",
        BackendState::Transient => "transient",
        BackendState::PermissionDenied => "permission-denied",
        BackendState::Corrupt => "corrupt",
        BackendState::SchemaMismatch => "schema-mismatch",
    }
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
