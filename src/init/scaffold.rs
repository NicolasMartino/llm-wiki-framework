use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::init::answers::Answers;
use crate::init::collision::refuse_framework_collision;
use crate::init::compose::{RenderPlan, compose};
use crate::init::manifest::InitManifest;
use crate::init::runtime::RuntimeManifest;
use crate::init::sources::{copy_initial_sources, validate_initial_sources};
use crate::paths::Paths;
use crate::search_profile::{ProjectSearchConfig, SearchConfig};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum InitMode {
    Fresh,
    Rerun,
}

pub(super) fn create_project(
    path: &Path,
    answers: &Answers,
    plan: &RenderPlan,
    initial_sources: &[PathBuf],
) -> Result<InitMode> {
    let mode = init_mode(path);
    if mode == InitMode::Fresh {
        refuse_framework_collision(path)?;
    }
    validate_initial_sources(initial_sources)?;
    fs::create_dir_all(path).with_context(|| format!("failed to create {}", path.display()))?;

    let output = compose(plan)?;

    for folder in &output.folders {
        fs::create_dir_all(path.join(folder))
            .with_context(|| format!("failed to create {}", path.join(folder).display()))?;
    }

    for file in &output.files {
        if mode == InitMode::Rerun && preserves_project_knowledge(&file.path) {
            let target = path.join(&file.path);
            if target.exists() {
                continue;
            }
        }
        fs::write(path.join(&file.path), &file.contents)
            .with_context(|| format!("failed to write {}", path.join(&file.path).display()))?;
    }

    if let Some(bundle) = copy_initial_sources(path, initial_sources)? {
        println!(
            "Sources copied to `{}`. Run `wiki-ingest` to compile them into the wiki.",
            bundle.display()
        );
    }

    write_manifest(path, answers, output.resolved_packs, mode)?;

    match mode {
        InitMode::Fresh => println!("Initialized LLM Wiki project at {}", path.display()),
        InitMode::Rerun => println!("Updated LLM Wiki project at {}", path.display()),
    }
    Ok(mode)
}

fn init_mode(path: &Path) -> InitMode {
    if path.join(".llm_wiki/init.toml").is_file() {
        InitMode::Rerun
    } else {
        InitMode::Fresh
    }
}

fn preserves_project_knowledge(path: &str) -> bool {
    matches!(path, "wiki/index.md" | "wiki/log.md")
}

fn write_manifest(
    path: &Path,
    answers: &Answers,
    packs: Vec<crate::init::packs::Pack>,
    mode: InitMode,
) -> Result<()> {
    let manifest_dir = path.join(".llm_wiki");
    fs::create_dir_all(&manifest_dir)
        .with_context(|| format!("failed to create {}", manifest_dir.display()))?;
    let manifest = InitManifest::new(
        answers.name.clone(),
        answers.description.clone(),
        answers.blueprint,
        packs,
    )
    .to_toml()?;
    fs::write(manifest_dir.join("init.toml"), manifest).with_context(|| {
        format!(
            "failed to write {}",
            manifest_dir.join("init.toml").display()
        )
    })?;
    let paths = Paths::from_env()?;
    let runtime = RuntimeManifest::from_paths(&paths)?;
    fs::write(manifest_dir.join("runtime.toml"), runtime.to_toml()?).with_context(|| {
        format!(
            "failed to write {}",
            manifest_dir.join("runtime.toml").display()
        )
    })?;
    if mode == InitMode::Fresh || !manifest_dir.join("search.toml").exists() {
        seed_project_search_config(&manifest_dir, &paths, runtime.install_id)?;
    }
    Ok(())
}

fn seed_project_search_config(
    manifest_dir: &Path,
    paths: &Paths,
    source_install_id: Option<String>,
) -> Result<()> {
    let Some(global_config) = SearchConfig::read(&paths.search_config())? else {
        return Ok(());
    };
    let project_config =
        ProjectSearchConfig::from_project_default(&global_config, source_install_id);
    project_config.write_atomic(&manifest_dir.join("search.toml"))?;
    Ok(())
}
