# Init Project Skill

- Document Class: Spec
- Status: Active
- Date: 2026-04-23
- Category: Tooling
- Scope: The /init-project skill that creates or updates projects using the LLM Wiki framework.
- Sources: wiki/references/niharshrotri-llm-wiki.reference.md
- Related: wiki/specs/documentation-model.spec.md, wiki/decisions/three-layer-architecture.decision.md

## What It Does

`/init-project [path]` is a Claude Code skill that scaffolds a new project
or updates an existing one with the LLM Wiki project management framework.

Two modes:
- **Create**: asks questions, generates tailored project_guidelines.md + CLAUDE.md,
  scaffolds folder structure, creates index.md and log.md, optionally ingests
  initial raw sources.
- **Update**: compares existing project against latest template, proposes and
  applies changes.

## Location

Skill definition: `.claude/skills/init-project.md`
Template: `project_guidelines.template.md`

For global access, symlink the skill to `~/.claude/skills/`:
```bash
ln -s /path/to/framework/.claude/skills/init-project.md ~/.claude/skills/init-project.md
```

## Question Flow

1. Project name
2. One-sentence description
3. Project type (web, api, cli, ml, data, lib, other)
4. New or existing codebase
5. Expected scale (small, medium, large)
6. Initial raw sources to ingest (optional)

## Project Profiles

Answers determine which template sections are included:

| Flag | Trigger | Effect |
| --- | --- | --- |
| INCLUDE_ML_AI | project type is ml or data | Adds experiment/eval doc types, models/, data/, notebooks/, evals/ |
| INCLUDE_QMD | scale is medium or large | Adds QMD search section and setup instructions |
| IS_EXISTING | adding to existing codebase | Skips creating src/, tests/, etc. |

## What Gets Generated

1. `project_guidelines.md` — from template, with conditional sections resolved
2. `CLAUDE.md` — project-specific agent schema
3. `raw/` — empty, ready for sources
4. `wiki/` — full structure with index.md and log.md
5. `.gitignore` — excludes .wiki/, sqlite, cache
6. Conditional: ML/AI folders, QMD setup instructions

## Proven By

- Skill file exists at `.claude/skills/init-project.md`
- Template file exists at `project_guidelines.template.md`
- Not yet tested on a real project spawn (see roadmap D5)

## Limitations

- Template path is hardcoded to this repo's location
- Update mode not yet tested
- No automated QMD setup (user runs commands manually)
- Symlink to global skills must be done manually
