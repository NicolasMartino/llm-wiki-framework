# Wiki Research Skill

- Document Class: Spec
- Status: Active
- Date: 2026-05-08
- Category: Tooling
- Scope: Gather source material into `raw/research/` before ingest.
- Sources: .claude/skills/wiki-research/SKILL.md, wiki/decisions/llm-wiki-binary-distribution.decision.md
- Related: wiki/specs/wiki-ingest-skill.spec.md, wiki/specs/documentation-model.spec.md

## Contract

`wiki-research` is the guided intake surface for source acquisition. It
creates a research bundle under `raw/research/YYYY-MM-DD-topic-slug/` with a
manifest, summary, and saved source files. It supports path, URL, site, and web
modes. It does not write durable knowledge directly into `wiki/`.

Ingest remains a separate operation and only runs when the user explicitly
asks for it.

## Skill Source

Authored source: `.claude/skills/wiki-research/SKILL.md`, a repo-local Claude
Code skill. The MCP-first surface no longer renders runtime variants or globally
installs skills; hosts read and search the wiki through the `llm_wiki_*` MCP
tools while gathering sources.

Invocation:

- Claude Code: `/wiki-research <request>` (repo-local skill)

## Proven By

- Research-first routing in `wiki-ingest`.
