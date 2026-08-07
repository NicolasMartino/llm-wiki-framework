# Wiki Ingest Skill

- Document Class: Spec
- Status: Active
- Date: 2026-05-08
- Category: Tooling
- Scope: Ingest raw source material into typed wiki pages.
- Sources: .claude/skills/wiki-ingest/SKILL.md, wiki/decisions/llm-wiki-binary-distribution.decision.md
- Related: wiki/specs/documentation-model.spec.md, wiki/specs/wiki-research-skill.spec.md

## Contract

`wiki-ingest` compiles explicit raw source material into `wiki/`.
It reads `wiki/index.md`, project guidelines, and relevant existing pages
before drafting updates. It processes one source at a time through extraction,
page drafting, contradiction handling, index updates, and log updates.

Web, URL, or site acquisition is not owned by ingest. Those requests route to
`wiki-research` first so saved source snapshots exist under `raw/`.

## Skill Source

Authored source: `.claude/skills/wiki-ingest/SKILL.md`, a repo-local Claude Code
skill. The MCP-first surface no longer renders runtime variants or globally
installs skills; hosts read and search the wiki through the `llm_wiki_*` MCP
tools while ingesting.

Invocation:

- Claude Code: `/wiki-ingest [source]` (repo-local skill)

## Proven By

- V1 fixture smoke checklist for ingest/query/lint behavior.
