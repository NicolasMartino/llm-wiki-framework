# Knowledge Ingest Skill

- Document Class: Spec
- Status: Active
- Date: 2026-05-06
- Category: Tooling
- Scope: Ingest raw source material into typed wiki pages.
- Sources: assets/skills/knowledge-ingest/SKILL.md, wiki/decisions/llm-wiki-binary-distribution.decision.md
- Related: wiki/specs/documentation-model.spec.md, wiki/specs/knowledge-research-skill.spec.md

## Contract

`knowledge-ingest` compiles explicit raw source material into `wiki/`.
It reads `wiki/index.md`, project guidelines, and relevant existing pages
before drafting updates. It processes one source at a time through extraction,
page drafting, contradiction handling, index updates, and log updates.

Web, URL, or site acquisition is not owned by ingest. Those requests route to
`knowledge-research` first so saved source snapshots exist under `raw/`.

## Runtime Projection

Canonical source: `assets/skills/knowledge-ingest/SKILL.md`.
Runtime variants are rendered by `llm-wiki build` and globally installed by
`llm-wiki install`.

Invocation:

- Claude: `/knowledge-ingest [source]`
- Codex: `$knowledge-ingest [source]` or `$knowledge ingest [source]`

## Proven By

- Canonical schema parsing tests.
- Runtime projection snapshots.
- V1 fixture smoke checklist for ingest/query/lint behavior.
