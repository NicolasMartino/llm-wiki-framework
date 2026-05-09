# Wiki Research Skill

- Document Class: Spec
- Status: Active
- Date: 2026-05-08
- Category: Tooling
- Scope: Gather source material into `raw/research/` before ingest.
- Sources: assets/skills/wiki-research/SKILL.md, wiki/decisions/llm-wiki-binary-distribution.decision.md
- Related: wiki/specs/wiki-ingest-skill.spec.md, wiki/specs/documentation-model.spec.md

## Contract

`wiki-research` is the guided intake surface for source acquisition. It
creates a research bundle under `raw/research/YYYY-MM-DD-topic-slug/` with a
manifest, summary, and saved source files. It supports path, URL, site, and web
modes. It does not write durable knowledge directly into `wiki/`.

Ingest remains a separate operation and only runs when the user explicitly
asks for it.

## Runtime Projection

Canonical source: `assets/skills/wiki-research/SKILL.md`.
Runtime variants are rendered by `llm-wiki build` and globally installed by
`llm-wiki install`.

Invocation:

- Claude: `/wiki-research <request>`
- Codex: `$wiki-research <request>` or `$wiki research <request>`

## Proven By

- Canonical schema parsing tests.
- Runtime projection snapshots.
- Research-first routing in `wiki-ingest`.
