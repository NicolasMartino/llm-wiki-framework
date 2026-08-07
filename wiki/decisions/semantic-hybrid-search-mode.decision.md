# Semantic Hybrid Search Mode

- Document Class: Decision
- Status: Accepted
- Date: 2026-05-12
- Category: Search UX, semantic retrieval, hybrid retrieval
- Scope: Durable mode contract for `llm-wiki search`, `llm-wiki search-all`,
  threshold promotion, and v1 reranker treatment.
- Sources: wiki/proposals/search-query-interpretation.proposal.md, wiki/plans/semantic-hybrid-search.plan.md, wiki/evals/natural-language-search.eval.md, wiki/evals/natural-language-search-impact.md, wiki/plans/idempotent-search-model-install.plan.md
- Related: wiki/decisions/search-backend-selection.decision.md, wiki/specs/documentation-model.spec.md, wiki/specs/wiki-query-skill.spec.md, wiki/references/llm-search-model-licensing.reference.md

## Choice

Adopt the semantic/hybrid search mode contract implemented by the completed
semantic/hybrid search plan.

`llm-wiki search` defaults to `auto`. On a completed LLM-search install profile,
`auto` selects `hybrid` only when the current project has compatible model
artifacts, accepted model-license records, a fresh semantic index, and a
matching scoped threshold record.
When LLM search is disabled or not configured, `auto` selects lexical and
reports that selection. Explicit `semantic` and `hybrid` modes fail closed on
missing readiness unless the caller passes `--allow-lexical-fallback`.

`llm-wiki search-all` resolves the mode independently for every selected
project. Mixed-readiness cross-project searches keep ready projects in the
result set, skip unready projects with per-project readiness metadata, and
report selected mode, fallback, readiness, and result counts in the JSON
`projects` array.

Hybrid v1 uses query expansion, lexical retrieval, semantic retrieval,
threshold gates, rank fusion, and exact-identifier preservation. `--rerank`
remains an opt-in extension that executes the configured reranker when the
reranker artifact and accepted-license record are present. Calibrated reranking
is not part of the v1 hybrid promotion baseline. The current shipped balanced
profile intentionally has no configured reranker, so `--rerank` fails closed
until a future reranker profile has local artifact materialization, accepted
license records, eval evidence, calibration, and a threshold scope.

Semantic indexing depends only on the embedding model artifact and its accepted
license record. Query-expansion and reranker artifacts are runtime requirements
for hybrid and rerank execution, not prerequisites for building semantic vector
metadata.

Thresholds are project-scoped. A threshold record is valid only for its project
or corpus, profile, embedding artifact, embedding dimensions, qmd-rs adapter
schema, qmd-rs version, and chunking strategy. Legacy unscoped threshold files
remain readable only as fallback input when no scoped records exist.

## Validation

The accepted framework baseline is the balanced run
`20260512T154322Z-77945`, with human-accepted labels and post-apply natural
language harness validation. It promotes hybrid and auto to 30 / 0 overall,
20 / 0 on hold-out, 4 / 4 no-match precision, and 4 / 4 exact-identifier
preservation for the framework wiki.

The independent electric-car corpus run `20260512T162918Z-86751` is applied
under its own scope, with hybrid and auto at 20 / 0 overall, 12 / 0 hold-out,
3 / 3 no-match precision, and 5 / 5 exact-identifier preservation. Its higher
semantic floors are evidence that thresholds must not be global across corpora.

## Rationale

The eval showed that lexical search remains necessary for exact identifiers,
but it is not enough for natural-language project questions. Semantic-only
retrieval is useful diagnostic evidence, but it can admit unrelated no-match
topics. Hybrid retrieval is the promoted user-facing path because it combines
lexical anchors, semantic recall, final relevance gates, and exact-identifier
guards.

The final promoted framework floor
`hybrid_final_semantic_floor=0.399904` intentionally sits above the strongest
observed anchor-leaking no-match. This is a calibrated boundary, not a timeless
constant: future corpus, label, retrieval, model, qmd-rs, or chunking changes
must rerun `eval run` and `eval calibrate` before threshold promotion.

## Consequences

- `wiki-query` uses `llm-wiki search --mode auto --format json` for every query
  after reading `wiki/index.md` when the current project is registered, but it
  must inspect selected-mode, readiness, fallback, zero-result, and per-result
  metadata before reading and citing the returned wiki pages.
- If search is unavailable, the project is unregistered, the index is stale, or
  results are not useful, `wiki-query` falls back to index-based navigation
  rather than treating search failure or zero results as an answer.
- Search results remain retrieval results, not synthesized answers. Wiki pages
  remain the citation source.
- Zero results are valid when relevance thresholds reject all candidates.
- Eval `auto` is a readiness-checked hybrid surface. It records readiness
  failures for unready candidates instead of silently measuring lexical
  behavior.
- Managed model and index directories are rebuildable artifacts. On macOS user
  home paths, install/index attempt best-effort Time Machine exclusion for
  those directories.
- Repeated install reuses verified managed model artifacts and repairs
  control-plane records without re-downloading unchanged bytes. Disabling LLM
  search changes readiness immediately but does not delete model files or
  semantic indexes; explicit cleanup is owned by `llm-wiki uninstall
  --search-artifacts` or full uninstall.
- Backend cache access failures are reported as readiness/status metadata, not
  as forced-reindex corruption. Single-project JSON search emits parseable
  `backend_status` for `permission_denied` and exhausted `transient` failures;
  `search-all` reports backend status per project while preserving results
  from ready projects.
- The v1 shipped baseline is calibrated hybrid/auto without a calibrated
  reranker profile. A future reranker profile needs its own artifact/license
  readiness, eval run, calibration report, and threshold scope.

## Revisit When

- A new embedding, query-expansion, reranker, chunking strategy, qmd-rs version,
  or adapter schema is adopted.
- A project wants a calibrated reranker profile as part of the default hybrid
  path.
- No-match sentinels become valid project topics or new corpus topics create
  higher anchor-leak scores.
- Cross-machine numeric drift proves material for the same model artifacts and
  corpus.
