use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::Utc;

use crate::init::answers::Answers;
use crate::init::collision::refuse_framework_collision;
use crate::init::profile::ProjectProfile;
use crate::init::sources::copy_initial_sources;
use crate::init::template::{render_agent_template, render_project_guidelines};

pub(super) fn create_project(
    path: &Path,
    answers: &Answers,
    profile: &ProjectProfile,
    initial_sources: &[PathBuf],
) -> Result<()> {
    fs::create_dir_all(path).with_context(|| format!("failed to create {}", path.display()))?;
    refuse_framework_collision(path)?;

    let project_guidelines =
        render_project_guidelines(&answers.name, &answers.description, profile)?;
    let agents = render_agent_template(&answers.name, &answers.description, profile)?;

    create_wiki_dirs(path, profile)?;
    create_code_dirs(path, profile)?;

    fs::write(path.join("project_guidelines.md"), project_guidelines)?;
    fs::write(path.join("AGENTS.md"), agents)?;
    fs::write(path.join("CLAUDE.md"), "See @AGENTS.md.\n")?;
    fs::write(path.join("wiki/index.md"), index_md(&answers.name, profile))?;
    fs::write(path.join("wiki/log.md"), log_md(&answers.name))?;

    if let Some(bundle) = copy_initial_sources(path, initial_sources)? {
        println!(
            "Sources copied to `{}`. Run `knowledge-ingest` to compile them into the wiki.",
            bundle.display()
        );
    }

    println!("Initialized LLM Wiki project at {}", path.display());
    Ok(())
}

fn create_wiki_dirs(path: &Path, profile: &ProjectProfile) -> Result<()> {
    for dir in [
        "raw",
        "wiki/specs",
        "wiki/decisions",
        "wiki/proposals",
        "wiki/roadmaps",
        "wiki/plans",
        "wiki/checklists",
        "wiki/references",
        "wiki/archive",
    ] {
        fs::create_dir_all(path.join(dir))
            .with_context(|| format!("failed to create {}", path.join(dir).display()))?;
    }
    if profile.include_ml_ai {
        for dir in ["wiki/experiments", "wiki/evals"] {
            fs::create_dir_all(path.join(dir))
                .with_context(|| format!("failed to create {}", path.join(dir).display()))?;
        }
    }
    Ok(())
}

fn create_code_dirs(path: &Path, profile: &ProjectProfile) -> Result<()> {
    if !profile.is_existing {
        for dir in ["src", "tests", "scripts", "infra"] {
            fs::create_dir_all(path.join(dir))
                .with_context(|| format!("failed to create {}", path.join(dir).display()))?;
        }
    }
    Ok(())
}

fn index_md(project_name: &str, profile: &ProjectProfile) -> String {
    let mut sections = vec![
        "Specs",
        "Decisions",
        "Roadmaps",
        "References",
        "Proposals",
        "Plans",
        "Checklists",
    ];
    if profile.include_ml_ai {
        sections.push("Experiments");
        sections.push("Evals");
    }
    let mut output = format!(
        "# Wiki Index\n\nProject: {project_name}\nStage: Bootstrap\nUpdated: {}\n\n",
        Utc::now().date_naive()
    );
    for section in sections {
        output.push_str(&format!("## {section}\n\n(none yet)\n\n"));
    }
    output.push_str("## Archive\n\n(none yet)\n");
    output
}

fn log_md(project_name: &str) -> String {
    format!(
        "# Wiki Log\n\n## [{}] create | project bootstrap\n\nInitialized `{project_name}` with the LLM Wiki framework.\n",
        Utc::now().date_naive()
    )
}
