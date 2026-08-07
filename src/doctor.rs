use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::instance;
use crate::legacy_skills;
use crate::manifest::Manifest;
use crate::manifest::hash::sha256_hex;
use crate::mcp_config;
use crate::mcp_config::ServerWiring;
use crate::paths::{Paths, managed_binary_name};
use crate::registry::{ProjectRegistry, RegisteredProject};
use crate::search::adapter::{BackendState, SearchBackend, SearchMode};
use crate::search::project::discover_from_cwd;
use crate::search::qmd_rs::QmdRsBackend;
use crate::search::runtime_probe::{
    self, RuntimeProbeOutcome, RuntimeProbeRecord, RuntimeProbeStore, artifact_for_model,
    identity_for_model, probe_targets_for_search_profile,
};
use crate::search::semantic::{SemanticIndexMetadata, SemanticVectorIndex};
use crate::search_models::{AcceptedLicenses, ModelArtifacts, SearchThresholdStore, sha256_file};
use crate::search_profile::{ExternalDependencies, SearchConfig, SearchProfile};

/// Hash a manifest-managed file for drift detection. A read failure (e.g. a
/// permission-denied file) is reported as a diagnostic finding rather than
/// aborting the whole `doctor` run via `?`.
fn hash_managed_file(path: &Path, role: &str) -> std::result::Result<String, String> {
    match fs::read(path) {
        Ok(bytes) => Ok(sha256_hex(&bytes)),
        Err(error) => Err(format!("Unreadable {role}: {} ({error})", path.display())),
    }
}

pub fn run(_args: &crate::cli::DoctorArgs, context: &crate::cli::CliContext) -> Result<()> {
    context.diagnostic("command: doctor");
    let paths = Paths::from_env()?;
    let bin = instance::binary_stem();
    println!("Instance: {bin}");
    if instance::is_test() {
        println!(
            "Caveat: the `{bin}` instance isolates the *install* (managed home, skills, \
             registry, caches, manifest) from production, but NOT the *project tree*. A \
             mutating skill run against a project writes that project's wiki/ and raw/ \
             exactly as production would."
        );
    }
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
    let mut findings = Vec::new();
    // A corrupt manifest is exactly what `doctor` should report, so surface the
    // parse failure as a finding instead of aborting the whole run via `?`.
    let manifest = match Manifest::read(&paths.manifest()) {
        Ok(manifest) => manifest,
        Err(error) => {
            findings.push(format!("manifest.json failed to parse: {error:#}"));
            None
        }
    };
    context.diagnostic(format!(
        "manifest state: {}",
        if manifest.is_some() {
            "present"
        } else {
            "missing"
        }
    ));
    if let Some(manifest) = &manifest {
        if !manifest.binary.path.exists() {
            findings.push(format!(
                "Missing managed binary: {}",
                manifest.binary.path.display()
            ));
        } else {
            match hash_managed_file(&manifest.binary.path, "managed binary") {
                Ok(current) if current != manifest.binary.hash => findings.push(format!(
                    "Drifted managed binary: {} (run `{bin} install` to refresh)",
                    manifest.binary.path.display()
                )),
                Ok(_) => {}
                Err(finding) => findings.push(finding),
            }
        }
        if let Some(path_binary) = find_on_path(managed_binary_name())
            && !same_path(&path_binary, &manifest.binary.path)
            && manifest.binary.path.exists()
        {
            context.diagnostic(format!("PATH binary: {}", path_binary.display()));
            match hash_managed_file(&path_binary, "PATH binary") {
                Ok(path_hash) if path_hash != manifest.binary.hash => findings.push(format!(
                    "PATH {bin} differs from managed binary: {} (rerun `{bin} install` after upgrading)",
                    path_binary.display()
                )),
                Ok(_) => {}
                Err(finding) => findings.push(finding),
            }
        }
        for entry in &manifest.skills {
            if !entry.path.exists() {
                findings.push(format!("Missing manifest file: {}", entry.path.display()));
            } else {
                match hash_managed_file(&entry.path, "manifest file") {
                    Ok(current) if current != entry.hash => findings.push(format!(
                        "Drifted manifest file: {} (run `{bin} install --force` to replace)",
                        entry.path.display()
                    )),
                    Ok(_) => {}
                    Err(finding) => findings.push(finding),
                }
            }
        }
        for asset in &manifest.assets {
            if !asset.path.exists() {
                findings.push(format!("Missing manifest asset: {}", asset.path.display()));
            } else {
                match hash_managed_file(&asset.path, "manifest asset") {
                    Ok(current) if current != asset.hash => findings.push(format!(
                        "Drifted manifest asset: {} (run `{bin} install --force` to replace)",
                        asset.path.display()
                    )),
                    Ok(_) => {}
                    Err(finding) => findings.push(finding),
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

    findings.extend(
        legacy_skills::existing_legacy_skill_dirs(&paths)
            .into_iter()
            .map(|dir| {
                format!(
                    "Legacy generated skill directory remains: {}",
                    dir.display()
                )
            }),
    );
    println!("Install:");
    if findings.is_empty() {
        println!("No {bin} install issues found.");
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
                "LLM search profile missing: {} (run `{} install --configure-search`)",
                search_config.display(),
                instance::binary_stem()
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

    print_runtime_probe_diagnostics(paths, context)?;

    Ok(())
}

fn print_runtime_probe_diagnostics(paths: &Paths, context: &crate::cli::CliContext) -> Result<()> {
    let probe_path = paths.search_runtime_probes();
    context.diagnostic(format!("runtime probes: {}", probe_path.display()));

    println!();
    println!("GGUF runtime:");

    let Some(config) = SearchConfig::read(&paths.search_config())? else {
        println!("Runtime probe skipped: LLM search profile missing.");
        print_last_probe_summary(RuntimeProbeStore::read(&probe_path)?.as_ref(), &[]);
        return Ok(());
    };
    let Some(profile) = active_llm_profile(&config) else {
        println!("Runtime probe skipped: LLM search disabled.");
        print_last_probe_summary(RuntimeProbeStore::read(&probe_path)?.as_ref(), &[]);
        return Ok(());
    };
    let profile_id = profile.profile.as_deref().unwrap_or("<unknown>");
    println!("Runtime profile: {profile_id}");

    let targets = match probe_targets_for_search_profile(profile) {
        Ok(targets) => targets,
        Err(error) => {
            println!("Current runtime probe skipped: {error:#}");
            print_last_probe_summary(RuntimeProbeStore::read(&probe_path)?.as_ref(), &[]);
            return Ok(());
        }
    };
    let store = RuntimeProbeStore::read(&probe_path)?;
    let artifacts = ModelArtifacts::read(&paths.model_artifacts())?;
    let identities = artifacts
        .as_ref()
        .map(|artifacts| {
            targets
                .iter()
                .copied()
                .filter_map(|target| {
                    artifact_for_model(artifacts, target.model.id)
                        .map(|artifact| identity_for_model(profile_id, target.model, artifact))
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    print_last_probe_summary(store.as_ref(), &identities);

    let Some(licenses) = AcceptedLicenses::read(&paths.accepted_licenses())? else {
        println!("Current runtime probe skipped: accepted license records missing.");
        return Ok(());
    };
    if let Some(target) = targets
        .iter()
        .copied()
        .find(|target| !licenses.accepts_model(target.model))
    {
        println!(
            "Current runtime probe skipped: accepted license record missing for {}.",
            target.model.id
        );
        return Ok(());
    }

    let Some(artifacts) = artifacts else {
        println!("Current runtime probe skipped: model artifact records missing.");
        return Ok(());
    };
    if let Some(target) = targets
        .iter()
        .copied()
        .find(|target| artifact_for_model(&artifacts, target.model.id).is_none())
    {
        println!(
            "Current runtime probe skipped: model artifact record missing for {}.",
            target.model.id
        );
        return Ok(());
    }
    for target in &targets {
        let artifact = artifact_for_model(&artifacts, target.model.id)
            .expect("artifact presence checked before runtime probe");
        match artifact.path.try_exists() {
            Ok(true) => {}
            Ok(false) => {
                println!(
                    "Current runtime probe skipped: model artifact file missing for {} at {}.",
                    target.model.id,
                    artifact.path.display()
                );
                return Ok(());
            }
            Err(error) => {
                println!(
                    "Current runtime probe skipped: model artifact file inaccessible for {} at {} ({error}).",
                    target.model.id,
                    artifact.path.display()
                );
                return Ok(());
            }
        }
        let observed = match sha256_file(&artifact.path) {
            Ok(observed) => observed,
            Err(error) => {
                println!(
                    "Current runtime probe skipped: model artifact file unreadable for {} at {} ({error:#}).",
                    target.model.id,
                    artifact.path.display()
                );
                return Ok(());
            }
        };
        if observed != artifact.observed_sha256 {
            println!(
                "Current runtime probe skipped: model artifact hash mismatch for {} at {}; expected {}, observed {}.",
                target.model.id,
                artifact.path.display(),
                artifact.observed_sha256,
                observed
            );
            return Ok(());
        }
    }

    let run = runtime_probe::probe_search_profile(profile, &artifacts)?;
    for record in &run.records {
        context.diagnostic(format!(
            "runtime probe current: role={}, outcome={}, requested_backend={}, used_backend={}, fallback={}, duration_ms={}",
            record.role,
            record.outcome.label(),
            record.requested_backend,
            record.used_backend.as_deref().unwrap_or("<unknown>"),
            record.fallback,
            record.duration_ms
        ));
    }
    if let Some(failure) = run.first_required_problem() {
        println!(
            "Current runtime probe: {} for {} during {} ({})",
            failure.outcome.label(),
            failure.role,
            failure.failure_stage.as_deref().unwrap_or("runtime_probe"),
            failure.failure_kind.as_deref().unwrap_or("unknown")
        );
    } else {
        let fallback = run.records.iter().any(|record| record.fallback);
        let used = common_used_backend(&run.records).unwrap_or("mixed");
        println!("Current runtime probe: passed (backend_used={used}, fallback={fallback})");
    }
    Ok(())
}

fn active_llm_profile(config: &SearchConfig) -> Option<&SearchProfile> {
    if config.project_default.llm_search_enabled {
        Some(&config.project_default)
    } else if config.global_search.llm_search_enabled {
        Some(&config.global_search)
    } else {
        None
    }
}

fn print_last_probe_summary(
    store: Option<&RuntimeProbeStore>,
    identities: &[runtime_probe::RuntimeProbeIdentity],
) {
    let Some(store) = store else {
        println!("Last runtime probe: missing.");
        return;
    };
    if store.records.is_empty() {
        println!("Last runtime probe: empty.");
        return;
    }
    if let Some(reason) = store.store_stale_reason() {
        println!("Last runtime probe: stale ({reason}).");
        return;
    }
    for identity in identities {
        let Some(record) = store.newest_record_for(identity) else {
            println!("Last runtime probe: stale (missing_record).");
            return;
        };
        if let Some(reason) = store.record_stale_reason(record, identity) {
            println!("Last runtime probe: stale ({reason}).");
            return;
        }
    }
    if let Some(record) = store
        .records
        .iter()
        .find(|record| record.outcome == RuntimeProbeOutcome::Failed)
    {
        println!(
            "Last runtime probe: failed for {} at {} (requested_backend={}, used_backend={})",
            record.role,
            record.probed_at,
            record.requested_backend,
            record.used_backend.as_deref().unwrap_or("<unknown>")
        );
        return;
    }
    if let Some(record) = store
        .records
        .iter()
        .find(|record| record.outcome == RuntimeProbeOutcome::Skipped)
    {
        println!(
            "Last runtime probe: skipped for {} at {} ({})",
            record.role,
            record.probed_at,
            record.message.as_deref().unwrap_or("no reason recorded")
        );
        return;
    }

    let fallback = store.records.iter().any(|record| record.fallback);
    let used = common_used_backend(&store.records).unwrap_or("mixed");
    println!(
        "Last runtime probe: passed at {} (backend_used={used}, fallback={fallback})",
        store.updated_at
    );
}

fn common_used_backend(records: &[RuntimeProbeRecord]) -> Option<&str> {
    let mut used = records
        .iter()
        .filter_map(|record| record.used_backend.as_deref());
    let first = used.next()?;
    if used.all(|value| value == first) {
        Some(first)
    } else {
        None
    }
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
    println!();
    println!("Registry:");
    // A corrupt registry is a diagnosis, not a reason to abort the whole run:
    // report it and continue with an empty registry for the remaining checks.
    let registry = match ProjectRegistry::read(&registry_path) {
        Ok(registry) => {
            context.diagnostic(format!("registered projects: {}", registry.projects.len()));
            if registry_path.exists() {
                println!(
                    "Project registry: {} ({} projects)",
                    registry_path.display(),
                    registry.projects.len()
                );
            } else {
                println!("Project registry missing: {}", registry_path.display());
            }
            registry
        }
        Err(error) => {
            context.diagnostic(format!("registry parse failed: {error:#}"));
            println!(
                "projects.json failed to parse: {} ({error:#})",
                registry_path.display()
            );
            ProjectRegistry::default()
        }
    };
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

    print_mcp_wiring_diagnostics(paths, &registry, context);

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
            "Current project is not registered; run `{} register {}`",
            instance::binary_stem(),
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
            "Legacy qmd-rs cache present: {} (run `{} index` to migrate into {})",
            legacy_store_path.display(),
            instance::binary_stem(),
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

/// Surface whether each host actually references the managed MCP server, turning
/// the otherwise-silent "server never connected" failure into a warning with a
/// fix hint. This never changes `doctor`'s exit status: a user may intentionally
/// run unwired, so every problem here is informational, and even an unreadable
/// config degrades to a printed line rather than aborting the run.
fn print_mcp_wiring_diagnostics(
    paths: &Paths,
    registry: &ProjectRegistry,
    context: &crate::cli::CliContext,
) {
    let bin = instance::binary_stem();
    let binary = paths.managed_binary();
    // A wired config still cannot spawn unless the managed binary it names is a
    // runnable file, so runnability (not mere existence) gates every "configured"
    // verdict below: matching config text is necessary but not sufficient for the
    // server to actually connect.
    let binary_state = managed_binary_state(&binary);
    let binary_runnable = binary_state == BinaryState::Runnable;
    println!();
    println!("MCP wiring:");
    if let Some(reason) = binary_state.unrunnable_reason() {
        println!(
            "Managed binary {reason}: {} (run `{bin} install`); host configs below cannot spawn the server until it is fixed",
            binary.display()
        );
    }

    let codex_path = paths.codex_config_toml();
    context.diagnostic(format!("codex config: {}", codex_path.display()));
    match fs::read_to_string(&codex_path) {
        Ok(contents) => match mcp_config::codex_server_wiring(&contents, &binary) {
            Ok(ServerWiring::Wired) if binary_runnable => {
                println!("Codex MCP server configured: {}", codex_path.display())
            }
            Ok(ServerWiring::Wired) => println!(
                "Codex MCP server configured but managed binary {}: {} (run `{bin} install`)",
                binary_state.unrunnable_reason().unwrap_or("unavailable"),
                codex_path.display()
            ),
            Ok(ServerWiring::Mismatched) => println!(
                "Codex MCP server command stale: {} (points at a different binary; run `{bin} install` to refresh)",
                codex_path.display()
            ),
            Ok(ServerWiring::Absent) => println!(
                "Codex MCP server not configured: {} (run `{bin} install`)",
                codex_path.display()
            ),
            Err(error) => println!(
                "Codex config unreadable as TOML: {} ({error})",
                codex_path.display()
            ),
        },
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => println!(
            "Codex config missing: {} (run `{bin} install`)",
            codex_path.display()
        ),
        Err(error) => println!(
            "Codex config unreadable: {} ({error})",
            codex_path.display()
        ),
    }

    let mut checked = 0usize;
    let mut unwired = 0usize;
    for project in &registry.projects {
        // A missing root is already reported above; skip it here to avoid a
        // redundant "not wired" line for a project that no longer exists.
        if !project.root.exists() {
            continue;
        }
        checked += 1;
        let mcp_json = project.root.join(".mcp.json");
        // Distinguish absent / invalid / (in)correctly-wired: an invalid
        // `.mcp.json` must NOT be reported as merely "not wired", because the
        // suggested `register`/`init` re-merge would hit the same parse failure
        // and cannot repair it — the file needs a manual fix or delete.
        let wiring = match fs::read_to_string(&mcp_json) {
            Ok(contents) => match mcp_config::claude_project_server_wiring(&contents, &binary) {
                Ok(wiring) => wiring,
                Err(error) => {
                    unwired += 1;
                    println!(
                        "Project {} MCP config invalid: {} ({error}); fix or delete it, then run `{bin} register {}` or `init`, or run `{bin} install` to materialize the fallback Claude template for manual wiring",
                        project.id,
                        mcp_json.display(),
                        project.root.display()
                    );
                    continue;
                }
            },
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => ServerWiring::Absent,
            Err(error) => {
                unwired += 1;
                println!(
                    "Project {} MCP config unreadable: {} ({error})",
                    project.id,
                    mcp_json.display()
                );
                continue;
            }
        };
        // "Wired" only counts when the managed binary it names is runnable.
        let hint = match wiring {
            ServerWiring::Wired if binary_runnable => None,
            ServerWiring::Wired => Some(format!(
                "wired but managed binary {}: {} (run `{bin} install`)",
                binary_state.unrunnable_reason().unwrap_or("unavailable"),
                mcp_json.display()
            )),
            ServerWiring::Mismatched => Some(format!(
                "wiring stale: {} (command points at a different binary; run `{bin} register {}` or `init` to refresh)",
                mcp_json.display(),
                project.root.display()
            )),
            ServerWiring::Absent => Some(format!(
                "not wired: {} (run `{bin} register {}` or `init` to wire it, or run `{bin} install` to materialize the fallback Claude template for manual wiring)",
                mcp_json.display(),
                project.root.display()
            )),
        };
        if let Some(hint) = hint {
            unwired += 1;
            println!("Project {} MCP {hint}", project.id);
        }
    }
    if checked == 0 {
        println!("No registered projects with an existing root to check for Claude MCP wiring.");
    } else if unwired == 0 {
        println!("All {checked} registered project(s) have Claude MCP wiring.");
    }
}

/// Whether the managed binary can actually be spawned by a host.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum BinaryState {
    Runnable,
    Missing,
    NotExecutable,
}

impl BinaryState {
    /// Short reason the binary cannot spawn, or `None` when it is runnable.
    /// Reads naturally after "Managed binary " and after "managed binary ".
    fn unrunnable_reason(self) -> Option<&'static str> {
        match self {
            Self::Runnable => None,
            Self::Missing => Some("missing"),
            Self::NotExecutable => Some("not executable"),
        }
    }
}

/// Classify the managed binary for `doctor`. A host cannot spawn a file that is
/// absent or (on Unix) lacks any execute bit, so existence alone would overstate
/// connectability. On Windows executability is not a POSIX permission bit, so a
/// present regular file is treated as runnable.
fn managed_binary_state(binary: &Path) -> BinaryState {
    let Ok(metadata) = fs::metadata(binary) else {
        return BinaryState::Missing;
    };
    if !metadata.is_file() {
        return BinaryState::Missing;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o111 == 0 {
            return BinaryState::NotExecutable;
        }
    }
    BinaryState::Runnable
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
