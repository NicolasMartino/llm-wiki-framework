# Wiki Query Search-First Retrieval

- Document Class: Decision
- Status: Accepted
- Date: 2026-05-23
- Category: Query workflow, search dogfooding
- Scope: `wiki-query` retrieval order for project questions.
- Sources: conversational input 2026-05-23, assets/skills/wiki-query/SKILL.md, wiki/specs/wiki-query-skill.spec.md, wiki/decisions/semantic-hybrid-search-mode.decision.md, wiki/specs/documentation-model.spec.md
- Related: wiki/specs/wiki-query-skill.spec.md, wiki/specs/documentation-model.spec.md, wiki/decisions/semantic-hybrid-search-mode.decision.md, wiki/evals/natural-language-search.eval.md

## Decision

`wiki-query` must attempt `llm-wiki search --mode auto --format json
"<question>"` for every query after reading `wiki/index.md`, when the current
project is registered.

The index remains the mandatory first-read orientation and catalog. Search is
the default retrieval pass, not a fallback reserved for large or unclear
queries. The skill still reads returned wiki pages directly before answering,
and wiki pages remain the citation source.

If search is unavailable, the project is unregistered, the search index is
missing or stale, semantic/hybrid readiness is missing, or the result set is not
useful, `wiki-query` falls back to index-based navigation. A search failure or
zero-result outcome is retrieval metadata, not an answer by itself.

## Rationale

The framework built qmd-rs-backed lexical search and calibrated auto/hybrid
search so agents can use it. Keeping search as a rare large-wiki fallback would
under-dogfood the product and increase the chance that a relevant page is missed
when the index summary is too compressed or ambiguous.

The search-first contract preserves the index discipline while making retrieval
more complete:

- `wiki/index.md` still prevents blind filesystem browsing and gives the agent
  project shape, document classes, and known page inventory.
- `llm-wiki search --mode auto --format json` exercises the validated
  retrieval stack on every query.
- JSON metadata forces the agent to distinguish useful retrieval from
  readiness failures, lexical fallback, stale indexes, and honest zero-result
  outcomes.
- Search snippets remain navigation aids only, so answer synthesis still comes
  from directly read wiki pages.

## Consequences

- Canonical and projected `wiki-query` skills must describe search as an
  always-attempted retrieval pass for registered projects.
- `wiki-query` answers should mention search readiness gaps only when they
  materially affect confidence or explain why index navigation was used.
- Projects that want complete query dogfooding should stay registered and keep
  search indexes fresh.
- The fallback path remains required because new, unregistered, stale, or
  partially configured projects still need to answer from the wiki.

## Revisit When

- `llm-wiki search` becomes available without prior project registration.
- `search-all` becomes part of the default `wiki-query` workflow.
- Search latency or readiness churn proves too expensive for ordinary query
  interactions.
