use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::io::{self, IsTerminal};
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, anyhow, bail};
use chrono::{Timelike, Utc};
use serde::Serialize;

use crate::backup_policy::exclude_rebuildable_from_time_machine;
use crate::cli::{CliContext, InstallArgs};
use crate::instance;
use crate::legacy_skills;
use crate::manifest::collision::{Collision, classify};
use crate::manifest::hash::sha256_hex;
use crate::manifest::{
    BackupEntry, BinaryEntry, FileKind, HashAlgorithm, ManagedAssetEntry, ManagedAssetKind,
    Manifest, ManifestEntry, Ownership, PartialInstall, RuntimeName,
};
use crate::mcp_config;
use crate::mcp_wiring;
use crate::path_guidance;
use crate::paths::{Paths, poman_binary_name};
use crate::progress::ProgressReporter;
use crate::search::runtime_probe::{self, RuntimeProbeStore};
use crate::search_models::{
    AcceptedLicenses, DEFAULT_PROFILE_ID, ModelArtifactClassification, ModelArtifactRecord,
    ModelArtifacts, ModelPosition, ProfileBundle, SearchModel, classify_model_artifact,
    classify_model_artifact_with_progress, download_and_verify_model, network_download,
    profile_by_id,
};
use crate::search_profile::{ExternalDependencies, SearchConfig, SearchProfile};

pub fn run(args: &InstallArgs, context: &CliContext) -> Result<()> {
    if args.disable_llm_search {
        context.diagnostic("search configuration: explicit disabled profile requested");
    }
    context.diagnostic("command: install");
    context.diagnostic(format!("force: {}", args.force));
    context.diagnostic(format!("path guidance: {}", !args.skip_path_guidance));
    context.diagnostic(format!("configure search: {}", args.configure_search));
    context.diagnostic(format!(
        "configure search requested: {}",
        args.configure_search
    ));
    context.diagnostic(format!("non-interactive: {}", args.non_interactive));
    context.diagnostic(format!("enable llm search: {}", args.enable_llm_search));
    context.diagnostic(format!("disable llm search: {}", args.disable_llm_search));
    context.diagnostic(format!(
        "search profile argument: {}",
        args.profile
            .map(|profile| profile.id())
            .unwrap_or(DEFAULT_PROFILE_ID)
    ));
    context.diagnostic(format!(
        "confirm model downloads: {}",
        args.confirm_model_downloads
    ));
    context.diagnostic(format!(
        "accept profile licenses: {}",
        args.accept_profile_licenses
    ));
    validate_install_args(args)?;
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
    context.diagnostic(format!(
        "search config: {}",
        paths.search_config().display()
    ));
    context.diagnostic(format!(
        "external dependencies: {}",
        paths.external_dependencies().display()
    ));
    context.diagnostic(format!(
        "accepted licenses: {}",
        paths.accepted_licenses().display()
    ));
    context.diagnostic(format!(
        "model artifacts: {}",
        paths.model_artifacts().display()
    ));
    context.diagnostic(format!(
        "runtime probes: {}",
        paths.search_runtime_probes().display()
    ));
    ensure_search_prompt_available(args)?;
    let progress = ProgressReporter::stderr();
    let enabled_search_preflight =
        preflight_noninteractive_enabled_search(args, &paths, context, &progress)?;
    let current_exe = env::current_exe().context("failed to resolve current executable")?;
    context.diagnostic(format!("current executable: {}", current_exe.display()));
    let current_exe_bytes = fs::read(&current_exe).with_context(|| {
        format!(
            "failed to read current executable {}",
            current_exe.display()
        )
    })?;
    let current_exe_hash = sha256_hex(&current_exe_bytes);
    let manifest = Manifest::read(&paths.manifest())?;
    context.diagnostic(format!(
        "manifest state: {}",
        if manifest.is_some() {
            "present"
        } else {
            "missing"
        }
    ));

    let running = RunningExe {
        path: &current_exe,
        hash: &current_exe_hash,
    };
    let recorded_poman = manifest
        .as_ref()
        .and_then(|manifest| manifest.poman.as_ref());
    let poman_source = choose_poman_source(&current_exe, recorded_poman, context)?;
    let poman_running = poman_source
        .as_ref()
        .map(|(path, hash)| RunningExe { path, hash });
    let managed_binary_hash = found_target_hash(&paths.managed_binary(), &running)?;
    let managed_poman_hash = match &poman_running {
        Some(source) => found_target_hash(&paths.managed_poman(), source)?,
        None => None,
    };

    let partial_state = recover_or_reject_partial(
        &paths,
        manifest.as_ref(),
        &running,
        managed_binary_hash.as_deref(),
        args.force,
        context,
    )?;
    context.diagnostic(format!(
        "partial marker recovery: {}",
        partial_state.label()
    ));

    let files = render_install_files(&paths, context)?;
    context.diagnostic(format!("rendered install files: {}", files.len()));
    preflight_binary(
        &BinaryInstall {
            source: &running,
            target: paths.managed_binary(),
            found_hash: managed_binary_hash.as_deref(),
            recorded: manifest.as_ref().map(|manifest| &manifest.binary),
            signing_identifier: managed_binary_signing_identifier(),
        },
        args.force,
        partial_state,
        context,
    )?;
    if let Some(source) = &poman_running {
        preflight_binary(
            &poman_install(
                &paths,
                source,
                managed_poman_hash.as_deref(),
                recorded_poman,
            ),
            args.force,
            partial_state,
            context,
        )?;
    }
    preflight_install_files(&files, manifest.as_ref(), args.force, context)?;
    let retained_legacy_skill_entries =
        cleanup_legacy_generated_skills(&paths, manifest.as_ref(), context)?;

    let partial = PartialInstall::new(
        current_exe.clone(),
        paths.managed_binary(),
        current_exe_hash.clone(),
    );
    partial.write_atomic(&paths.partial_install())?;

    let binary_install = BinaryInstall {
        source: &running,
        target: paths.managed_binary(),
        found_hash: managed_binary_hash.as_deref(),
        recorded: manifest.as_ref().map(|manifest| &manifest.binary),
        signing_identifier: managed_binary_signing_identifier(),
    };
    let poman_install = poman_running.as_ref().map(|source| {
        poman_install(
            &paths,
            source,
            managed_poman_hash.as_deref(),
            recorded_poman,
        )
    });
    let backup = write_backup_snapshot(
        &paths,
        &files,
        manifest.as_ref(),
        &binary_install,
        poman_install.as_ref(),
    )?;
    let binary = install_binary(&binary_install, args.force, partial_state, context)?;
    let poman = match &poman_install {
        Some(install) => Some(install_binary(install, args.force, partial_state, context)?),
        None => recorded_poman.cloned(),
    };
    let mcp_config = materialize_mcp_configs(&paths, &binary.path, context)?;
    let mut skill_entries = install_files(files, manifest.as_ref(), args.force, context)?;
    skill_entries.extend(retained_legacy_skill_entries);
    let mut assets = manifest
        .as_ref()
        .map(|manifest| manifest.assets.clone())
        .unwrap_or_default();
    assets.retain(|existing| existing.kind != ManagedAssetKind::McpConfig);
    assets.push(mcp_config);
    let mut backups = manifest
        .as_ref()
        .map(|manifest| manifest.backups.clone())
        .unwrap_or_default();
    backups.push(backup);
    Manifest::new(binary, poman, skill_entries, assets, backups).write_atomic(&paths.manifest())?;
    if paths.partial_install().exists() {
        fs::remove_file(paths.partial_install()).with_context(|| {
            format!(
                "failed to remove partial install marker {}",
                paths.partial_install().display()
            )
        })?;
    }
    configure_search(args, &paths, context, enabled_search_preflight, &progress)?;
    if !args.skip_path_guidance {
        path_guidance::print_guidance(&paths);
    }
    Ok(())
}

fn validate_install_args(args: &InstallArgs) -> Result<()> {
    if args.non_interactive && !args.enable_llm_search && !args.disable_llm_search {
        bail!(
            "non-interactive install requires either `--enable-llm-search --profile balanced` or `--disable-llm-search`"
        );
    }
    Ok(())
}

fn configure_search(
    args: &InstallArgs,
    paths: &Paths,
    context: &CliContext,
    enabled_search_preflight: Option<EnabledSearchPreflight>,
    progress: &ProgressReporter,
) -> Result<()> {
    context.diagnostic("search configuration action: start");
    if args.disable_llm_search {
        context.diagnostic("search configuration selected posture: disabled");
        if args.configure_search {
            context.diagnostic(
                "search configuration: --configure-search redundant beside explicit disabled posture",
            );
        }
        return configure_disabled_search(paths, context);
    }
    if args.enable_llm_search {
        context.diagnostic("search configuration selected posture: enabled");
        context.diagnostic("search configuration prompt skipped: non-interactive posture supplied");
        if args.configure_search {
            context.diagnostic(
                "search configuration: --configure-search redundant beside explicit enabled posture",
            );
        }
        return configure_enabled_search(args, paths, context, enabled_search_preflight, progress);
    }

    let posture = current_search_posture(paths, context);
    let selected = prompt_search_posture(posture)?;
    if selected == SearchInstallPosture::SemanticHybrid {
        return configure_enabled_search(args, paths, context, None, progress);
    }
    configure_disabled_search(paths, context)
}

fn ensure_search_prompt_available(args: &InstallArgs) -> Result<()> {
    if args.disable_llm_search || args.enable_llm_search || io::stdin().is_terminal() {
        return Ok(());
    }
    bail!(
        "interactive install requires a terminal for search setup; rerun with `--non-interactive --disable-llm-search` for lexical-only/no-LLM automation, rerun with `--non-interactive --enable-llm-search --profile balanced --confirm-model-downloads --accept-profile-licenses` for scripted LLM search, or run `{} install` from a terminal",
        instance::binary_stem()
    )
}

#[derive(Clone, Debug)]
struct EnabledSearchPreflight {
    profile: ProfileBundle,
    models: Vec<SearchModel>,
    install_plan: EnabledSearchInstallPlan,
}

fn preflight_noninteractive_enabled_search(
    args: &InstallArgs,
    paths: &Paths,
    context: &CliContext,
    progress: &ProgressReporter,
) -> Result<Option<EnabledSearchPreflight>> {
    if !args.enable_llm_search {
        return Ok(None);
    }

    context.diagnostic("search non-interactive preflight: start");
    let preflight = build_enabled_search_preflight(args, paths, context, progress)?;
    validate_noninteractive_enabled_search(args, &preflight.install_plan, context)?;
    context.diagnostic("search non-interactive preflight: accepted");
    Ok(Some(preflight))
}

fn build_enabled_search_preflight(
    args: &InstallArgs,
    paths: &Paths,
    context: &CliContext,
    progress: &ProgressReporter,
) -> Result<EnabledSearchPreflight> {
    let profile_id = args
        .profile
        .map(|profile| profile.id())
        .unwrap_or(DEFAULT_PROFILE_ID);
    let profile = profile_by_id(profile_id)
        .with_context(|| format!("LLM search profile `{profile_id}` is not available"))?;
    let models = runtime_probe::probe_targets_for_profile(profile)?
        .into_iter()
        .map(|target| target.model)
        .collect::<Vec<_>>();
    context.diagnostic(format!(
        "search profile selected: {} (source: {})",
        profile.id,
        if args.profile.is_some() {
            "--profile"
        } else {
            "default"
        }
    ));
    context.diagnostic(format!("search profile models: {}", models.len()));

    let accepted_licenses = AcceptedLicenses::read(&paths.accepted_licenses())?;
    let total_models = models.len();
    let mut classifications = Vec::with_capacity(total_models);
    for (offset, model) in models.iter().copied().enumerate() {
        let model_path = model.managed_path(&paths.managed_model_root());
        let classification = if model_path.exists() {
            let mut operation = progress.begin(
                "verify",
                model.id,
                offset + 1,
                total_models,
                model.expected_size_bytes,
            );
            let classification = classify_model_artifact_with_progress(
                model,
                profile,
                &paths.managed_model_root(),
                Some(&mut operation),
            )?;
            operation.finish();
            classification
        } else {
            classify_model_artifact(model, profile, &paths.managed_model_root())?
        };
        classifications.push((model, classification));
    }
    diagnose_model_classifications(&classifications, context);
    let install_plan = plan_enabled_search_install(
        &models,
        accepted_licenses.as_ref(),
        classifications,
        args.force,
    )?;
    diagnose_license_classifications(&models, &install_plan, context);

    Ok(EnabledSearchPreflight {
        profile,
        models,
        install_plan,
    })
}

fn diagnose_model_classifications(
    classifications: &[(SearchModel, ModelArtifactClassification)],
    context: &CliContext,
) {
    let mut verified = 0;
    let mut missing = 0;
    let mut hash_mismatch = 0;
    for (model, classification) in classifications {
        match classification {
            ModelArtifactClassification::Verified { .. } => {
                verified += 1;
                context.diagnostic(format!(
                    "search artifact classification: {} verified",
                    model.id
                ));
            }
            ModelArtifactClassification::Missing { path } => {
                missing += 1;
                context.diagnostic(format!(
                    "search artifact classification: {} missing at {}",
                    model.id,
                    path.display()
                ));
            }
            ModelArtifactClassification::HashMismatch {
                path,
                observed_sha256,
            } => {
                hash_mismatch += 1;
                context.diagnostic(format!(
                    "search artifact classification: {} hash-mismatch at {} observed_sha256={}",
                    model.id,
                    path.display(),
                    observed_sha256
                ));
            }
        }
    }
    context.diagnostic(format!(
        "search artifact classification summary: verified={verified}, missing={missing}, hash_mismatch={hash_mismatch}"
    ));
}

fn diagnose_license_classifications(
    models: &[SearchModel],
    install_plan: &EnabledSearchInstallPlan,
    context: &CliContext,
) {
    let missing_or_stale = install_plan.models_requiring_license_ack.len();
    let current = models.len().saturating_sub(missing_or_stale);
    context.diagnostic(format!(
        "search license classification summary: current={current}, missing_or_stale={missing_or_stale}"
    ));
}

fn validate_noninteractive_enabled_search(
    args: &InstallArgs,
    install_plan: &EnabledSearchInstallPlan,
    context: &CliContext,
) -> Result<()> {
    let mut missing = Vec::new();
    if install_plan.requires_download() && !args.confirm_model_downloads {
        missing.push("--confirm-model-downloads");
    }
    if install_plan.license_prompt_required && !args.accept_profile_licenses {
        missing.push("--accept-profile-licenses");
    }
    if !missing.is_empty() {
        context.diagnostic(format!(
            "search non-interactive missing runtime confirmation: {}",
            missing.join(", ")
        ));
        context.diagnostic("search non-interactive refusal: no install state mutated");
        bail!(
            "non-interactive LLM search install requires {}; no install or search state was changed",
            missing.join(" and ")
        );
    }
    if args.confirm_model_downloads && !install_plan.requires_download() {
        context.diagnostic(
            "search non-interactive confirmation: --confirm-model-downloads accepted as no-op",
        );
    }
    if args.accept_profile_licenses && !install_plan.license_prompt_required {
        context.diagnostic(
            "search non-interactive confirmation: --accept-profile-licenses accepted as no-op",
        );
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SearchInstallPosture {
    SemanticHybrid,
    LexicalOnly,
    Unconfigured,
}

impl SearchInstallPosture {
    fn starting_cursor(self) -> usize {
        match self {
            Self::SemanticHybrid | Self::Unconfigured => 0,
            Self::LexicalOnly => 1,
        }
    }
}

fn current_search_posture(paths: &Paths, context: &CliContext) -> SearchInstallPosture {
    match SearchConfig::read(&paths.search_config()) {
        Ok(Some(config))
            if config.project_default.llm_search_enabled
                || config.global_search.llm_search_enabled =>
        {
            context.diagnostic("search configuration current choice: semantic/hybrid");
            SearchInstallPosture::SemanticHybrid
        }
        Ok(Some(_)) => {
            context.diagnostic("search configuration current choice: lexical-only");
            SearchInstallPosture::LexicalOnly
        }
        Ok(None) => {
            context.diagnostic("search configuration current choice: unconfigured");
            SearchInstallPosture::Unconfigured
        }
        Err(error) => {
            context.diagnostic(format!(
                "search configuration current choice: unconfigured ({error:#})"
            ));
            SearchInstallPosture::Unconfigured
        }
    }
}

fn prompt_search_posture(current: SearchInstallPosture) -> Result<SearchInstallPosture> {
    const SEMANTIC_HYBRID: &str = "semantic/hybrid LLM search with the balanced profile";
    const LEXICAL_ONLY: &str = "lexical-only / no LLM search for now";

    let selected = inquire::Select::new("Search setup", vec![SEMANTIC_HYBRID, LEXICAL_ONLY])
        .with_starting_cursor(current.starting_cursor())
        .without_filtering()
        .prompt()?;
    if selected == SEMANTIC_HYBRID {
        Ok(SearchInstallPosture::SemanticHybrid)
    } else {
        Ok(SearchInstallPosture::LexicalOnly)
    }
}

fn configure_disabled_search(paths: &Paths, context: &CliContext) -> Result<()> {
    let config = SearchConfig::disabled();
    config.write_atomic(&paths.search_config())?;
    ExternalDependencies::empty().write_atomic(&paths.external_dependencies())?;
    AcceptedLicenses::from_models(&[]).write_atomic(&paths.accepted_licenses())?;
    context.diagnostic("search configuration action: wrote disabled LLM search profile");
    context.diagnostic(format!(
        "search config: {}",
        paths.search_config().display()
    ));
    context.diagnostic(format!(
        "external dependencies: {}",
        paths.external_dependencies().display()
    ));
    context.diagnostic(format!(
        "accepted licenses: {}",
        paths.accepted_licenses().display()
    ));
    if search_artifacts_present(paths)? {
        println!(
            "LLM search artifacts remain on disk. Remove them with `{} uninstall --search-artifacts` when you no longer need them.",
            instance::binary_stem()
        );
    }
    Ok(())
}

fn configure_enabled_search(
    args: &InstallArgs,
    paths: &Paths,
    context: &CliContext,
    preflight: Option<EnabledSearchPreflight>,
    progress: &ProgressReporter,
) -> Result<()> {
    let EnabledSearchPreflight {
        profile,
        models,
        install_plan,
    } = match preflight {
        Some(preflight) => preflight,
        None => build_enabled_search_preflight(args, paths, context, progress)?,
    };

    println!("LLM search profile: {}", profile.display_name);
    if args.enable_llm_search {
        if install_plan.requires_download() {
            println!("Models to download and verify:");
            for model in install_plan.models_to_download() {
                print_model_license_line(model, Some(model.expected_sha256));
            }
            for replacement in install_plan.models_to_replace() {
                println!(
                    "- {}: {} / {} ({}, sha256 {}, replacing local hash {}){}",
                    replacement.model.role.label(),
                    replacement.model.repository,
                    replacement.model.file,
                    license_label(replacement.model),
                    replacement.model.expected_sha256,
                    replacement.observed_sha256,
                    terms_suffix(replacement.model)
                );
            }
            let reused_unaccepted = install_plan.reused_models_requiring_license_ack();
            if !reused_unaccepted.is_empty() {
                println!(
                    "Already installed and verified, but license/terms acknowledgement was also required:"
                );
                for model in reused_unaccepted {
                    print_model_license_line(model, None);
                }
            }
            println!(
                "Model bytes are stored under {} and are not bundled with llm-wiki.",
                paths.managed_model_root().display()
            );
        } else if install_plan.license_prompt_required {
            println!("All required model artifacts are already installed and verified.");
            println!("Model license/terms acknowledgement was required for this profile:");
            for model in &models {
                print_model_license_line(*model, None);
            }
        } else {
            println!(
                "All required model artifacts are already installed and verified; reusing managed copies."
            );
        }
    } else if install_plan.requires_download() {
        println!("Models to download and verify:");
        for model in install_plan.models_to_download() {
            print_model_license_line(model, Some(model.expected_sha256));
        }
        for replacement in install_plan.models_to_replace() {
            println!(
                "- {}: {} / {} ({}, sha256 {}, replacing local hash {}){}",
                replacement.model.role.label(),
                replacement.model.repository,
                replacement.model.file,
                license_label(replacement.model),
                replacement.model.expected_sha256,
                replacement.observed_sha256,
                terms_suffix(replacement.model)
            );
        }
        let reused_unaccepted = install_plan.reused_models_requiring_license_ack();
        if !reused_unaccepted.is_empty() {
            println!(
                "Already installed and verified, but license/terms acknowledgement is also required:"
            );
            for model in reused_unaccepted {
                print_model_license_line(model, None);
            }
        }
        println!(
            "Model bytes are stored under {} and are not bundled with llm-wiki.",
            paths.managed_model_root().display()
        );
        let accepted = inquire::Confirm::new(
            "I acknowledge the listed model licenses/terms and want to download them now",
        )
        .with_default(false)
        .prompt()?;
        if !accepted {
            bail!("LLM search enablement cancelled; search profile was not changed");
        }
    } else if install_plan.license_prompt_required {
        println!("All required model artifacts are already installed and verified.");
        println!("Model license/terms acknowledgement is required before enabling this profile:");
        for model in &models {
            print_model_license_line(*model, None);
        }
        let accepted = inquire::Confirm::new(
            "I acknowledge the listed model licenses/terms and want to enable LLM search",
        )
        .with_default(false)
        .prompt()?;
        if !accepted {
            bail!("LLM search enablement cancelled; search profile was not changed");
        }
    } else {
        println!(
            "All required model artifacts are already installed and verified; reusing managed copies."
        );
    }

    for action in &install_plan.actions {
        if let PlannedModelAction::Reuse { model, record } = action {
            context.diagnostic(format!(
                "search model materialization: {} -> {} reused verified artifact",
                model.id,
                record.path.display()
            ));
        }
    }

    AcceptedLicenses::from_models(&models).write_atomic(&paths.accepted_licenses())?;
    context.diagnostic("search configuration action: recorded accepted licenses");

    fs::create_dir_all(paths.managed_model_root()).with_context(|| {
        format!(
            "failed to create model root {}",
            paths.managed_model_root().display()
        )
    })?;
    exclude_rebuildable_from_time_machine(&paths.managed_model_root(), context);
    fs::create_dir_all(paths.managed_index_root()).with_context(|| {
        format!(
            "failed to create index root {}",
            paths.managed_index_root().display()
        )
    })?;
    exclude_rebuildable_from_time_machine(&paths.managed_index_root(), context);

    let mut artifact_records = Vec::new();
    let total_models = install_plan.actions.len();
    for (offset, action) in install_plan.actions.into_iter().enumerate() {
        match action {
            PlannedModelAction::Reuse { record, .. } => {
                artifact_records.push(*record);
            }
            PlannedModelAction::Download { model }
            | PlannedModelAction::Replace {
                model,
                observed_sha256: _,
            } => {
                context.diagnostic(format!(
                    "search model materialization: {} -> {}",
                    model.id,
                    model.managed_path(&paths.managed_model_root()).display()
                ));
                let record = download_and_verify_model(
                    model,
                    profile,
                    &model.managed_path(&paths.managed_model_root()),
                    ModelPosition {
                        index: offset + 1,
                        total: total_models,
                    },
                    progress,
                    network_download(context),
                )?;
                context.diagnostic(format!(
                    "search model materialization outcome: {} -> downloaded",
                    model.id
                ));
                artifact_records.push(record);
            }
        }
    }
    let artifacts = ModelArtifacts::from_records(artifact_records);
    artifacts.write_atomic(&paths.model_artifacts())?;
    context.diagnostic("search configuration action: recorded model artifacts");

    let probe_run = runtime_probe::probe_profile_bundle(profile, &artifacts)?;
    let probe_store = RuntimeProbeStore::from_records(probe_run.records.clone());
    probe_store.write_atomic(&paths.search_runtime_probes())?;
    context.diagnostic(format!(
        "search runtime probes: records={} all_passed={}",
        probe_run.records.len(),
        probe_run.all_passed()
    ));
    context.diagnostic(format!(
        "runtime probes: {}",
        paths.search_runtime_probes().display()
    ));
    if let Some(failure) = probe_run.first_required_problem() {
        let reason = format!(
            "runtime_probe_failed:{}:{}:{}",
            failure.role,
            failure.failure_stage.as_deref().unwrap_or("runtime_probe"),
            failure.failure_kind.as_deref().unwrap_or("unknown")
        );
        SearchConfig::disabled_with_reason(reason).write_atomic(&paths.search_config())?;
        ExternalDependencies::empty().write_atomic(&paths.external_dependencies())?;
        context.diagnostic(
            "search configuration action: wrote disabled LLM search profile after failed runtime probe",
        );
        bail!(
            "GGUF runtime probe {} for {} during {} ({}); requested_backend={}, used_backend={}, LLM search profile was disabled and was not enabled",
            failure.outcome.label(),
            failure.role,
            failure.failure_stage.as_deref().unwrap_or("runtime_probe"),
            failure.failure_kind.as_deref().unwrap_or("unknown"),
            failure.requested_backend,
            failure.used_backend.as_deref().unwrap_or("<unknown>")
        );
    }

    let config = SearchConfig::enabled(SearchProfile::enabled(
        profile.id,
        profile.embedding_model,
        Some(profile.query_expansion_model.to_string()),
        profile.reranker_model.map(ToString::to_string),
    ));
    config.write_atomic(&paths.search_config())?;
    ExternalDependencies::empty().write_atomic(&paths.external_dependencies())?;
    context.diagnostic("search configuration action: wrote enabled LLM search profile");
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct EnabledSearchInstallPlan {
    actions: Vec<PlannedModelAction>,
    license_prompt_required: bool,
    models_requiring_license_ack: Vec<SearchModel>,
}

impl EnabledSearchInstallPlan {
    fn requires_download(&self) -> bool {
        self.actions.iter().any(|action| {
            matches!(
                action,
                PlannedModelAction::Download { .. } | PlannedModelAction::Replace { .. }
            )
        })
    }

    fn models_to_download(&self) -> Vec<SearchModel> {
        self.actions
            .iter()
            .filter_map(|action| match action {
                PlannedModelAction::Download { model } => Some(*model),
                _ => None,
            })
            .collect()
    }

    fn models_to_replace(&self) -> Vec<PlannedReplacement<'_>> {
        self.actions
            .iter()
            .filter_map(|action| match action {
                PlannedModelAction::Replace {
                    model,
                    observed_sha256,
                } => Some(PlannedReplacement {
                    model: *model,
                    observed_sha256,
                }),
                _ => None,
            })
            .collect()
    }

    fn reused_models_requiring_license_ack(&self) -> Vec<SearchModel> {
        self.actions
            .iter()
            .filter_map(|action| match action {
                PlannedModelAction::Reuse { model, .. }
                    if self.models_requiring_license_ack.contains(model) =>
                {
                    Some(*model)
                }
                _ => None,
            })
            .collect()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum PlannedModelAction {
    Reuse {
        model: SearchModel,
        record: Box<ModelArtifactRecord>,
    },
    Download {
        model: SearchModel,
    },
    Replace {
        model: SearchModel,
        observed_sha256: String,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PlannedReplacement<'a> {
    model: SearchModel,
    observed_sha256: &'a str,
}

fn plan_enabled_search_install(
    models: &[SearchModel],
    accepted_licenses: Option<&AcceptedLicenses>,
    classifications: Vec<(SearchModel, ModelArtifactClassification)>,
    force: bool,
) -> Result<EnabledSearchInstallPlan> {
    let models_requiring_license_ack = models
        .iter()
        .copied()
        .filter(|model| match accepted_licenses {
            Some(accepted) => !accepted.accepts_model(*model),
            None => true,
        })
        .collect::<Vec<_>>();
    let license_prompt_required = !models_requiring_license_ack.is_empty();
    if classifications.len() != models.len() {
        bail!(
            "internal search install plan error: classified {} models for {} required models",
            classifications.len(),
            models.len()
        );
    }
    let mut actions = Vec::with_capacity(classifications.len());

    for (expected_model, (model, classification)) in models.iter().copied().zip(classifications) {
        if model.id != expected_model.id {
            bail!(
                "internal search install plan error: classified {} while planning {}",
                model.id,
                expected_model.id
            );
        }
        match classification {
            ModelArtifactClassification::Verified { record } => {
                actions.push(PlannedModelAction::Reuse { model, record });
            }
            ModelArtifactClassification::Missing { .. } => {
                actions.push(PlannedModelAction::Download { model });
            }
            ModelArtifactClassification::HashMismatch {
                path,
                observed_sha256,
            } => {
                if !force {
                    bail!(
                        "model artifact hash mismatch for {}; rerun `{} install --configure-search --force` to replace {}",
                        model.id,
                        instance::binary_stem(),
                        path.display()
                    );
                }
                actions.push(PlannedModelAction::Replace {
                    model,
                    observed_sha256,
                });
            }
        }
    }

    Ok(EnabledSearchInstallPlan {
        actions,
        license_prompt_required,
        models_requiring_license_ack,
    })
}

fn print_model_license_line(model: SearchModel, expected_sha256: Option<&str>) {
    match expected_sha256 {
        Some(expected_sha256) => println!(
            "- {}: {} / {} ({}, sha256 {}){}",
            model.role.label(),
            model.repository,
            model.file,
            license_label(model),
            expected_sha256,
            terms_suffix(model)
        ),
        None => println!(
            "- {}: {} / {} ({}){}",
            model.role.label(),
            model.repository,
            model.file,
            license_label(model),
            terms_suffix(model)
        ),
    }
}

fn license_label(model: SearchModel) -> &'static str {
    model.license
}

fn terms_suffix(model: SearchModel) -> String {
    model
        .terms_url
        .map(|terms_url| format!(", terms {terms_url}"))
        .unwrap_or_default()
}

fn search_artifacts_present(paths: &Paths) -> Result<bool> {
    Ok(dir_has_entries(&paths.managed_model_root())?
        || semantic_sidecars_present(&paths.managed_index_root())?)
}

fn dir_has_entries(path: &Path) -> Result<bool> {
    if !path.exists() {
        return Ok(false);
    }
    let mut entries =
        fs::read_dir(path).with_context(|| format!("failed to read {}", path.display()))?;
    Ok(entries.next().transpose()?.is_some())
}

fn semantic_sidecars_present(root: &Path) -> Result<bool> {
    if !root.exists() {
        return Ok(false);
    }
    for entry in fs::read_dir(root).with_context(|| format!("failed to read {}", root.display()))? {
        let entry = entry.with_context(|| format!("failed to read {}", root.display()))?;
        let path = entry.path();
        if path.is_dir() {
            if semantic_sidecars_present(&path)? {
                return Ok(true);
            }
            continue;
        }
        if is_semantic_sidecar(&path) {
            return Ok(true);
        }
    }
    Ok(false)
}

fn is_semantic_sidecar(path: &Path) -> bool {
    matches!(
        path.file_name().and_then(|name| name.to_str()),
        Some("semantic-index.json" | "semantic-vectors.json")
    )
}

/// The running executable and its hash, taken once per install.
struct RunningExe<'a> {
    path: &'a Path,
    hash: &'a str,
}

/// A binary install copies into the managed bin folder: the managed binary
/// itself, or poman from beside it.
struct BinaryInstall<'a> {
    source: &'a RunningExe<'a>,
    target: PathBuf,
    /// The target's hash as install found it, `None` when it is missing.
    found_hash: Option<&'a str>,
    /// What the manifest recorded for this binary.
    recorded: Option<&'a BinaryEntry>,
    signing_identifier: String,
}

impl BinaryInstall<'_> {
    fn same_file(&self) -> bool {
        same_file_when_possible(self.source.path, &self.target)
    }

    fn manifest_owned(&self) -> bool {
        self.recorded.is_some_and(|entry| entry.path == self.target)
    }

    /// The target is the copy install made of this same source; on macOS the
    /// copy's hash differs from the source's because install signs it.
    fn source_matches(&self, target_hash: &str) -> bool {
        self.recorded.is_some_and(|entry| {
            entry.path == self.target
                && entry.hash == target_hash
                && entry.source_hash.as_deref().unwrap_or(target_hash) == self.source.hash
        })
    }

    fn refuse_unmanaged(
        &self,
        target_hash: &str,
        force: bool,
        partial_state: PartialState,
    ) -> Result<()> {
        if !self.source_matches(target_hash)
            && target_hash != self.source.hash
            && !self.manifest_owned()
            && !force
        {
            if partial_state == PartialState::Resuming {
                bail!(
                    "previous install left a partial managed binary {}; rerun with --force to replace it",
                    self.target.display()
                );
            }
            bail!(
                "refusing to replace unmanaged binary {}; rerun with --force to replace it",
                self.target.display()
            );
        }
        Ok(())
    }
}

fn managed_binary_signing_identifier() -> String {
    format!("dev.llm-wiki.{}", instance::binary_stem())
}

fn poman_install<'a>(
    paths: &Paths,
    source: &'a RunningExe<'a>,
    found_hash: Option<&'a str>,
    recorded: Option<&'a BinaryEntry>,
) -> BinaryInstall<'a> {
    BinaryInstall {
        source,
        target: paths.managed_poman(),
        found_hash,
        recorded,
        signing_identifier: "dev.llm-wiki.poman".to_string(),
    }
}

/// Where install takes poman from: the poman beside the running binary when
/// it is the same version, else none. With none, a poman the manifest already
/// records is kept as it is; with nothing recorded either, install refuses,
/// because llm-wiki is never installed without its poman.
fn choose_poman_source(
    current_exe: &Path,
    recorded: Option<&BinaryEntry>,
    context: &CliContext,
) -> Result<Option<(PathBuf, String)>> {
    match find_poman_beside(current_exe, context)? {
        Ok(found) => Ok(Some(found)),
        Err(why) => match recorded {
            Some(entry) => {
                let message = format!(
                    "poman was not updated: {why}; keeping the installed {}",
                    entry.path.display()
                );
                context.diagnostic(&message);
                println!("Warning: {message}");
                Ok(None)
            }
            None => bail!(
                "{} install needs poman {VERSION} beside it: {why}. The release ships poman with {}; put both in one folder and run install from there. If you installed with `cargo install llm-wiki-rs`, also run `cargo install poman --version {VERSION}`.",
                instance::binary_stem(),
                instance::binary_stem()
            ),
        },
    }
}

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The poman beside `current_exe` and its hash, or why there is no usable one.
fn find_poman_beside(
    current_exe: &Path,
    context: &CliContext,
) -> Result<std::result::Result<(PathBuf, String), String>> {
    let path = current_exe.with_file_name(poman_binary_name());
    context.diagnostic(format!(
        "poman beside the running binary: {}",
        path.display()
    ));
    if !path.is_file() {
        return Ok(Err(format!("there is no {}", path.display())));
    }
    let output = match Command::new(&path).arg("--version").output() {
        Ok(output) if output.status.success() => output,
        Ok(output) => {
            return Ok(Err(format!(
                "`{} --version` failed with {}",
                path.display(),
                output.status
            )));
        }
        Err(error) => {
            return Ok(Err(format!(
                "`{} --version` could not run: {error}",
                path.display()
            )));
        }
    };
    let reported = String::from_utf8_lossy(&output.stdout).trim().to_string();
    context.diagnostic(format!("poman version: {reported}"));
    if reported != format!("poman {VERSION}") {
        return Ok(Err(format!(
            "{} is `{reported}`, not poman {VERSION}, the version of {}",
            path.display(),
            instance::binary_stem()
        )));
    }
    let bytes = fs::read(&path).with_context(|| format!("failed to read {}", path.display()))?;
    Ok(Ok((path, sha256_hex(&bytes))))
}

fn hash_managed_binary(managed_binary: &Path) -> Result<String> {
    let bytes = fs::read(managed_binary)
        .with_context(|| format!("failed to read managed binary {}", managed_binary.display()))?;
    Ok(sha256_hex(&bytes))
}

/// A managed binary's hash as install finds it (`None` when it is missing),
/// taken once and reused: a debug build is large enough that each extra hash
/// costs seconds.
fn found_target_hash(target: &Path, source: &RunningExe) -> Result<Option<String>> {
    if same_file_when_possible(source.path, target) {
        // The source's bytes, already hashed, are this file's.
        return Ok(Some(source.hash.to_string()));
    }
    if !target.exists() {
        return Ok(None);
    }
    hash_managed_binary(target).map(Some)
}

fn preflight_binary(
    install: &BinaryInstall,
    force: bool,
    partial_state: PartialState,
    context: &CliContext,
) -> Result<()> {
    context.diagnostic(format!("managed binary path: {}", install.target.display()));
    if install.same_file() {
        context.diagnostic("managed binary comparison: same-file");
        return Ok(());
    }

    let Some(target_hash) = install.found_hash else {
        context.diagnostic("managed binary comparison: target missing");
        return Ok(());
    };
    context.diagnostic(format!(
        "managed binary comparison: hash_match={}, manifest_owned={}, source_match={}, force={}",
        target_hash == install.source.hash,
        install.manifest_owned(),
        install.source_matches(target_hash),
        force
    ));
    install.refuse_unmanaged(target_hash, force, partial_state)
}

fn preflight_install_files(
    files: &[InstallFile],
    manifest: Option<&Manifest>,
    force: bool,
    context: &CliContext,
) -> Result<()> {
    let manifest_by_path = manifest_entries_by_path(manifest);

    for file in files {
        let collision = classify_install_file(file, &manifest_by_path)?;
        context.diagnostic(format!(
            "collision classification: skill={} runtime={} kind={} path={} -> {:?}",
            file.skill,
            file.runtime.label(),
            file.kind.label(),
            file.path.display(),
            collision
        ));
        match collision {
            Collision::FreshInstall
            | Collision::RestoreMissing
            | Collision::Upgrade
            | Collision::UpToDate => {}
            Collision::UserEdited | Collision::UserEditedAndUpgrade | Collision::UnknownFile => {
                if !force {
                    bail!(
                        "refusing to overwrite {} ({collision:?}); rerun with --force to back up and replace",
                        file.path.display()
                    );
                }
            }
            Collision::Symlink => {
                if !force {
                    bail!(
                        "refusing to replace symlink {}; run `{} doctor` or rerun install --force",
                        file.path.display(),
                        instance::binary_stem()
                    );
                }
            }
        }
    }

    Ok(())
}

fn recover_or_reject_partial(
    paths: &Paths,
    manifest: Option<&Manifest>,
    running: &RunningExe,
    managed_binary_hash: Option<&str>,
    force: bool,
    context: &CliContext,
) -> Result<PartialState> {
    let partial_path = paths.partial_install();
    let Some(partial) = PartialInstall::read(&partial_path)? else {
        context.diagnostic("partial marker decision: none");
        return Ok(PartialState::None);
    };

    let managed_binary = paths.managed_binary();
    if let Some(manifest) = manifest
        && manifest.binary.path == managed_binary
        && managed_binary_hash == Some(manifest.binary.hash.as_str())
    {
        fs::remove_file(&partial_path).with_context(|| {
            format!(
                "failed to remove leaked partial install marker {}",
                partial_path.display()
            )
        })?;
        context.diagnostic("partial marker decision: leaked marker cleaned");
        return Ok(PartialState::LeakedMarkerCleaned);
    }
    if partial.target_binary != managed_binary && !force {
        context.diagnostic("partial marker decision: stale target refused");
        bail!(
            "stale partial install targets {}; rerun with --force to replace it",
            partial.target_binary.display()
        );
    }
    if partial.current_exe_hash != running.hash && !force {
        context.diagnostic("partial marker decision: stale hash refused");
        bail!(
            "stale partial install was started by a different binary; rerun with --force to replace it"
        );
    }
    context.diagnostic("partial marker decision: resuming");
    Ok(PartialState::Resuming)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PartialState {
    None,
    Resuming,
    LeakedMarkerCleaned,
}

impl PartialState {
    fn label(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Resuming => "resuming",
            Self::LeakedMarkerCleaned => "leaked-marker-cleaned",
        }
    }
}

fn install_binary(
    install: &BinaryInstall,
    force: bool,
    partial_state: PartialState,
    context: &CliContext,
) -> Result<BinaryEntry> {
    let (source, target) = (install.source, &install.target);
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    let target_hash = if install.same_file() {
        context.diagnostic("managed binary action: already running managed binary");
        source.hash.to_string()
    } else if target.exists() {
        let target_hash = match install.found_hash {
            Some(hash) => hash.to_string(),
            None => hash_managed_binary(target)?,
        };
        install.refuse_unmanaged(&target_hash, force, partial_state)?;
        if !install.source_matches(&target_hash) && target_hash != source.hash {
            context.diagnostic("managed binary action: replace existing target");
            copy_binary(source.path, target, &install.signing_identifier, context)?;
            hash_managed_binary(target)?
        } else {
            context.diagnostic("managed binary action: existing target hash matches");
            target_hash
        }
    } else {
        context.diagnostic("managed binary action: copy new target");
        copy_binary(source.path, target, &install.signing_identifier, context)?;
        hash_managed_binary(target)?
    };
    #[cfg(not(target_os = "macos"))]
    if target_hash != source.hash {
        bail!(
            "managed binary hash mismatch after copy: {}",
            target.display()
        );
    }

    Ok(BinaryEntry {
        path: target.clone(),
        version: VERSION.to_string(),
        hash_algorithm: HashAlgorithm::Sha256,
        hash: target_hash,
        source_hash: Some(source.hash.to_string()),
        ownership: Ownership::ManifestOwned,
    })
}

fn same_file_when_possible(left: &Path, right: &Path) -> bool {
    if !right.exists() {
        return left == right;
    }
    match (fs::canonicalize(left), fs::canonicalize(right)) {
        (Ok(left), Ok(right)) => left == right,
        _ => left == right,
    }
}

fn copy_binary(
    source: &Path,
    target: &Path,
    signing_identifier: &str,
    context: &CliContext,
) -> Result<()> {
    let parent = target.parent().ok_or_else(|| {
        anyhow!(
            "managed binary path has no parent directory: {}",
            target.display()
        )
    })?;
    let temp = tempfile::Builder::new()
        .prefix(".llm-wiki-bin-")
        .tempfile_in(parent)
        .with_context(|| {
            format!(
                "failed to create temporary managed binary in {}",
                parent.display()
            )
        })?;

    fs::copy(source, temp.path()).with_context(|| {
        format!(
            "failed to copy {} to temporary managed binary {}",
            source.display(),
            temp.path().display()
        )
    })?;
    let permissions = fs::metadata(source)
        .with_context(|| format!("failed to inspect {}", source.display()))?
        .permissions();
    fs::set_permissions(temp.path(), permissions)
        .with_context(|| format!("failed to set permissions on {}", temp.path().display()))?;
    sign_macos_binary(temp.path(), signing_identifier, context)?;
    temp.persist(target).map(|_| ()).map_err(|error| {
        anyhow!(
            "failed to atomically replace {} with staged binary {}: {}",
            target.display(),
            error.file.path().display(),
            error.error
        )
    })?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn sign_macos_binary(path: &Path, identifier: &str, context: &CliContext) -> Result<()> {
    context.diagnostic(format!(
        "macOS managed binary signing: codesign --force --sign - --identifier {identifier} {}",
        path.display()
    ));
    let output = Command::new("/usr/bin/codesign")
        .args(["--force", "--sign", "-", "--identifier", identifier])
        .arg(path)
        .output()
        .with_context(|| "failed to run codesign for managed binary")?;
    if !output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!(
            "codesign failed for managed binary {}: status={} stdout={} stderr={}",
            path.display(),
            output.status,
            stdout.trim(),
            stderr.trim()
        );
    }
    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn sign_macos_binary(_path: &Path, _identifier: &str, context: &CliContext) -> Result<()> {
    context.diagnostic("macOS managed binary signing: skipped on non-macOS");
    Ok(())
}

fn install_files(
    files: Vec<InstallFile>,
    manifest: Option<&Manifest>,
    force: bool,
    context: &CliContext,
) -> Result<Vec<ManifestEntry>> {
    let manifest_by_path = manifest_entries_by_path(manifest);

    let mut new_entries = Vec::with_capacity(files.len());
    for file in files {
        let collision = classify_install_file(&file, &manifest_by_path)?;
        context.diagnostic(format!(
            "install file action: skill={} runtime={} kind={} path={} collision={:?}",
            file.skill,
            file.runtime.label(),
            file.kind.label(),
            file.path.display(),
            collision
        ));

        match collision {
            Collision::FreshInstall | Collision::RestoreMissing | Collision::Upgrade => {
                write_file(&file.path, &file.contents)?;
            }
            Collision::UpToDate => {}
            Collision::UserEdited | Collision::UserEditedAndUpgrade | Collision::UnknownFile => {
                if !force {
                    bail!(
                        "refusing to overwrite {} ({collision:?}); rerun with --force to back up and replace",
                        file.path.display()
                    );
                }
                backup(&file.path)?;
                write_file(&file.path, &file.contents)?;
            }
            Collision::Symlink => {
                if !force {
                    bail!(
                        "refusing to replace symlink {}; run `{} doctor` or rerun install --force",
                        file.path.display(),
                        instance::binary_stem()
                    );
                }
                backup(&file.path)?;
                write_file(&file.path, &file.contents)?;
            }
        }

        new_entries.push(file.entry());
    }

    Ok(new_entries)
}

fn cleanup_legacy_generated_skills(
    paths: &Paths,
    manifest: Option<&Manifest>,
    context: &CliContext,
) -> Result<Vec<ManifestEntry>> {
    let Some(manifest) = manifest else {
        warn_existing_legacy_skill_dirs(paths, context);
        return Ok(Vec::new());
    };

    let mut retained = Vec::new();
    let mut touched_dirs = HashSet::new();

    for entry in &manifest.skills {
        if !legacy_skills::is_legacy_global_skill_path(paths, &entry.path) {
            retained.push(entry.clone());
            continue;
        }
        if !matches!(entry.ownership, Ownership::ManifestOwned) {
            warn_legacy_skill(
                context,
                format!(
                    "manual cleanup required: legacy generated skill is not manifest-owned: {}",
                    entry.path.display()
                ),
            );
            retained.push(entry.clone());
            continue;
        }
        if !matches!(entry.hash_algorithm, HashAlgorithm::Sha256) {
            warn_legacy_skill(
                context,
                format!(
                    "manual cleanup required: legacy generated skill uses unsupported hash algorithm: {}",
                    entry.path.display()
                ),
            );
            retained.push(entry.clone());
            continue;
        }

        match fs::read(&entry.path) {
            Ok(contents) if sha256_hex(&contents) == entry.hash => {
                fs::remove_file(&entry.path).with_context(|| {
                    format!(
                        "failed to remove legacy generated skill {}",
                        entry.path.display()
                    )
                })?;
                context.diagnostic(format!(
                    "removed legacy generated skill: {}",
                    entry.path.display()
                ));
                touched_dirs.insert(entry.path.clone());
            }
            Ok(_) => {
                warn_legacy_skill(
                    context,
                    format!(
                        "manual cleanup required: legacy generated skill was edited and was preserved: {}",
                        entry.path.display()
                    ),
                );
                retained.push(entry.clone());
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                context.diagnostic(format!(
                    "legacy generated skill already absent: {}",
                    entry.path.display()
                ));
            }
            Err(error) => {
                return Err(error).with_context(|| {
                    format!(
                        "failed to read legacy generated skill {}",
                        entry.path.display()
                    )
                });
            }
        }
    }

    for path in touched_dirs {
        for removed in
            legacy_skills::remove_empty_legacy_parents(&path, paths).with_context(|| {
                format!(
                    "failed to remove empty legacy generated skill directories for {}",
                    path.display()
                )
            })?
        {
            context.diagnostic(format!(
                "removed empty legacy generated skill directory: {}",
                removed.display()
            ));
        }
    }

    warn_existing_legacy_skill_dirs(paths, context);
    Ok(retained)
}

fn warn_existing_legacy_skill_dirs(paths: &Paths, context: &CliContext) {
    for dir in legacy_skills::existing_legacy_skill_dirs(paths) {
        warn_legacy_skill(
            context,
            format!(
                "manual cleanup required: legacy generated skill directory remains: {}",
                dir.display()
            ),
        );
    }
}

fn warn_legacy_skill(context: &CliContext, message: String) {
    context.diagnostic(&message);
    println!("Warning: {message}");
}

fn manifest_entries_by_path(manifest: Option<&Manifest>) -> HashMap<PathBuf, ManifestEntry> {
    manifest
        .map(|manifest| {
            manifest
                .skills
                .iter()
                .cloned()
                .map(|entry| (entry.path.clone(), entry))
                .collect()
        })
        .unwrap_or_default()
}

fn classify_install_file(
    file: &InstallFile,
    manifest_by_path: &HashMap<PathBuf, ManifestEntry>,
) -> Result<Collision> {
    let symlink = fs::symlink_metadata(&file.path)
        .map(|metadata| metadata.file_type().is_symlink())
        .unwrap_or(false);
    let current_hash = if file.path.exists() && !symlink {
        Some(sha256_hex(&fs::read(&file.path)?))
    } else {
        None
    };
    let manifest_hash = manifest_by_path
        .get(&file.path)
        .map(|entry| entry.hash.as_str());
    Ok(classify(
        current_hash.as_deref(),
        manifest_hash,
        &file.sha256,
        symlink,
    ))
}

/// Copies a managed binary install is about to replace into the backup
/// snapshot, unless it is the copy the manifest recorded or the source itself.
fn back_up_replaced_binary(
    install: &BinaryInstall,
    backup_path: &Path,
) -> Result<Option<BackupSnapshotFile>> {
    let Some(current_hash) = install.found_hash else {
        return Ok(None);
    };
    let manifest_owned = install
        .recorded
        .is_some_and(|entry| entry.path == install.target && entry.hash == current_hash);
    if current_hash == install.source.hash || manifest_owned {
        return Ok(None);
    }
    let current = fs::read(&install.target)
        .with_context(|| format!("failed to read managed binary {}", install.target.display()))?;
    fs::write(backup_path, current)
        .with_context(|| format!("failed to write {}", backup_path.display()))?;
    Ok(Some(BackupSnapshotFile {
        kind: BackupSnapshotKind::ManagedBinary,
        original_path: install.target.clone(),
        backup_path: backup_path.to_path_buf(),
        hash_algorithm: HashAlgorithm::Sha256,
        hash: current_hash.to_string(),
    }))
}

fn write_backup_snapshot(
    paths: &Paths,
    files: &[InstallFile],
    manifest: Option<&Manifest>,
    binary: &BinaryInstall,
    poman: Option<&BinaryInstall>,
) -> Result<BackupEntry> {
    let id = backup_id();
    let dir = paths.managed_home().join("backups").join(&id);
    fs::create_dir_all(&dir).with_context(|| format!("failed to create {}", dir.display()))?;

    let manifest_by_path = manifest_entries_by_path(manifest);

    let mut backed_up = Vec::new();
    for (install, name) in std::iter::once((binary, "0000-llm-wiki"))
        .chain(poman.map(|install| (install, "0000-poman")))
    {
        if let Some(backup) = back_up_replaced_binary(install, &dir.join(name))? {
            backed_up.push(backup);
        }
    }
    for (index, file) in files.iter().enumerate() {
        let symlink = fs::symlink_metadata(&file.path)
            .map(|metadata| metadata.file_type().is_symlink())
            .unwrap_or(false);
        if !file.path.exists() || symlink {
            continue;
        }
        let current = fs::read(&file.path)?;
        let current_hash = sha256_hex(&current);
        let manifest_hash = manifest_by_path
            .get(&file.path)
            .map(|entry| entry.hash.as_str());
        if Some(current_hash.as_str()) == manifest_hash {
            continue;
        }

        let file_name = file
            .path
            .file_name()
            .map(|name| name.to_string_lossy())
            .unwrap_or_else(|| "file".into());
        let backup_path = dir.join(format!("{:04}-{file_name}", index + 1));
        fs::write(&backup_path, current)
            .with_context(|| format!("failed to write {}", backup_path.display()))?;
        backed_up.push(BackupSnapshotFile {
            kind: BackupSnapshotKind::InstallFile,
            original_path: file.path.clone(),
            backup_path,
            hash_algorithm: HashAlgorithm::Sha256,
            hash: current_hash,
        });
    }

    let manifest_path = dir.join("backup-manifest.json");
    let snapshot = BackupSnapshotManifest {
        schema_version: 1,
        id: id.clone(),
        created_at: Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        files: backed_up,
    };
    let mut temp = tempfile::NamedTempFile::new_in(&dir)
        .with_context(|| format!("failed to create temp backup manifest in {}", dir.display()))?;
    serde_json::to_writer_pretty(&mut temp, &snapshot)
        .context("failed to serialize backup manifest")?;
    temp.persist(&manifest_path)
        .map_err(|err| err.error)
        .with_context(|| {
            format!(
                "failed to persist backup manifest {}",
                manifest_path.display()
            )
        })?;

    Ok(BackupEntry {
        id,
        path: manifest_path,
    })
}

fn materialize_mcp_configs(
    paths: &Paths,
    managed_binary: &Path,
    context: &CliContext,
) -> Result<ManagedAssetEntry> {
    // Codex is globally wired via the same shared core that `init`/`register`
    // use for project onboarding, so the two paths cannot drift: the merge is
    // non-destructive, the pre-llm-wiki config is snapshotted once, and the
    // merged config is written atomically.
    let codex_path = paths.codex_config_toml();
    mcp_wiring::ensure_codex_mcp_config(&codex_path, managed_binary, context)?;
    context.diagnostic(format!(
        "MCP server startup is {}: hosts spawn `{}` on demand; no background daemon is installed",
        mcp_config::SERVER_STARTUP,
        mcp_config::server_start_command(managed_binary)
    ));

    let claude_contents = mcp_config::render_claude_project_mcp_config(managed_binary)?;
    let claude_path = paths.claude_project_mcp_config();
    // The staged Claude MCP config is manifest-owned and drift-tracked, and
    // uninstall hard-fails if it was edited. Mirror that safety here: install
    // used to overwrite it silently. Least-invasive correct fix — warn on drift
    // before overwriting so a user edit is never lost without notice (a full
    // backup would leak an untracked sibling that uninstall does not clean).
    if let Ok(previous) = fs::read_to_string(&claude_path)
        && previous != claude_contents
    {
        let message = format!(
            "overwriting edited staged Claude MCP config {}; re-copy it to your project .mcp.json if you customized it",
            claude_path.display()
        );
        context.diagnostic(&message);
        println!("Warning: {message}");
    }
    write_file(&claude_path, &claude_contents)?;
    let hash = sha256_hex(claude_contents.as_bytes());
    context.diagnostic(format!(
        "materialized Claude MCP config: {} ({hash})",
        claude_path.display()
    ));
    context.diagnostic(format!(
        "Claude MCP is wired per project by `{} init`/`register`; this staged copy at {} is a fallback for manual setups",
        instance::binary_stem(),
        claude_path.display()
    ));

    Ok(ManagedAssetEntry {
        path: claude_path,
        asset: "claude-project-mcp-config".to_string(),
        kind: ManagedAssetKind::McpConfig,
        hash_algorithm: HashAlgorithm::Sha256,
        hash,
        ownership: Ownership::ManifestOwned,
        installed_by_version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

fn render_install_files(paths: &Paths, context: &CliContext) -> Result<Vec<InstallFile>> {
    context.diagnostic(format!(
        "runtime skill projection disabled; MCP config points to {}",
        paths.managed_binary().display()
    ));
    Ok(Vec::new())
}

fn write_file(path: &Path, contents: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    fs::write(path, contents).with_context(|| format!("failed to write {}", path.display()))
}

fn backup(path: &Path) -> Result<PathBuf> {
    let suffix = backup_suffix();
    backup_with_suffix(path, &suffix)
}

fn backup_id() -> String {
    format!("install-{}", backup_suffix())
}

fn backup_suffix() -> String {
    let now = Utc::now();
    format!("{}{:09}Z", now.format("%Y%m%dT%H%M%S"), now.nanosecond())
}

#[derive(Serialize)]
struct BackupSnapshotManifest {
    schema_version: u32,
    id: String,
    created_at: String,
    files: Vec<BackupSnapshotFile>,
}

#[derive(Serialize)]
struct BackupSnapshotFile {
    kind: BackupSnapshotKind,
    original_path: PathBuf,
    backup_path: PathBuf,
    hash_algorithm: HashAlgorithm,
    hash: String,
}

#[derive(Serialize)]
#[serde(rename_all = "kebab-case")]
enum BackupSnapshotKind {
    ManagedBinary,
    InstallFile,
}

fn backup_with_suffix(path: &Path, suffix: &str) -> Result<PathBuf> {
    for attempt in 0..100 {
        let candidate = if attempt == 0 {
            path.with_file_name(format!(
                "{}.bak.{suffix}",
                path.file_name().unwrap().to_string_lossy()
            ))
        } else {
            path.with_file_name(format!(
                "{}.bak.{suffix}.{attempt}",
                path.file_name().unwrap().to_string_lossy()
            ))
        };
        if !candidate.exists() {
            fs::rename(path, &candidate).with_context(|| {
                format!(
                    "failed to back up {} to {}",
                    path.display(),
                    candidate.display()
                )
            })?;
            return Ok(candidate);
        }
    }
    bail!("failed to create unique backup name for {}", path.display())
}

struct InstallFile {
    path: PathBuf,
    skill: String,
    runtime: RuntimeName,
    kind: FileKind,
    contents: String,
    sha256: String,
}

impl InstallFile {
    fn entry(&self) -> ManifestEntry {
        ManifestEntry {
            path: self.path.clone(),
            skill: self.skill.clone(),
            runtime: self.runtime,
            kind: self.kind,
            hash_algorithm: HashAlgorithm::Sha256,
            hash: self.sha256.clone(),
            ownership: Ownership::ManifestOwned,
            installed_by_version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}

trait InstallLabel {
    fn label(self) -> &'static str;
}

impl InstallLabel for RuntimeName {
    fn label(self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Codex => "codex",
        }
    }
}

impl InstallLabel for FileKind {
    fn label(self) -> &'static str {
        match self {
            Self::Skill => "skill",
            Self::RuntimeConfig => "runtime-config",
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    use tempfile::TempDir;

    use super::{
        EnabledSearchInstallPlan, EnabledSearchPreflight, PlannedModelAction, SearchInstallPosture,
        backup_with_suffix, configure_enabled_search, plan_enabled_search_install,
        validate_noninteractive_enabled_search,
    };
    use crate::cli::{InstallArgs, InstallSearchProfileArg};
    use crate::paths::Paths;
    use crate::progress::ProgressReporter;
    use crate::search_models::{
        ADAPTER_SCHEMA_VERSION, AcceptedLicenses, BALANCED_PROFILE, EMBEDDING_GEMMA_300M,
        ModelArtifactClassification, ModelArtifactRecord, QMD_QUERY_EXPANSION_17B, QMD_RS_VERSION,
        SearchModel,
    };
    use crate::search_profile::SearchConfig;
    use crate::test_env::EnvVarGuard;

    #[test]
    fn backup_retries_when_first_candidate_exists() {
        let temp = TempDir::new().expect("tempdir");
        let path = temp.path().join("SKILL.md");
        let first_backup = temp.path().join("SKILL.md.bak.fixed");
        fs::write(&path, "current").expect("write current");
        fs::write(&first_backup, "existing").expect("write first backup");

        let backup = backup_with_suffix(&path, "fixed").expect("backup");

        assert_eq!(backup, temp.path().join("SKILL.md.bak.fixed.1"));
        assert_eq!(fs::read_to_string(backup).expect("read backup"), "current");
        assert_eq!(
            fs::read_to_string(first_backup).expect("read first backup"),
            "existing"
        );
        assert!(!path.exists());
    }

    #[test]
    fn search_posture_cursor_defaults_to_semantic_hybrid() {
        assert_eq!(SearchInstallPosture::Unconfigured.starting_cursor(), 0);
        assert_eq!(SearchInstallPosture::SemanticHybrid.starting_cursor(), 0);
        assert_eq!(SearchInstallPosture::LexicalOnly.starting_cursor(), 1);
    }

    #[test]
    fn enabled_search_plan_reuses_verified_artifacts_when_licenses_current() {
        let models = [EMBEDDING_GEMMA_300M, QMD_QUERY_EXPANSION_17B];
        let accepted = AcceptedLicenses::from_models(&models);
        let plan = plan_enabled_search_install(
            &models,
            Some(&accepted),
            vec![
                (
                    EMBEDDING_GEMMA_300M,
                    ModelArtifactClassification::Verified {
                        record: Box::new(artifact_record(EMBEDDING_GEMMA_300M)),
                    },
                ),
                (
                    QMD_QUERY_EXPANSION_17B,
                    ModelArtifactClassification::Verified {
                        record: Box::new(artifact_record(QMD_QUERY_EXPANSION_17B)),
                    },
                ),
            ],
            false,
        )
        .expect("plan");

        assert!(!plan.license_prompt_required);
        assert!(!plan.requires_download());
        validate_noninteractive_enabled_search(
            &install_args(false, false, false),
            &plan,
            context(),
        )
        .expect("no confirmations required");
        assert!(matches!(
            plan.actions[0],
            PlannedModelAction::Reuse {
                model: EMBEDDING_GEMMA_300M,
                ..
            }
        ));
        assert!(matches!(
            plan.actions[1],
            PlannedModelAction::Reuse {
                model: QMD_QUERY_EXPANSION_17B,
                ..
            }
        ));
    }

    #[test]
    fn enabled_search_plan_prompts_license_only_for_verified_artifacts_with_stale_acceptance() {
        let models = [EMBEDDING_GEMMA_300M, QMD_QUERY_EXPANSION_17B];
        let plan = plan_enabled_search_install(
            &models,
            None,
            vec![
                (
                    EMBEDDING_GEMMA_300M,
                    ModelArtifactClassification::Verified {
                        record: Box::new(artifact_record(EMBEDDING_GEMMA_300M)),
                    },
                ),
                (
                    QMD_QUERY_EXPANSION_17B,
                    ModelArtifactClassification::Verified {
                        record: Box::new(artifact_record(QMD_QUERY_EXPANSION_17B)),
                    },
                ),
            ],
            false,
        )
        .expect("plan");

        assert!(plan.license_prompt_required);
        assert!(!plan.requires_download());
        let error = validate_noninteractive_enabled_search(
            &install_args(false, false, false),
            &plan,
            context(),
        )
        .expect_err("license confirmation required");
        assert!(format!("{error:#}").contains("--accept-profile-licenses"));
        validate_noninteractive_enabled_search(&install_args(false, true, false), &plan, context())
            .expect("license confirmation accepted");
    }

    #[test]
    fn enabled_search_plan_downloads_only_missing_artifacts() {
        let models = [EMBEDDING_GEMMA_300M, QMD_QUERY_EXPANSION_17B];
        let accepted = AcceptedLicenses::from_models(&models);
        let plan = plan_enabled_search_install(
            &models,
            Some(&accepted),
            vec![
                (
                    EMBEDDING_GEMMA_300M,
                    ModelArtifactClassification::Verified {
                        record: Box::new(artifact_record(EMBEDDING_GEMMA_300M)),
                    },
                ),
                (
                    QMD_QUERY_EXPANSION_17B,
                    ModelArtifactClassification::Missing {
                        path: PathBuf::from("/tmp/missing-expansion.gguf"),
                    },
                ),
            ],
            false,
        )
        .expect("plan");

        assert!(!plan.license_prompt_required);
        assert!(plan.requires_download());
        let error = validate_noninteractive_enabled_search(
            &install_args(false, false, false),
            &plan,
            context(),
        )
        .expect_err("download confirmation required");
        assert!(format!("{error:#}").contains("--confirm-model-downloads"));
        validate_noninteractive_enabled_search(&install_args(true, false, false), &plan, context())
            .expect("download confirmation accepted");
        assert_eq!(plan.models_to_download(), vec![QMD_QUERY_EXPANSION_17B]);
        assert!(matches!(
            plan.actions[0],
            PlannedModelAction::Reuse {
                model: EMBEDDING_GEMMA_300M,
                ..
            }
        ));
        assert!(matches!(
            plan.actions[1],
            PlannedModelAction::Download {
                model: QMD_QUERY_EXPANSION_17B
            }
        ));
    }

    #[test]
    fn enabled_search_plan_tracks_reused_unaccepted_models_when_download_needed() {
        let models = [EMBEDDING_GEMMA_300M, QMD_QUERY_EXPANSION_17B];
        let accepted = AcceptedLicenses::from_models(&[QMD_QUERY_EXPANSION_17B]);
        let plan = plan_enabled_search_install(
            &models,
            Some(&accepted),
            vec![
                (
                    EMBEDDING_GEMMA_300M,
                    ModelArtifactClassification::Verified {
                        record: Box::new(artifact_record(EMBEDDING_GEMMA_300M)),
                    },
                ),
                (
                    QMD_QUERY_EXPANSION_17B,
                    ModelArtifactClassification::Missing {
                        path: PathBuf::from("/tmp/missing-expansion.gguf"),
                    },
                ),
            ],
            false,
        )
        .expect("plan");

        assert!(plan.requires_download());
        assert!(plan.license_prompt_required);
        let error = validate_noninteractive_enabled_search(
            &install_args(true, false, false),
            &plan,
            context(),
        )
        .expect_err("profile license confirmation required");
        assert!(format!("{error:#}").contains("--accept-profile-licenses"));
        let error = validate_noninteractive_enabled_search(
            &install_args(false, true, false),
            &plan,
            context(),
        )
        .expect_err("download confirmation required");
        assert!(format!("{error:#}").contains("--confirm-model-downloads"));
        validate_noninteractive_enabled_search(&install_args(true, true, false), &plan, context())
            .expect("both confirmations accepted");
        assert_eq!(
            plan.reused_models_requiring_license_ack(),
            vec![EMBEDDING_GEMMA_300M]
        );
        assert_eq!(plan.models_to_download(), vec![QMD_QUERY_EXPANSION_17B]);
    }

    #[test]
    fn enabled_search_plan_refuses_hash_mismatch_without_force() {
        let models = [EMBEDDING_GEMMA_300M];

        let error = plan_enabled_search_install(
            &models,
            None,
            vec![(
                EMBEDDING_GEMMA_300M,
                ModelArtifactClassification::HashMismatch {
                    path: PathBuf::from("/tmp/corrupt-embedding.gguf"),
                    observed_sha256: "bad-sha".to_string(),
                },
            )],
            false,
        )
        .expect_err("mismatch should fail");

        assert!(format!("{error:#}").contains("model artifact hash mismatch"));
    }

    #[test]
    fn enabled_search_plan_force_replaces_only_mismatched_artifacts() {
        let models = [EMBEDDING_GEMMA_300M, QMD_QUERY_EXPANSION_17B];
        let accepted = AcceptedLicenses::from_models(&models);
        let plan = plan_enabled_search_install(
            &models,
            Some(&accepted),
            vec![
                (
                    EMBEDDING_GEMMA_300M,
                    ModelArtifactClassification::Verified {
                        record: Box::new(artifact_record(EMBEDDING_GEMMA_300M)),
                    },
                ),
                (
                    QMD_QUERY_EXPANSION_17B,
                    ModelArtifactClassification::HashMismatch {
                        path: PathBuf::from("/tmp/corrupt-expansion.gguf"),
                        observed_sha256: "bad-sha".to_string(),
                    },
                ),
            ],
            true,
        )
        .expect("plan");

        assert!(!plan.license_prompt_required);
        assert!(plan.requires_download());
        let error = validate_noninteractive_enabled_search(
            &install_args(false, false, true),
            &plan,
            context(),
        )
        .expect_err("forced replacement still needs download confirmation");
        assert!(format!("{error:#}").contains("--confirm-model-downloads"));
        validate_noninteractive_enabled_search(&install_args(true, false, true), &plan, context())
            .expect("forced replacement download confirmed");
        assert!(matches!(
            plan.actions[0],
            PlannedModelAction::Reuse {
                model: EMBEDDING_GEMMA_300M,
                ..
            }
        ));
        assert!(matches!(
            plan.actions[1],
            PlannedModelAction::Replace {
                model: QMD_QUERY_EXPANSION_17B,
                ..
            }
        ));
        assert_eq!(plan.models_to_replace()[0].observed_sha256, "bad-sha");
    }

    #[test]
    fn noninteractive_enabled_validator_accepts_extra_confirmations_when_current() {
        let models = [EMBEDDING_GEMMA_300M, QMD_QUERY_EXPANSION_17B];
        let accepted = AcceptedLicenses::from_models(&models);
        let plan = plan_enabled_search_install(
            &models,
            Some(&accepted),
            vec![
                (
                    EMBEDDING_GEMMA_300M,
                    ModelArtifactClassification::Verified {
                        record: Box::new(artifact_record(EMBEDDING_GEMMA_300M)),
                    },
                ),
                (
                    QMD_QUERY_EXPANSION_17B,
                    ModelArtifactClassification::Verified {
                        record: Box::new(artifact_record(QMD_QUERY_EXPANSION_17B)),
                    },
                ),
            ],
            false,
        )
        .expect("plan");

        validate_noninteractive_enabled_search(&install_args(true, true, false), &plan, context())
            .expect("extra confirmations are no-op scripted intent");
    }

    #[test]
    fn configure_enabled_search_records_runtime_probe_before_enabling_search() {
        let temp = TempDir::new().expect("tempdir");
        let paths = fixture_paths(temp.path());
        let _guard = EnvVarGuard::set("LLM_WIKI_TEST_GGUF_RUNTIME_PROBE", "pass");

        let progress = ProgressReporter::hidden();
        configure_enabled_search(
            &install_args(true, true, false),
            &paths,
            context(),
            Some(reuse_preflight()),
            &progress,
        )
        .expect("enabled search");

        let search = SearchConfig::read(&paths.search_config())
            .expect("read search")
            .expect("search config");
        assert!(search.project_default.llm_search_enabled);
        assert!(search.global_search.llm_search_enabled);
        let probes =
            crate::search::runtime_probe::RuntimeProbeStore::read(&paths.search_runtime_probes())
                .expect("read probes")
                .expect("runtime probes");
        assert_eq!(probes.records.len(), 2);
        assert!(probes.records.iter().all(|record| record.required));
        assert!(
            probes.records.iter().all(|record| record.outcome
                == crate::search::runtime_probe::RuntimeProbeOutcome::Passed)
        );
    }

    #[test]
    fn failed_runtime_probe_disables_previously_enabled_search_config() {
        let temp = TempDir::new().expect("tempdir");
        let paths = fixture_paths(temp.path());
        let existing = SearchConfig::enabled(crate::search_profile::SearchProfile::enabled(
            BALANCED_PROFILE.id,
            EMBEDDING_GEMMA_300M.id,
            Some(QMD_QUERY_EXPANSION_17B.id.to_string()),
            None,
        ));
        existing
            .write_atomic(&paths.search_config())
            .expect("existing search config");
        let _guard = EnvVarGuard::set("LLM_WIKI_TEST_GGUF_RUNTIME_PROBE", "fail:embedding");

        let progress = ProgressReporter::hidden();
        let error = configure_enabled_search(
            &install_args(true, true, false),
            &paths,
            context(),
            Some(reuse_preflight()),
            &progress,
        )
        .expect_err("runtime probe should fail");

        assert!(format!("{error:#}").contains("GGUF runtime probe failed"));
        let search = SearchConfig::read(&paths.search_config())
            .expect("read search")
            .expect("search config");
        assert!(!search.project_default.llm_search_enabled);
        assert!(!search.global_search.llm_search_enabled);
        assert!(
            search
                .project_default
                .reason
                .as_deref()
                .is_some_and(|reason| reason.starts_with("runtime_probe_failed:embedding:"))
        );
        let probes =
            crate::search::runtime_probe::RuntimeProbeStore::read(&paths.search_runtime_probes())
                .expect("read probes")
                .expect("runtime probes");
        assert_eq!(
            probes.records[0].outcome,
            crate::search::runtime_probe::RuntimeProbeOutcome::Failed
        );
    }

    fn context() -> &'static crate::cli::CliContext {
        const CONTEXT: crate::cli::CliContext = crate::cli::CliContext { verbose: false };
        &CONTEXT
    }

    fn fixture_paths(root: &Path) -> Paths {
        Paths {
            home: root.to_path_buf(),
            cache_home: root.join(".cache/llm-wiki"),
            data_home: root.join(".local/share/llm-wiki"),
            managed_home: root.join(".llm_wiki"),
        }
    }

    fn reuse_preflight() -> EnabledSearchPreflight {
        let models = vec![EMBEDDING_GEMMA_300M, QMD_QUERY_EXPANSION_17B];
        EnabledSearchPreflight {
            profile: BALANCED_PROFILE,
            models,
            install_plan: EnabledSearchInstallPlan {
                actions: vec![
                    PlannedModelAction::Reuse {
                        model: EMBEDDING_GEMMA_300M,
                        record: Box::new(artifact_record(EMBEDDING_GEMMA_300M)),
                    },
                    PlannedModelAction::Reuse {
                        model: QMD_QUERY_EXPANSION_17B,
                        record: Box::new(artifact_record(QMD_QUERY_EXPANSION_17B)),
                    },
                ],
                license_prompt_required: false,
                models_requiring_license_ack: Vec::new(),
            },
        }
    }

    fn install_args(
        confirm_model_downloads: bool,
        accept_profile_licenses: bool,
        force: bool,
    ) -> InstallArgs {
        InstallArgs {
            force,
            skip_path_guidance: false,
            configure_search: false,
            non_interactive: true,
            enable_llm_search: true,
            profile: Some(InstallSearchProfileArg::Balanced),
            confirm_model_downloads,
            accept_profile_licenses,
            disable_llm_search: false,
        }
    }

    fn artifact_record(model: SearchModel) -> ModelArtifactRecord {
        ModelArtifactRecord {
            model_id: model.id.to_string(),
            role: model.role.label().to_string(),
            profile: BALANCED_PROFILE.id.to_string(),
            repository: model.repository.to_string(),
            revision: model.revision.to_string(),
            file: model.file.to_string(),
            download_url: model.download_url(),
            path: Path::new("/tmp").join(model.file),
            expected_sha256: model.expected_sha256.to_string(),
            observed_sha256: model.expected_sha256.to_string(),
            size_bytes: model.expected_size_bytes,
            license: model.license.to_string(),
            terms_url: model.terms_url.map(ToString::to_string),
            dimensions: model.dimensions,
            qmd_rs_version: QMD_RS_VERSION.to_string(),
            adapter_schema_version: ADAPTER_SCHEMA_VERSION,
            verified_at: "2026-05-14T00:00:00Z".to_string(),
        }
    }
}
