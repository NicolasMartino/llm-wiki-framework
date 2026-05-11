use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use llm_wiki_schema::{
    ClaudeProjector, CodexProjector, CodexRuntimeConfig, Projector, Runtime, parse,
};

use crate::cli::{BuildArgs, BuildTarget};
use crate::embed;
use crate::skill_render::apply_binary_context;

pub fn run(args: &BuildArgs, context: &crate::cli::CliContext) -> Result<()> {
    context.diagnostic("command: build");
    context.diagnostic(format!("build target: {}", build_target_label(args.target)));
    context.diagnostic(format!("output directory: {}", args.out.display()));
    let mut claude_count = 0usize;
    let mut codex_count = 0usize;

    for asset in embed::SKILLS {
        let doc = parse(asset.skill_md)
            .with_context(|| format!("failed to parse embedded skill {}", asset.name))?;
        let doc = apply_binary_context(doc, "llm-wiki");

        if matches!(args.target, BuildTarget::Claude | BuildTarget::Both)
            && doc.frontmatter.runtimes.contains(&Runtime::Claude)
        {
            let path = args
                .out
                .join(".claude/skills")
                .join(asset.name)
                .join("SKILL.md");
            context.diagnostic(format!(
                "render target: skill={} runtime=claude path={}",
                asset.name,
                path.display()
            ));
            let rendered = ClaudeProjector
                .project(&doc)
                .with_context(|| format!("failed to render {} for Claude", asset.name))?;
            write_file(&path, &rendered.skill_md)?;
            claude_count += 1;
        }

        if matches!(args.target, BuildTarget::Codex | BuildTarget::Both)
            && doc.frontmatter.runtimes.contains(&Runtime::Codex)
        {
            let runtime_config = CodexRuntimeConfig::from_yaml(asset.codex_openai)
                .with_context(|| format!("failed to parse {} Codex runtime config", asset.name))?;
            let rendered = CodexProjector::with_runtime_config(runtime_config)
                .project(&doc)
                .with_context(|| format!("failed to render {} for Codex", asset.name))?;
            let skill_dir = args.out.join(".codex/skills").join(asset.name);
            context.diagnostic(format!(
                "render target: skill={} runtime=codex path={}",
                asset.name,
                skill_dir.join("SKILL.md").display()
            ));
            write_file(&skill_dir.join("SKILL.md"), &rendered.skill_md)?;
            write_file(
                &skill_dir.join("agents/openai.yaml"),
                rendered
                    .runtime_config
                    .as_deref()
                    .context("missing Codex runtime config")?,
            )?;
            codex_count += 1;
        }
    }
    context.diagnostic(format!(
        "rendered runtime skill counts: claude={claude_count}, codex={codex_count}"
    ));
    Ok(())
}

fn build_target_label(target: BuildTarget) -> &'static str {
    match target {
        BuildTarget::Claude => "claude",
        BuildTarget::Codex => "codex",
        BuildTarget::Both => "both",
    }
}

fn write_file(path: &Path, contents: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    fs::write(path, contents).with_context(|| format!("failed to write {}", path.display()))
}
