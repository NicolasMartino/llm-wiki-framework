# Wiki Query Skill

- Document Class: Spec
- Status: Active
- Date: 2026-05-08
- Category: Tooling
- Scope: Query the project wiki and answer with citations.
- Sources: assets/skills/wiki-query/SKILL.md, wiki/decisions/llm-wiki-binary-distribution.decision.md, wiki/decisions/semantic-hybrid-search-mode.decision.md
- Related: wiki/specs/documentation-model.spec.md, wiki/specs/wiki-ingest-skill.spec.md, wiki/evals/natural-language-search.eval.md

## Contract

`wiki-query` answers from the compiled wiki. It reads `wiki/index.md`
first, reads relevant pages, cites wiki paths, flags gaps and contradictions,
and offers save-back when a synthesized answer creates durable knowledge.

The skill must not browse the filesystem for project knowledge. The index is
the entry point. `{llm_wiki_binary} search` may supplement navigation when the
current project is registered and the index is not enough.

For large or unclear queries, the projected skill uses
`{llm_wiki_binary} search --mode auto --format json "<question>"` as a
navigation supplement. It must inspect `selected_mode`, `readiness_reason`,
`fallback_reason`, `zero_result_reason`, and per-result mode/backend metadata
before trusting the result set. Search snippets are navigation aids only; the
skill still reads and cites the returned wiki pages directly.

If semantic/hybrid readiness is missing or search returns no useful result, the
skill continues from index-based navigation instead of treating the search
failure or zero-result outcome as an answer.

If a future skill projection uses `search-all` as an explicit cross-project
navigation supplement, it must inspect the JSON `projects` array as well as the
top-level mode fields so skipped projects, lexical fallback, and mixed
readiness do not get mistaken for corpus-wide absence.

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
