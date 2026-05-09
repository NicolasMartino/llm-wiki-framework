# Wiki Query Skill

- Document Class: Spec
- Status: Active
- Date: 2026-05-08
- Category: Tooling
- Scope: Query the project wiki and answer with citations.
- Sources: assets/skills/wiki-query/SKILL.md, wiki/decisions/llm-wiki-binary-distribution.decision.md
- Related: wiki/specs/documentation-model.spec.md, wiki/specs/wiki-ingest-skill.spec.md

## Contract

`wiki-query` answers from the compiled wiki. It reads `wiki/index.md`
first, reads relevant pages, cites wiki paths, flags gaps and contradictions,
and offers save-back when a synthesized answer creates durable knowledge.

The skill must not browse the filesystem for project knowledge. The index is
the entry point. QMD may supplement navigation when available and useful.

## Runtime Projection

Canonical source: `assets/skills/wiki-query/SKILL.md`.
Claude and Codex runtime variants are rendered by the `llm-wiki` binary.
Codex also receives `agents/openai.yaml` from
`assets/skills/wiki-query/codex/openai.yaml`.

Invocation:

- Claude: `/wiki-query <question>`
- Codex: `$wiki-query <question>` or `$wiki query <question>`

## Proven By

- Canonical schema parsing tests.
- Runtime projection snapshots.
- `llm-wiki build --out .` regenerated committed runtime outputs.
