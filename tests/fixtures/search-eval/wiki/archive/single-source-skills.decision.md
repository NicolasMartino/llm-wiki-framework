# Single Source Skills

- Document Class: Decision
- Status: Superseded
- Date: 2026-05-06
- Category: Tooling
- Scope: Maintain one canonical skill source tree and generate Claude/Codex runtime skill outputs from it.
- Sources: wiki/archive/single-source-skills.plan.md, skills/README.md, wiki/archive/project-local-codex-skills.decision.md
- Related: wiki/specs/init-project-skill.spec.md, wiki/specs/knowledge-ingest-skill.spec.md, wiki/specs/knowledge-query-skill.spec.md, wiki/specs/knowledge-research-skill.spec.md, wiki/specs/knowledge-lint-skill.spec.md, wiki/archive/framework-path-resolution.decision.md
- Superseded By: wiki/decisions/llm-wiki-binary-distribution.decision.md

## Choice

The framework uses `skills/` at the repository root as the canonical source
for framework skill definitions.

Runtime-specific skill directories are generated:

- `.claude/skills/`
- `.codex/skills/`

`legacy skill render script` renders canonical `skills/<name>/SKILL.md` files by keeping
shared content and selecting runtime-specific conditional blocks:

- `legacy Claude runtime marker ... legacy end marker`
- `legacy Codex runtime marker ... legacy end marker`

Codex UI metadata lives under `skills/<name>/codex/openai.yaml` and is copied
to `.codex/skills/<name>/agents/openai.yaml`.

Generated runtime outputs remain committed for first-clone usability.

## Why

The previous model maintained Claude and Codex skill definitions directly in
two separate trees. That created drift: invocation wording diverged, URL/web
ingest behavior diverged, and `knowledge-lint` existed only for Codex.

A canonical source tree removes the need to edit shared behavior twice while
still allowing legitimate runtime differences in invocation syntax,
frontmatter descriptions, and Codex-only UI metadata.

## Alternatives Considered

1. **Keep status quo.** Rejected because drift already occurred across shared
   skills.
2. **Pure symlinks from both runtimes to one skill tree.** Rejected because
   Claude and Codex need different invocation surfaces and Codex-only
   `agents/openai.yaml` metadata.
3. **MCP-based skill exposure.** Deferred because it is a runtime integration
   question larger than this consolidation.

## Consequences

- Edit `skills/<name>/SKILL.md`, not the generated runtime outputs.
- Run `bash legacy skill render script` after changing canonical skills.
- Claude and Codex now both have `knowledge-lint`.
- The Codex-only `knowledge` dispatcher remains generated only for Codex.
- Generated outputs should match canonical source after every skill edit.

## Revisit When

- Conditional blocks become hard to read or maintain.
- Codex or Claude supports a shared project-local skill format directly.
- Generated output diffs become too noisy to commit.
