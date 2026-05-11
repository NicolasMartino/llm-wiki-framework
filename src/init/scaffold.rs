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

pub(super) fn create_project(
    path: &Path,
    answers: &Answers,
    plan: &RenderPlan,
    initial_sources: &[PathBuf],
) -> Result<()> {
    refuse_framework_collision(path)?;
    validate_initial_sources(initial_sources)?;
    fs::create_dir_all(path).with_context(|| format!("failed to create {}", path.display()))?;

    let output = compose(plan)?;

    for folder in &output.folders {
        fs::create_dir_all(path.join(folder))
            .with_context(|| format!("failed to create {}", path.join(folder).display()))?;
    }

    for file in &output.files {
        fs::write(path.join(&file.path), &file.contents)
            .with_context(|| format!("failed to write {}", path.join(&file.path).display()))?;
    }

    if let Some(bundle) = copy_initial_sources(path, initial_sources)? {
        println!(
            "Sources copied to `{}`. Run `wiki-ingest` to compile them into the wiki.",
            bundle.display()
        );
    }

    write_manifest(path, answers, output.resolved_packs)?;

    println!("Initialized LLM Wiki project at {}", path.display());
    Ok(())
}

fn write_manifest(
    path: &Path,
    answers: &Answers,
    packs: Vec<crate::init::packs::Pack>,
) -> Result<()> {
    let manifest_dir = path.join(".llm_wiki");
    fs::create_dir_all(&manifest_dir)
        .with_context(|| format!("failed to create {}", manifest_dir.display()))?;
    let manifest = InitManifest::new(answers.blueprint, packs).to_toml()?;
    fs::write(manifest_dir.join("init.toml"), manifest).with_context(|| {
        format!(
            "failed to write {}",
            manifest_dir.join("init.toml").display()
        )
    })?;
    let paths = Paths::from_env()?;
    let runtime = RuntimeManifest::from_paths(&paths)?.to_toml()?;
    fs::write(manifest_dir.join("runtime.toml"), runtime).with_context(|| {
        format!(
            "failed to write {}",
            manifest_dir.join("runtime.toml").display()
        )
    })?;
    Ok(())
}
