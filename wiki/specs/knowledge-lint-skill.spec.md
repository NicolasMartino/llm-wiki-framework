# Knowledge Lint Skill

- Document Class: Spec
- Status: Active
- Date: 2026-05-06
- Category: Tooling
- Scope: Scan the wiki for bookkeeping issues and fix them directly.
- Sources: skills/knowledge-lint/SKILL.md, wiki/decisions/llm-wiki-binary-distribution.decision.md
- Related: wiki/specs/documentation-model.spec.md

## Contract

`knowledge-lint` checks contradictions, stale claims, orphan pages, missing
cross-references, and stale index entries. It fixes mechanical bookkeeping
issues directly and asks the user when substantive contradictions lack a
documented source of truth.

The skill reads `wiki/index.md` first and only reads additional pages needed to
confirm or repair specific issues.

## Runtime Projection

Canonical source: `skills/knowledge-lint/SKILL.md`.
Runtime variants are rendered by `llm-wiki build` and globally installed by
`llm-wiki install`.

Invocation:

- Claude: `/knowledge-lint`
- Codex: `$knowledge-lint` or `$knowledge lint`

## Proven By

- Canonical schema parsing tests.
- Runtime projection snapshots.
- V1 fixture smoke checklist for ingest/query/lint behavior.
