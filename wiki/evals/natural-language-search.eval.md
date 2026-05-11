# Natural-Language Search Eval

- Document Class: Eval
- Status: Planned
- Date: 2026-05-11
- Category: Search UX, semantic retrieval, hybrid retrieval
- Scope: Stage 0 natural-language eval suite, expected target draft, calibration
  split, threshold methodology, and observability evidence for semantic/hybrid
  `llm-wiki search`.
- Sources: wiki/plans/semantic-hybrid-search.plan.md, wiki/proposals/search-query-interpretation.proposal.md, wiki/evals/search-backend-selection.eval.md, wiki/references/llm-search-model-licensing.reference.md, wiki/checklists/observability-contract.checklist.md, raw/research/2026-05-11-llm-search-model-licensing/research-summary.md
- Related: wiki/plans/semantic-hybrid-search.plan.md, wiki/proposals/search-query-interpretation.proposal.md, wiki/references/llm-search-model-licensing.reference.md, wiki/references/qmd-rs-search-crate.reference.md, wiki/evals/search-backend-selection.eval.md

## Objective

This eval gates the first semantic/hybrid search implementation. It checks that
natural-language recall improves without regressing exact identifier precision
or honest no-result behavior.

The expected targets below are agent-drafted labels. Human approval is required
before threshold tuning or implementation gating can use them.

## Labeling And Gate Status

- Labeling owner: project human maintainer.
- Agent role: draft queries, expected targets, and pass/fail rules.
- Human target approval: pending.
- Seeded thresholds (see below) unlock runtime execution so the first
  end-to-end eval run can produce observed scores. Target labels and threshold
  values are both expected to be revised once observed results land.

## Calibration Identity

First calibration identity fields:

- qmd-rs version: `0.3.2`.
- Adapter schema version: `1`.
- Embedding model: `ggml-org/embeddinggemma-300M-GGUF` /
  `embeddinggemma-300M-Q8_0.gguf`.
- Embedding artifact SHA-256:
  `f470220f84b6235197541352d22f10bf00098a8242c18eaacea9c8a4add557bc`.
- Embedding dimensions: 768 from the upstream EmbeddingGemma model card.
- Query-expansion model: `tobil/qmd-query-expansion-1.7B-gguf` /
  `qmd-query-expansion-1.7B-q4_k_m.gguf`.
- Query-expansion artifact SHA-256:
  `000dfb1c06efa6a049e9f64ba921c3740e2454f62abab6fa10e77bd30bb2bcc0`.
- Reranker model, when `--rerank` is tested:
  `ggml-org/Qwen3-Reranker-0.6B-Q8_0-GGUF` /
  `qwen3-reranker-0.6b-q8_0.gguf`.
- Reranker artifact SHA-256:
  `22c9979ce4fbcdc5acdc310c6641c32797eff1aa980b8f7a2db8a8ea23429a48`.
- Chunking strategy: `qmd-rs-character-v1:3200:480`, with 3,200-character
  chunks, 480-character overlap, byte source spans, inherited title / document
  metadata, source page content hashes, and per-chunk text hashes recorded in
  `semantic-index.json`.

## Threshold Methodology

Use 30 total queries:

- 10 calibration queries: C1-C10.
- 20 hold-out queries: H1-H20.

Only C1-C10 may influence thresholds. H1-H20 are reserved for regression
checks after thresholds are chosen.

Accepted runtime thresholds (seeded, not calibrated):

| Field | Value | Calibration status |
| --- | --- | --- |
| `semantic_similarity_floor` | `0.35` | seeded |
| `hybrid_pre_fusion_semantic_floor` | `0.35` | seeded |
| `reranker_probability_floor` | `0.50` | seeded |
| `lexical_exact_identifier_guard` | `preserve_lexical_top_3` | seeded |

These were adopted on 2026-05-11 as a pragmatic starting point so the
semantic/hybrid runtime can execute end-to-end against this wiki and produce
the first observed eval results. When `~/.llm_wiki/search-thresholds.toml`
records these values with the matching embedding model artifact hash,
dimensions, qmd-rs version, adapter schema, and chunking strategy, explicit
`--mode semantic` / `--mode hybrid` and `--mode auto` on enabled profiles
execute instead of returning `thresholds_unconfigured`. Mismatched metadata
keeps the fail-closed behavior.

Seeded vs calibrated:

- Seeded means the value was chosen without observed eval data. The original
  zero-result failure mode (the battery-technology dogfood query) is still
  the worst-case shape; seeded floors should not bring it back.
- A change to any seeded value, or any change to embedding model, query
  expansion model, reranker model, chunking strategy, or qmd-rs version,
  must rerun the full 30-query set and update this table.
- The table is promoted from "seeded" to "calibrated" only after at least
  one full 30-query run has been recorded below and the human maintainer
  confirms the result is acceptable.

## Pass/Fail Rules

An expected-match query passes when at least one expected target appears in the
required top-K:

- Lexical: top 10 where lexical applies.
- Semantic: top 10 for conceptual queries after thresholds are configured.
- Hybrid: top 5 for conceptual and mixed queries.
- Auto: selected mode must match profile readiness and then satisfy the
  selected mode's rule.

Exact-identifier queries have an additional rule: hybrid must not bury the
lexical exact target below rank 5.

No-expected-match queries pass only when no candidate survives above the
configured relevance floor. As the wiki grows, each no-match query must be
revalidated because it may become a legitimate query later.

A regression is any of:

- expected-match target lost from the required top-K
- exact identifier query loses lexical precision
- no-expected-match query returns candidates above the relevance floor
- selected-mode, fallback, threshold, or zero-result metadata is absent or
  contradicts the mode contract
- verbose diagnostics contaminate JSON stdout

## Query Set

| ID | Split | Query | Purpose | Draft expected target pages |
| --- | --- | --- | --- | --- |
| C1 | Calibration | `what are the most cutting edge battery technologies` | Original dogfood failure phrase as recorded in this framework's search proposal | `wiki/proposals/search-query-interpretation.proposal.md`, `wiki/plans/semantic-hybrid-search.plan.md` |
| C2 | Calibration | `how does this framework make an agent productive in a fresh project` | Conceptual cross-page framework query | `wiki/roadmaps/framework-v1.roadmap.md`, `wiki/evals/v1-proof-run.eval.md`, `wiki/specs/wiki-init-skill.spec.md` |
| C3 | Calibration | `why did we choose qmd-rs instead of sqlite` | Decision rationale | `wiki/decisions/search-backend-selection.decision.md`, `wiki/evals/search-backend-selection.eval.md` |
| C4 | Calibration | `where should model files and semantic indexes live` | Managed runtime state | `wiki/proposals/search-query-interpretation.proposal.md`, `wiki/plans/semantic-hybrid-search.plan.md` |
| C5 | Calibration | `how should a project answer questions once the index is too large` | Search scale guidance | `wiki/specs/documentation-model.spec.md`, `wiki/references/qmd-rs-search-crate.reference.md` |
| C6 | Calibration | `what happens when a semantic index is stale` | Readiness behavior | `wiki/proposals/search-query-interpretation.proposal.md`, `wiki/plans/semantic-hybrid-search.plan.md` |
| C7 | Calibration | `how does search-all combine results from different projects` | Cross-project rank merge | `wiki/proposals/search-query-interpretation.proposal.md`, `wiki/plans/semantic-hybrid-search.plan.md`, `wiki/plans/project-registry-search-artifacts.plan.md` |
| C8 | Calibration | `what does interactive-only install mean for releases` | Install contract | `wiki/proposals/search-query-interpretation.proposal.md`, `wiki/plans/semantic-hybrid-search.plan.md` |
| C9 | Calibration | `which files prove the wiki init skill works` | Skill/spec retrieval | `wiki/evals/v1-proof-run.eval.md`, `wiki/specs/wiki-init-skill.spec.md` |
| C10 | Calibration | `what is the agent allowed to edit` | Agent ownership rule | `wiki/decisions/agent-owns-wiki.decision.md`, `wiki/specs/documentation-model.spec.md` |
| H1 | Hold-out | `qmd-rs search-all` | Exact identifier preservation | `wiki/evals/search-backend-selection.eval.md`, `wiki/plans/project-registry-search-artifacts.plan.md` |
| H2 | Hold-out | `--allow-lexical-fallback` | Exact fallback flag retrieval | `wiki/plans/semantic-hybrid-search.plan.md`, `wiki/proposals/search-query-interpretation.proposal.md` |
| H3 | Hold-out | `D10 composable init packs` | Mixed identifier plus concept | `wiki/plans/composable-project-init.plan.md`, `wiki/decisions/composable-project-init.decision.md` |
| H4 | Hold-out | `wiki-query search metadata` | Skill integration | `wiki/proposals/search-query-interpretation.proposal.md`, `wiki/plans/semantic-hybrid-search.plan.md`, `wiki/specs/wiki-query-skill.spec.md` |
| H5 | Hold-out | `install.partial.json` | Exact file/state retrieval | `wiki/plans/binary-path-bootstrap.plan.md` |
| H6 | Hold-out | `Time Machine model indexes` | Backup policy | `wiki/proposals/search-query-interpretation.proposal.md`, `wiki/plans/semantic-hybrid-search.plan.md` |
| H7 | Hold-out | `why is reranking optional in hybrid search` | Reranking contract and rejected mandatory-rerank alternative | `wiki/proposals/search-query-interpretation.proposal.md`, `wiki/plans/semantic-hybrid-search.plan.md` |
| H8 | Hold-out | `semantic index chunk ordinal source span` | Index metadata retrieval | `wiki/proposals/search-query-interpretation.proposal.md`, `wiki/plans/semantic-hybrid-search.plan.md` |
| H9 | Hold-out | `GPU shader compiler roadmap` | No expected match | none |
| H10 | Hold-out | `PostgreSQL connection pooling` | No expected match | none |
| H11 | Hold-out | `browser automation plugin release checklist` | No expected match; revalidate as plugin docs grow | none |
| H12 | Hold-out | `how do accepted proposals become plans` | Documentation model query | `wiki/specs/documentation-model.spec.md`, `wiki/roadmaps/framework-v1.roadmap.md` |
| H13 | Hold-out | `what command changes the search profile later` | Configure-search contract | `wiki/plans/semantic-hybrid-search.plan.md`, `wiki/proposals/search-query-interpretation.proposal.md` |
| H14 | Hold-out | `why should hybrid not silently downgrade to lexical` | Fallback contract | `wiki/proposals/search-query-interpretation.proposal.md`, `wiki/plans/semantic-hybrid-search.plan.md` |
| H15 | Hold-out | `how are verbose diagnostics kept out of JSON stdout` | Observability contract | `wiki/proposals/cli-observability.proposal.md`, `wiki/plans/cli-observability.plan.md`, `wiki/checklists/observability-contract.checklist.md` |
| H16 | Hold-out | `where are root code folders created for CLI products` | Code pack / CLI blueprint behavior | `wiki/decisions/code-pack-cli-blueprint.decision.md`, `wiki/plans/code-pack-cli-blueprint.plan.md` |
| H17 | Hold-out | `what evidence proved D4 through D7` | V1 proof evidence | `wiki/evals/v1-proof-run.eval.md`, `wiki/roadmaps/framework-v1.roadmap.md` |
| H18 | Hold-out | `how does the managed binary path avoid relying on PATH` | Managed runtime install behavior | `wiki/decisions/binary-path-bootstrap.decision.md`, `wiki/plans/binary-path-bootstrap.plan.md` |
| H19 | Hold-out | `how are crash reports stored for post-parse errors` | New crash proposal retrieval | `wiki/proposals/crash-reports.proposal.md` |
| H20 | Hold-out | `CUDA kernel occupancy tuning guide` | No expected match | none |

## No-Match Maintenance

The current no-expected-match queries are H9, H10, H11, and H20. Before each
eval run, verify that the wiki has not gained real content for these topics. If
one becomes a valid project topic, move it to expected-match status and add a
new no-match query before comparing results with prior runs.

The original battery-technology dogfood failure also needs an external
electric-car fixture replay once that project is made available as a stable
eval corpus. C1 keeps the phrase in this repo's calibration set by targeting
the proposal and plan that record the failure; it is not a substitute for the
domain-corpus replay.

## Observability Merge Evidence

Evidence checked on 2026-05-11 after rebasing onto the completed observability
work:

- `src/cli.rs` defines root global `-v` / `--verbose`, `CliContext`, the
  `verbose` field, and the `diagnostic(...)` helper.
- `src/main.rs` calls `init_tracing(cli.verbose)`, constructs
  `CliContext::new(cli.verbose)`, and passes the context into command handlers.
- `src/search/commands.rs` emits `search` diagnostics for raw query,
  normalized FTS query, class/status filters, limit, registry path, project
  selection, selected project, wiki root, qmd-rs store path, backend,
  index-state, result count, and no-hit reason.
- `src/search/commands.rs` emits `search-all` diagnostics for raw query,
  normalized FTS query, filters, limit, selected projects, per-project wiki
  root, per-project store path, per-project index-state, per-project result
  count, per-project no-hit reason, and final fused result count.
- `tests/search_commands.rs` covers verbose `search` and `search-all`, global
  verbose flag placement, JSON stdout parseability, zero-result explanations,
  filter exclusions, and explicit `--limit 0`.
- `wiki/checklists/observability-contract.checklist.md` records the same
  `CliContext` / tracing boundary as the standing review gate.

Semantic/hybrid implementation should extend this surface; it should not add a
second diagnostics system.

## Next Actions

1. Install the embedding and query-expansion models through
   `llm-wiki install --configure-search` so semantic/hybrid retrieval has
   verified artifacts to read.
2. Write `~/.llm_wiki/search-thresholds.toml` with the seeded values above
   plus the matching model artifact hash, dimensions, qmd-rs version,
   adapter schema, and chunking strategy so `thresholds_match_index_inputs`
   returns true.
3. Run the full 30-query set (C1-C10 and H1-H20) through
   `llm-wiki search --mode auto --format json` and
   `llm-wiki search --mode hybrid --format json`; record observed results
   in a new "Observed Runs" section below, including the semantic vector
   index fingerprint and artifact hashes.
4. Compare observed results to the pass/fail rules. Adjust seeded floors
   only after recording the run that motivated the change.
5. Promote the threshold table from "seeded" to "calibrated" once the
   human maintainer confirms a run satisfies the pass/fail rules.
