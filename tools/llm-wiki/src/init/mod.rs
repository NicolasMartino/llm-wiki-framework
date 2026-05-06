pub mod collision;
pub mod profile;
pub mod sources;
pub mod template;

use std::fs;
use std::io::{self, Write};
use std::path::Path;

use anyhow::{Context, Result, bail};
use chrono::Utc;

use crate::cli::InitArgs;
use crate::embed;
use crate::init::collision::refuse_framework_collision;
use crate::init::profile::ProjectProfile;
use crate::init::sources::copy_initial_sources;
use crate::init::template::{render_agent_template, render_project_guidelines};

pub fn run(args: &InitArgs) -> Result<()> {
    let answers = if args.non_interactive {
        Answers {
            name: required_flag("--name", &args.name)?,
            description: required_flag("--description", &args.description)?,
            project_type: required_flag("--type", &args.project_type)?,
            scale: required_flag("--scale", &args.scale)?,
            existing: args.existing,
        }
    } else {
        interactive_answers(args)?
    };
    let profile = ProjectProfile::resolve(&answers.project_type, &answers.scale, answers.existing)?;
    create_project(&args.path, &answers, &profile, &args.initial_sources)
}

fn create_project(
    path: &Path,
    answers: &Answers,
    profile: &ProjectProfile,
    initial_sources: &[std::path::PathBuf],
) -> Result<()> {
    fs::create_dir_all(path).with_context(|| format!("failed to create {}", path.display()))?;
    refuse_framework_collision(path)?;

    let project_guidelines = render_project_guidelines(
        embed::PROJECT_GUIDELINES_TEMPLATE,
        &answers.name,
        &answers.description,
        profile,
    );
    let claude = render_agent_template(
        embed::CLAUDE_TEMPLATE,
        &answers.name,
        &answers.description,
        profile,
    );

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
    if !profile.is_existing {
        for dir in ["src", "tests", "scripts", "infra"] {
            fs::create_dir_all(path.join(dir))
                .with_context(|| format!("failed to create {}", path.join(dir).display()))?;
        }
    }

    fs::write(path.join("project_guidelines.md"), project_guidelines)?;
    fs::write(path.join("CLAUDE.md"), claude)?;
    fs::write(path.join("wiki/index.md"), index_md(&answers.name, profile))?;
    fs::write(path.join("wiki/log.md"), log_md(&answers.name))?;
    append_gitignore(path)?;

    if let Some(bundle) = copy_initial_sources(path, initial_sources)? {
        println!(
            "Sources copied to `{}`. Run `knowledge-ingest` to compile them into the wiki.",
            bundle.display()
        );
    }

    println!("Initialized LLM Wiki project at {}", path.display());
    Ok(())
}

fn required_flag(name: &'static str, value: &Option<String>) -> Result<String> {
    value
        .clone()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| anyhow::anyhow!("{name} is required with --non-interactive"))
}

fn interactive_answers(args: &InitArgs) -> Result<Answers> {
    Ok(Answers {
        name: args.name.clone().unwrap_or(prompt("Project name")?),
        description: args
            .description
            .clone()
            .unwrap_or(prompt("One-sentence description")?),
        project_type: args
            .project_type
            .clone()
            .unwrap_or(prompt("Project type (web/api/cli/ml/data/lib/other)")?),
        scale: args
            .scale
            .clone()
            .unwrap_or(prompt("Scale (small/medium/large)")?),
        existing: args.existing,
    })
}

fn prompt(label: &str) -> Result<String> {
    print!("{label}: ");
    io::stdout().flush()?;
    let mut value = String::new();
    io::stdin().read_line(&mut value)?;
    let value = value.trim().to_string();
    if value.is_empty() {
        bail!("{label} is required");
    }
    Ok(value)
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

fn append_gitignore(path: &Path) -> Result<()> {
    let gitignore = path.join(".gitignore");
    let mut existing = if gitignore.exists() {
        fs::read_to_string(&gitignore)?
    } else {
        String::new()
    };
    for entry in [".wiki/", "*.sqlite", ".cache/"] {
        if !existing.lines().any(|line| line == entry) {
            if !existing.ends_with('\n') && !existing.is_empty() {
                existing.push('\n');
            }
            existing.push_str(entry);
            existing.push('\n');
        }
    }
    fs::write(gitignore, existing)?;
    Ok(())
}

struct Answers {
    name: String,
    description: String,
    project_type: String,
    scale: String,
    existing: bool,
}
