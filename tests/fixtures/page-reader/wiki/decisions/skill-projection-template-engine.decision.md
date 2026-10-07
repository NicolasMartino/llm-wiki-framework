# Skill Projection Template Engine

- Document Class: Decision
- Status: Superseded
- Date: 2026-05-09
- Category: Tooling, skill projection, internal architecture
- Scope: Runtime skill projection for Claude and Codex uses the shared Askama template engine instead of hand-rolled Rust string concatenation.
- Sources: wiki/proposals/skills-template-engine.proposal.md, crates/llm-wiki-schema/src/projector/{claude,codex,format,idiom,types}.rs, templates/skills/
- Superseded By: wiki/plans/mcp-first-agent-surface.plan.md
- Related: wiki/decisions/composable-project-init.decision.md, wiki/references/askama-template-engine.reference.md, wiki/plans/skill-projection-template-engine.plan.md

## Choice

Migrate skill projection onto the same compile-time Askama engine used by
composable project init.

## Supersession - 2026-06-22

This decision is superseded by the MCP-first no-legacy cutover. The framework no
longer installs, builds, renders, snapshots, or stores generated Claude/Codex
skill projection artifacts. Agent-facing guidance now lives in project
instructions plus the Rust-native MCP server: tools, resources, advisory
prompts, and initialize instructions. The schema crate remains only for skill
document parsing/validation tests that still describe historical source
material; it no longer exports Claude/Codex projectors.

Claude and Codex skill markdown now render from `templates/skills/` through
typed projector contexts in `crates/llm-wiki-schema`. The runtime-specific
projectors still own runtime support checks and idiom conversion, but section
ordering, frontmatter shape, and rendered markdown structure are template
authority.

Codex runtime config also renders through a typed YAML template instead of
ad-hoc `{skill_name}` string replacement. Per-skill Codex interface metadata
is parsed into `CodexRuntimeConfig`, then rendered through
`templates/skills/codex_runtime_config.yaml`.

## Why

The D10 init migration proved Askama works for framework-generated text. Skill
projection was the remaining second rendering pipeline. Moving both generated
project files and runtime skill files onto one engine makes rendered output
easier to review and keeps future section/layout edits in templates instead of
parallel Rust string builders.

## Consequences

- `crates/llm-wiki-schema` now depends on Askama because projection is part of
  the schema crate's contract.
- `templates/skills/` is the canonical place to inspect projected skill
  structure.
- Golden snapshots remain the compatibility contract; the migration was
  verified without snapshot churn for real skills.
- The old `CodexProjector::with_runtime_config_template` API is replaced by a
  typed `CodexRuntimeConfig` input.
