use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use llm_wiki_schema::{ClaudeProjector, CodexProjector, Projector, Runtime, parse};

use crate::cli::{BuildArgs, BuildTarget};
use crate::embed;

pub fn run(args: &BuildArgs) -> Result<()> {
    for asset in embed::SKILLS {
        let doc = parse(asset.skill_md)
            .with_context(|| format!("failed to parse embedded skill {}", asset.name))?;

        if matches!(args.target, BuildTarget::Claude | BuildTarget::Both)
            && doc.frontmatter.runtimes.contains(&Runtime::Claude)
        {
            let rendered = ClaudeProjector
                .project(&doc)
                .with_context(|| format!("failed to render {} for Claude", asset.name))?;
            write_file(
                &args
                    .out
                    .join(".claude/skills")
                    .join(asset.name)
                    .join("SKILL.md"),
                &rendered.skill_md,
            )?;
        }

        if matches!(args.target, BuildTarget::Codex | BuildTarget::Both)
            && doc.frontmatter.runtimes.contains(&Runtime::Codex)
        {
            let rendered = CodexProjector::with_runtime_config_template(asset.codex_openai)
                .project(&doc)
                .with_context(|| format!("failed to render {} for Codex", asset.name))?;
            let skill_dir = args.out.join(".codex/skills").join(asset.name);
            write_file(&skill_dir.join("SKILL.md"), &rendered.skill_md)?;
            write_file(
                &skill_dir.join("agents/openai.yaml"),
                rendered
                    .runtime_config
                    .as_deref()
                    .context("missing Codex runtime config")?,
            )?;
        }
    }
    Ok(())
}

fn write_file(path: &Path, contents: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    fs::write(path, contents).with_context(|| format!("failed to write {}", path.display()))
}
