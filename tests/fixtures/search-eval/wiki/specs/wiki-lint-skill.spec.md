# Wiki Lint Skill

- Document Class: Spec
- Status: Active
- Date: 2026-05-08
- Category: Tooling
- Scope: Scan the wiki for bookkeeping issues and fix them directly.
- Sources: .claude/skills/wiki-lint/SKILL.md, wiki/decisions/llm-wiki-binary-distribution.decision.md
- Related: wiki/specs/documentation-model.spec.md

## Contract

`wiki-lint` checks contradictions, stale claims, orphan pages, missing
cross-references, and stale index entries. It fixes mechanical bookkeeping
issues directly and asks the user when substantive contradictions lack a
documented source of truth.

The skill reads `wiki/index.md` first and only reads additional pages needed to
confirm or repair specific issues.

## Skill Source

Authored source: `.claude/skills/wiki-lint/SKILL.md`, a repo-local Claude Code
skill. The MCP-first surface no longer renders runtime variants or globally
installs skills; hosts read and search the wiki through the `llm_wiki_*` MCP
tools while linting.

Invocation:

- Claude Code: `/wiki-lint` (repo-local skill)

## Proven By

- V1 fixture smoke checklist for ingest/query/lint behavior.
