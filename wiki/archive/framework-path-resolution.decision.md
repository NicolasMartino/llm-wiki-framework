# Framework Path Resolution

- Document Class: Decision
- Status: Superseded
- Date: 2026-05-06
- Category: Tooling
- Scope: Resolve framework source paths relative to project-local skill files instead of hardcoding absolute repository paths.
- Sources: review.md, .codex/skills/init-project/SKILL.md, .claude/skills/init-project/SKILL.md, wiki/decisions/project-local-codex-skills.decision.md
- Related: wiki/specs/init-project-skill.spec.md, wiki/archive/project-local-codex-skills.decision.md
- Superseded By: wiki/decisions/llm-wiki-binary-distribution.decision.md

## Choice

Framework skills must resolve the framework root from their own skill file
location rather than embedding a rename-sensitive absolute path.

For project-local skills:

- `.codex/skills/init-project/SKILL.md` resolves the framework root by first
  resolving any global symlink target, then walking three directories up from
  `<framework-root>/.codex/skills/init-project/SKILL.md`.
- `.claude/skills/init-project/SKILL.md` follows the same rule from
  `<framework-root>/.claude/skills/init-project/SKILL.md`.
- The resolved root must contain `project_guidelines.template.md` before it is
  used as the framework source.

Global Codex symlinks may point to project-local skill directories, but the
skill content must not depend on the symlink path itself being the framework
root.

## Why

The project was renamed from `software_project_management` to
`llm_wiki_framework`. Hardcoded absolute paths in the init skills and global
Codex symlinks continued to reference the old project name after the rename.

Resolving paths relative to the skill file makes framework operations robust
to repository renames and moves. Rebuilding symlinks repairs the current
installation; relative path resolution prevents the same class of bug from
returning inside the skill instructions.

## Consequences

- `init-project` uses the framework template from the resolved framework root.
- Repository renames no longer require editing skill instructions just to
  update the framework source path.
- Global symlink targets still need to be rebuilt when the repository moves,
  but dangling symlinks fail noisily and do not create silent content drift.
- Documentation that claims global symlink availability must be verified after
  repository moves.

## Revisit When

- Codex or Claude supports first-class project-local skill discovery without
  global symlinks.
- The framework is packaged so skills are installed from a versioned
  distribution instead of this working tree.
