# Knowledge Research Skill

- Document Class: Spec
- Status: Active
- Date: 2026-05-06
- Category: Tooling
- Scope: Gather source material into `raw/research/` before ingest.
- Sources: assets/skills/knowledge-research/SKILL.md, wiki/decisions/llm-wiki-binary-distribution.decision.md
- Related: wiki/specs/knowledge-ingest-skill.spec.md, wiki/specs/documentation-model.spec.md

## Contract

`knowledge-research` is the guided intake surface for source acquisition. It
creates a research bundle under `raw/research/YYYY-MM-DD-topic-slug/` with a
manifest, summary, and saved source files. It supports path, URL, site, and web
modes. It does not write durable knowledge directly into `wiki/`.

Ingest remains a separate operation and only runs when the user explicitly
asks for it.

## Runtime Projection

Canonical source: `assets/skills/knowledge-research/SKILL.md`.
Runtime variants are rendered by `llm-wiki build` and globally installed by
`llm-wiki install`.

Invocation:

- Claude: `/knowledge-research <request>`
- Codex: `$knowledge-research <request>` or `$knowledge research <request>`

## Proven By

- Canonical schema parsing tests.
- Runtime projection snapshots.
- Research-first routing in `knowledge-ingest`.
