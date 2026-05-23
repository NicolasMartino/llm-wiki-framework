use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::{self, IsTerminal};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use chrono::{Timelike, Utc};
use llm_wiki_schema::{ClaudeProjector, CodexProjector, Projector, Runtime, parse};
use serde::Serialize;

use crate::backup_policy::exclude_rebuildable_from_time_machine;
use crate::cli::{CliContext, InstallArgs};
use crate::embed;
use crate::manifest::collision::{Collision, classify};
use crate::manifest::hash::sha256_hex;
use crate::manifest::{
    BackupEntry, BinaryEntry, FileKind, HashAlgorithm, Manifest, ManifestEntry, Ownership,
    PartialInstall, RuntimeName,
};
use crate::path_guidance;
use crate::paths::Paths;
use crate::search_models::{
    AcceptedLicenses, DEFAULT_PROFILE_ID, MaterializationOutcome, ModelArtifactClassification,
    ModelArtifactRecord, ModelArtifacts, ProfileBundle, SearchModel, classify_model_artifact,
    materialize_model, models_for_profile, profile_by_id,
};
use crate::search_profile::{ExternalDependencies, SearchConfig, SearchProfile};
use crate::skill_render::{apply_binary_context, managed_binary_invocation};

pub fn run(args: &InstallArgs, context: &CliContext) -> Result<()> {
    if args.disable_llm_search {
        context.diagnostic("search configuration: explicit disabled profile requested");
    }
    context.diagnostic("command: install");
    context.diagnostic(format!("force: {}", args.force));
    context.diagnostic(format!("path guidance: {}", !args.skip_path_guidance));
    context.diagnostic(format!("configure search: {}", true));
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
    ensure_search_prompt_available(args)?;
    let enabled_search_preflight = preflight_noninteractive_enabled_search(args, &paths, context)?;
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

    let partial_state = recover_or_reject_partial(
        &paths,
        manifest.as_ref(),
        &current_exe_hash,
        args.force,
        context,
    )?;
    context.diagnostic(format!(
        "partial marker recovery: {}",
        partial_state.label()
    ));

    let files = render_install_files(&paths, context)?;
    context.diagnostic(format!("rendered install files: {}", files.len()));
    preflight_managed_binary(
        &paths,
        &current_exe,
        &current_exe_bytes,
        manifest.as_ref(),
        args.force,
        partial_state,
        context,
    )?;
    preflight_install_files(&files, manifest.as_ref(), args.force, context)?;

    let partial = PartialInstall::new(
        current_exe.clone(),
        paths.managed_binary(),
        current_exe_hash.clone(),
    );
    partial.write_atomic(&paths.partial_install())?;

    let backup = write_backup_snapshot(&paths, &files, manifest.as_ref(), &current_exe_hash)?;
    let binary = install_managed_binary(
        &paths,
        &current_exe,
        &current_exe_bytes,
        &manifest,
        args.force,
        partial_state,
        context,
    )?;
    let skill_entries = install_files(files, manifest.as_ref(), args.force, context)?;
    let mut backups = manifest
        .as_ref()
        .map(|manifest| manifest.backups.clone())
        .unwrap_or_default();
    backups.push(backup);
    Manifest::new(binary, skill_entries, backups).write_atomic(&paths.manifest())?;
    if paths.partial_install().exists() {
        fs::remove_file(paths.partial_install()).with_context(|| {
            format!(
                "failed to remove partial install marker {}",
                paths.partial_install().display()
            )
        })?;
    }
    configure_search(args, &paths, context, enabled_search_preflight)?;
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
        return configure_enabled_search(args, paths, context, enabled_search_preflight);
    }

    let posture = current_search_posture(paths, context);
    let selected = prompt_search_posture(posture)?;
    if selected == SearchInstallPosture::SemanticHybrid {
        return configure_enabled_search(args, paths, context, None);
    }
    configure_disabled_search(paths, context)
}

fn ensure_search_prompt_available(args: &InstallArgs) -> Result<()> {
    if args.disable_llm_search || args.enable_llm_search || io::stdin().is_terminal() {
        return Ok(());
    }
    bail!(
        "interactive install requires a terminal for search setup; rerun with `--non-interactive --disable-llm-search` for lexical-only/no-LLM automation, rerun with `--non-interactive --enable-llm-search --profile balanced --confirm-model-downloads --accept-profile-licenses` for scripted LLM search, or run `llm-wiki install` from a terminal"
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
) -> Result<Option<EnabledSearchPreflight>> {
    if !args.enable_llm_search {
        return Ok(None);
    }

    context.diagnostic("search non-interactive preflight: start");
    let preflight = build_enabled_search_preflight(args, paths, context)?;
    validate_noninteractive_enabled_search(args, &preflight.install_plan, context)?;
    context.diagnostic("search non-interactive preflight: accepted");
    Ok(Some(preflight))
}

fn build_enabled_search_preflight(
    args: &InstallArgs,
    paths: &Paths,
    context: &CliContext,
) -> Result<EnabledSearchPreflight> {
    let profile_id = args
        .profile
        .map(|profile| profile.id())
        .unwrap_or(DEFAULT_PROFILE_ID);
    let profile = profile_by_id(profile_id)
        .with_context(|| format!("LLM search profile `{profile_id}` is not available"))?;
    let models = models_for_profile(profile, false)?;
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
    let classifications = models
        .iter()
        .copied()
        .map(|model| {
            classify_model_artifact(model, profile, &paths.managed_model_root())
                .map(|classification| (model, classification))
        })
        .collect::<Result<Vec<_>>>()?;
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
            "LLM search artifacts remain on disk. Remove them with `llm-wiki uninstall --search-artifacts` when you no longer need them."
        );
    }
    Ok(())
}

fn configure_enabled_search(
    args: &InstallArgs,
    paths: &Paths,
    context: &CliContext,
    preflight: Option<EnabledSearchPreflight>,
) -> Result<()> {
    let EnabledSearchPreflight {
        profile,
        models,
        install_plan,
    } = match preflight {
        Some(preflight) => preflight,
        None => build_enabled_search_preflight(args, paths, context)?,
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
    for action in install_plan.actions {
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
                let materialized =
                    materialize_model(model, profile, &paths.managed_model_root(), args.force)?;
                context.diagnostic(format!(
                    "search model materialization outcome: {} -> {}",
                    model.id,
                    match materialized.outcome {
                        MaterializationOutcome::Reused => "reused",
                        MaterializationOutcome::Downloaded => "downloaded",
                    }
                ));
                artifact_records.push(materialized.record);
            }
        }
    }
    ModelArtifacts::from_records(artifact_records).write_atomic(&paths.model_artifacts())?;
    context.diagnostic("search configuration action: recorded model artifacts");

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
                        "model artifact hash mismatch for {}; rerun `llm-wiki install --configure-search --force` to replace {}",
                        model.id,
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

fn preflight_managed_binary(
    paths: &Paths,
    current_exe: &Path,
    current_exe_bytes: &[u8],
    manifest: Option<&Manifest>,
    force: bool,
    partial_state: PartialState,
    context: &CliContext,
) -> Result<()> {
    let managed_binary = paths.managed_binary();
    let current_hash = sha256_hex(current_exe_bytes);
    context.diagnostic(format!("managed binary path: {}", managed_binary.display()));
    if same_file_when_possible(current_exe, &managed_binary) {
        let managed_hash = sha256_hex(&fs::read(&managed_binary).with_context(|| {
            format!("failed to read managed binary {}", managed_binary.display())
        })?);
        context.diagnostic(format!(
            "managed binary comparison: same-file hash_match={}",
            managed_hash == current_hash
        ));
        if managed_hash != current_hash {
            bail!(
                "managed binary {} differs from current executable",
                managed_binary.display()
            );
        }
        return Ok(());
    }

    if !managed_binary.exists() {
        context.diagnostic("managed binary comparison: target missing");
        return Ok(());
    }

    let managed_hash =
        sha256_hex(&fs::read(&managed_binary).with_context(|| {
            format!("failed to read managed binary {}", managed_binary.display())
        })?);
    let manifest_owned = manifest.is_some_and(|manifest| manifest.binary.path == managed_binary);
    context.diagnostic(format!(
        "managed binary comparison: hash_match={}, manifest_owned={}, force={}",
        managed_hash == current_hash,
        manifest_owned,
        force
    ));
    if managed_hash != current_hash && !manifest_owned && !force {
        if partial_state == PartialState::Resuming {
            bail!(
                "previous install left a partial managed binary {}; rerun with --force to replace it",
                managed_binary.display()
            );
        }
        bail!(
            "refusing to replace unmanaged binary {}; rerun with --force to replace it",
            managed_binary.display()
        );
    }
    Ok(())
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
                        "refusing to replace symlink {}; run llm-wiki doctor or rerun install --force",
                        file.path.display()
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
    current_exe_hash: &str,
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
        && managed_binary.exists()
        && sha256_hex(&fs::read(&managed_binary).with_context(|| {
            format!("failed to read managed binary {}", managed_binary.display())
        })?) == manifest.binary.hash
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
    if partial.current_exe_hash != current_exe_hash && !force {
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

fn install_managed_binary(
    paths: &Paths,
    current_exe: &Path,
    current_exe_bytes: &[u8],
    manifest: &Option<Manifest>,
    force: bool,
    partial_state: PartialState,
    context: &CliContext,
) -> Result<BinaryEntry> {
    let managed_binary = paths.managed_binary();
    if let Some(parent) = managed_binary.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    let current_hash = sha256_hex(current_exe_bytes);
    if same_file_when_possible(current_exe, &managed_binary) {
        let managed_hash = sha256_hex(&fs::read(&managed_binary).with_context(|| {
            format!("failed to read managed binary {}", managed_binary.display())
        })?);
        context.diagnostic("managed binary action: already running managed binary");
        if managed_hash != current_hash {
            bail!(
                "managed binary {} differs from current executable",
                managed_binary.display()
            );
        }
    } else if managed_binary.exists() {
        let managed_hash = sha256_hex(&fs::read(&managed_binary)?);
        let manifest_owned = manifest
            .as_ref()
            .is_some_and(|manifest| manifest.binary.path == managed_binary);
        if managed_hash != current_hash && !manifest_owned && !force {
            if partial_state == PartialState::Resuming {
                bail!(
                    "previous install left a partial managed binary {}; rerun with --force to replace it",
                    managed_binary.display()
                );
            }
            bail!(
                "refusing to replace unmanaged binary {}; rerun with --force to replace it",
                managed_binary.display()
            );
        }
        if managed_hash != current_hash {
            context.diagnostic("managed binary action: replace existing target");
            copy_current_exe(current_exe, &managed_binary)?;
        } else {
            context.diagnostic("managed binary action: existing target hash matches");
        }
    } else {
        context.diagnostic("managed binary action: copy new target");
        copy_current_exe(current_exe, &managed_binary)?;
    }

    let managed_hash = sha256_hex(&fs::read(&managed_binary)?);
    if managed_hash != current_hash {
        bail!(
            "managed binary hash mismatch after copy: {}",
            managed_binary.display()
        );
    }

    Ok(BinaryEntry {
        path: managed_binary,
        version: env!("CARGO_PKG_VERSION").to_string(),
        hash_algorithm: HashAlgorithm::Sha256,
        hash: managed_hash,
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

fn copy_current_exe(current_exe: &Path, managed_binary: &Path) -> Result<()> {
    fs::copy(current_exe, managed_binary).with_context(|| {
        format!(
            "failed to copy {} to {}",
            current_exe.display(),
            managed_binary.display()
        )
    })?;
    let permissions = fs::metadata(current_exe)
        .with_context(|| format!("failed to inspect {}", current_exe.display()))?
        .permissions();
    fs::set_permissions(managed_binary, permissions)
        .with_context(|| format!("failed to set permissions on {}", managed_binary.display()))?;
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
                        "refusing to replace symlink {}; run llm-wiki doctor or rerun install --force",
                        file.path.display()
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

fn write_backup_snapshot(
    paths: &Paths,
    files: &[InstallFile],
    manifest: Option<&Manifest>,
    current_exe_hash: &str,
) -> Result<BackupEntry> {
    let id = backup_id();
    let dir = paths.managed_home().join("backups").join(&id);
    fs::create_dir_all(&dir).with_context(|| format!("failed to create {}", dir.display()))?;

    let manifest_by_path = manifest_entries_by_path(manifest);

    let mut backed_up = Vec::new();
    let managed_binary = paths.managed_binary();
    if managed_binary.exists() {
        let current = fs::read(&managed_binary).with_context(|| {
            format!("failed to read managed binary {}", managed_binary.display())
        })?;
        let current_hash = sha256_hex(&current);
        if current_hash != current_exe_hash {
            let backup_path = dir.join("0000-llm-wiki");
            fs::write(&backup_path, current)
                .with_context(|| format!("failed to write {}", backup_path.display()))?;
            backed_up.push(BackupSnapshotFile {
                kind: BackupSnapshotKind::ManagedBinary,
                original_path: managed_binary,
                backup_path,
                hash_algorithm: HashAlgorithm::Sha256,
                hash: current_hash,
            });
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

fn render_install_files(paths: &Paths, context: &CliContext) -> Result<Vec<InstallFile>> {
    let mut files = Vec::new();
    for asset in embed::SKILLS {
        let doc = parse(asset.skill_md)
            .with_context(|| format!("failed to parse embedded skill {}", asset.name))?;
        let binary_invocation = managed_binary_invocation(&paths.managed_binary());
        let doc = apply_binary_context(doc, &binary_invocation);
        if doc.frontmatter.runtimes.contains(&Runtime::Claude) {
            let rendered = ClaudeProjector.project(&doc)?;
            context.diagnostic(format!(
                "render target: skill={} runtime=claude path={}",
                asset.name,
                paths.claude_skill(asset.name).display()
            ));
            files.push(InstallFile::new(
                paths.claude_skill(asset.name),
                asset.name,
                RuntimeName::Claude,
                FileKind::Skill,
                rendered.skill_md,
            ));
        }
        if doc.frontmatter.runtimes.contains(&Runtime::Codex) {
            let runtime_config = llm_wiki_schema::CodexRuntimeConfig::from_yaml(asset.codex_openai)
                .with_context(|| format!("failed to parse {} Codex runtime config", asset.name))?;
            let rendered = CodexProjector::with_runtime_config(runtime_config).project(&doc)?;
            context.diagnostic(format!(
                "render target: skill={} runtime=codex path={}",
                asset.name,
                paths.codex_skill(asset.name).display()
            ));
            files.push(InstallFile::new(
                paths.codex_skill(asset.name),
                asset.name,
                RuntimeName::Codex,
                FileKind::Skill,
                rendered.skill_md,
            ));
            files.push(InstallFile::new(
                paths.codex_config(asset.name),
                asset.name,
                RuntimeName::Codex,
                FileKind::RuntimeConfig,
                rendered.runtime_config.context("missing runtime config")?,
            ));
        }
    }
    Ok(files)
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
    fn new(
        path: PathBuf,
        skill: impl Into<String>,
        runtime: RuntimeName,
        kind: FileKind,
        contents: String,
    ) -> Self {
        let sha256 = sha256_hex(contents.as_bytes());
        Self {
            path,
            skill: skill.into(),
            runtime,
            kind,
            contents,
            sha256,
        }
    }

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
        PlannedModelAction, SearchInstallPosture, backup_with_suffix, plan_enabled_search_install,
        validate_noninteractive_enabled_search,
    };
    use crate::cli::{InstallArgs, InstallSearchProfileArg};
    use crate::search_models::{
        ADAPTER_SCHEMA_VERSION, AcceptedLicenses, BALANCED_PROFILE, EMBEDDING_GEMMA_300M,
        ModelArtifactClassification, ModelArtifactRecord, QMD_QUERY_EXPANSION_17B, QMD_RS_VERSION,
        SearchModel,
    };

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

    fn context() -> &'static crate::cli::CliContext {
        const CONTEXT: crate::cli::CliContext = crate::cli::CliContext { verbose: false };
        &CONTEXT
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
