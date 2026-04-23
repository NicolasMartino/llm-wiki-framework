# Project-Local Codex Skills

- Document Class: Decision
- Status: Accepted
- Date: 2026-04-23
- Category: Tooling
- Scope: Keep Codex skill definitions inside this framework repo and expose them globally through symlinks.
- Sources: .claude/skills/init-project/SKILL.md, .claude/skills/knowledge-query/SKILL.md, .claude/skills/knowledge-ingest/SKILL.md
- Related: wiki/specs/init-project-skill.spec.md, wiki/specs/knowledge-query-skill.spec.md, wiki/specs/knowledge-ingest-skill.spec.md

## Choice

Maintain Codex translations of the framework skills under `.codex/skills/`
inside this repository, and expose them to Codex through symlinks from
`~/.codex/skills/`.

Current symlinks:

- `~/.codex/skills/init-project` -> `.codex/skills/init-project`
- `~/.codex/skills/knowledge` -> `.codex/skills/knowledge`
- `~/.codex/skills/knowledge-query` -> `.codex/skills/knowledge-query`
- `~/.codex/skills/knowledge-ingest` -> `.codex/skills/knowledge-ingest`
- `~/.codex/skills/knowledge-lint` -> `.codex/skills/knowledge-lint`
- `~/.codex/skills/knowledge-research` -> `.codex/skills/knowledge-research`

## Why

The framework is the product, so the skills that operate the framework should
version with the framework source. Keeping canonical skill files in the repo
makes changes reviewable, dogfoodable, and tied to wiki updates.

Global symlinks preserve normal Codex discovery without copying skill content
into `~/.codex/skills/`. This avoids drift between the project-local source of
truth and the globally available skill definitions.

## Consequences

- `.codex/skills/` is now a project-owned tooling surface.
- Updating a project-local Codex skill immediately updates the globally exposed
  skill through the symlink.
- Symlink targets must remain valid when the repo is moved.
- The Claude and Codex skill variants need to be kept behaviorally aligned
  when framework workflows change.

## Revisit When

- Codex supports first-class project-local skill discovery without symlinks.
- The framework is packaged for distribution beyond this local repo.
- Skill variants begin to diverge enough that a generator or shared source is
  needed.
