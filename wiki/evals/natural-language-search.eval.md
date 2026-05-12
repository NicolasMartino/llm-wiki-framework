# Natural-Language Search Eval

- Document Class: Eval
- Status: Active
- Date: 2026-05-12
- Category: Search UX, semantic retrieval, hybrid retrieval
- Scope: Stage 0 natural-language eval suite, expected target draft, calibration
  split, threshold methodology, and observability evidence for semantic/hybrid
  `llm-wiki search`.
- Sources: wiki/plans/semantic-hybrid-search.plan.md, wiki/proposals/search-query-interpretation.proposal.md, wiki/evals/search-backend-selection.eval.md, wiki/references/llm-search-model-licensing.reference.md, wiki/checklists/observability-contract.checklist.md, raw/research/2026-05-11-llm-search-model-licensing/research-summary.md, raw/data/eval/natural-language-search/20260512T091123Z-10597/balanced/eval-calibration.json, raw/data/eval/natural-language-search/20260512T145231Z-68005/balanced/eval-calibration.json, raw/data/eval/natural-language-search/20260512T154322Z-77945/balanced/eval-calibration.json, raw/data/eval/electric-cars/20260512T162918Z-86751/balanced/eval-calibration.json
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
- Human target approval: accepted on 2026-05-12 for all current framework and
  electric-car target labels. The latest balanced framework threshold proposal
  has passed the post-apply natural-language production harness, and the
  electric-car domain-corpus replay has been applied under its own project
  scope.
- The current durable baseline thresholds are calibrated from the balanced
  2026-05-12 run listed below. Future label edits remain allowed, but any
  retrieval-affecting label change must trigger a fresh `eval run` and
  `eval calibrate` before thresholds are promoted again.

## Calibration Identity

First calibration identity fields:

- qmd-rs version: `0.3.2`.
- Adapter schema version: `1`.
- Embedding model: `ggml-org/embeddinggemma-300M-GGUF` /
  `embeddinggemma-300M-Q8_0.gguf`.
- Embedding repository revision:
  `0f741b5a6585bd53aeb15cd1372c56f2a0f65e12`.
- Embedding artifact SHA-256:
  `b5ce9d77a3fc4b3b39ccb5643c36777911cc4eb46a66962eadfa3f5f60490d63`.
- Embedding dimensions: 768 from the upstream EmbeddingGemma model card.
- Query-expansion model: `tobil/qmd-query-expansion-1.7B-gguf` /
  `qmd-query-expansion-1.7B-q4_k_m.gguf`.
- Query-expansion repository revision:
  `7816de0b72572c6c860ca1eddf97ba9e7fb8cc65`.
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

- 10 calibration queries: C1-C10, including C10 as the calibration no-match
  sentinel.
- 20 hold-out queries: H1-H20.

Only C1-C10 may influence thresholds. H1-H20 are reserved for regression
checks after thresholds are chosen.

Accepted runtime thresholds:

| Field | Value | Calibration status |
| --- | --- | --- |
| `semantic_similarity_floor` | `0.328807` | calibrated from `20260512T154322Z-77945` |
| `hybrid_pre_fusion_semantic_floor` | `0.103599` | calibrated from `20260512T154322Z-77945` |
| `hybrid_final_semantic_floor` | `0.399904` | calibrated from anchor-leaking no-match evidence in `20260512T154322Z-77945` |
| `hybrid_semantic_only_floor` | `0.50` | calibrated from `20260512T154322Z-77945` with default floor preserved |
| `hybrid_strong_lexical_score_floor` | `0.5` | calibrated from `20260512T154322Z-77945` |
| `reranker_probability_floor` | `0.50` | seeded |
| `lexical_exact_identifier_guard` | `preserve_lexical_top_3` | configured strategy |

The original 2026-05-11 values were seeded to make the semantic/hybrid runtime
execute end-to-end. The table now records the locally applied balanced
calibration from 2026-05-12. When `~/.llm_wiki/search-thresholds.toml` records
these values with the matching embedding model artifact hash, dimensions,
qmd-rs version, adapter schema, and chunking strategy, explicit `--mode
semantic` / `--mode hybrid` and `--mode auto` on enabled profiles execute
instead of returning `thresholds_unconfigured`. Mismatched metadata keeps the
fail-closed behavior.

Seeded vs calibrated:

- Seeded means the value was chosen without observed eval data. The original
  zero-result failure mode (the battery-technology dogfood query) is still
  the worst-case shape; seeded floors should not bring it back.
- A change to any calibrated value, or any change to embedding model, query
  expansion model, reranker model, chunking strategy, or qmd-rs version,
  must rerun the full 30-query set and update this table.
- The table is the durable accepted baseline for this framework wiki as of
  2026-05-12. Thresholds are applied as project-scoped records so the
  electric-car replay can keep its higher floors without overwriting this
  framework-wiki profile.

## Pass/Fail Rules

An expected-match query passes when at least one expected target appears in the
required top-K for modes listed in the query table's `Applies` column:

- Lexical: top 10 where lexical applies.
- Semantic: top 10 for conceptual queries after thresholds are configured.
- Hybrid: top 5 for conceptual and mixed queries.
- Auto: selected mode must match profile readiness and then satisfy the
  selected mode's rule. If the candidate is not hybrid-ready, auto records a
  readiness outcome rather than falling back to lexical for eval measurement.

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

The query table is intentionally hidden from the search index. It is eval
fixture material, not project knowledge; indexing it would let no-match queries
match the eval page that defines them and would overstate retrieval quality.

<!-- llm-wiki-search-ignore-start -->

| ID | Split | Query | Purpose | Applies | Draft expected target pages |
| --- | --- | --- | --- | --- | --- |
| C1 | Calibration | `what are the most cutting edge battery technologies` | Original dogfood failure phrase as recorded in this framework's search proposal; hybrid/lexical paper-trail query, not pure semantic domain retrieval | `lexical`, `hybrid`, `auto` | `wiki/proposals/search-query-interpretation.proposal.md`, `wiki/plans/semantic-hybrid-search.plan.md` |
| C2 | Calibration | `how does this framework make an agent productive in a fresh project` | Conceptual cross-page framework query | all | `wiki/roadmaps/framework-v1.roadmap.md`, `wiki/evals/v1-proof-run.eval.md`, `wiki/specs/wiki-init-skill.spec.md` |
| C3 | Calibration | `why did we choose qmd-rs instead of sqlite` | Decision rationale | all | `wiki/decisions/search-backend-selection.decision.md`, `wiki/evals/search-backend-selection.eval.md` |
| C4 | Calibration | `where should model files and semantic indexes live` | Managed runtime state | all | `wiki/proposals/search-query-interpretation.proposal.md`, `wiki/plans/semantic-hybrid-search.plan.md` |
| C5 | Calibration | `how should a project answer questions once the index is too large` | Search scale guidance | all | `wiki/specs/documentation-model.spec.md`, `wiki/references/qmd-rs-search-crate.reference.md`, `wiki/proposals/search-query-interpretation.proposal.md`, `wiki/specs/wiki-query-skill.spec.md` |
| C6 | Calibration | `what happens when a semantic index is stale` | Readiness behavior | all | `wiki/proposals/search-query-interpretation.proposal.md`, `wiki/plans/semantic-hybrid-search.plan.md` |
| C7 | Calibration | `how does search-all combine results from different projects` | Cross-project rank merge | all | `wiki/proposals/search-query-interpretation.proposal.md`, `wiki/plans/semantic-hybrid-search.plan.md`, `wiki/plans/project-registry-search-artifacts.plan.md` |
| C8 | Calibration | `what does interactive-only install mean for releases` | Install contract | all | `wiki/proposals/search-query-interpretation.proposal.md`, `wiki/plans/semantic-hybrid-search.plan.md` |
| C9 | Calibration | `which files prove the wiki init skill works` | Skill/spec retrieval | all | `wiki/evals/v1-proof-run.eval.md`, `wiki/specs/wiki-init-skill.spec.md` |
| C10 | Calibration | `PostgreSQL connection pooling` | Stable no expected match; calibration no-match coverage for relevance floors | all | none |
| H1 | Hold-out | `qmd-rs search-all` | Exact identifier preservation | all | `wiki/evals/search-backend-selection.eval.md`, `wiki/plans/project-registry-search-artifacts.plan.md` |
| H2 | Hold-out | `--allow-lexical-fallback` | Exact fallback flag retrieval | all | `wiki/plans/semantic-hybrid-search.plan.md`, `wiki/proposals/search-query-interpretation.proposal.md` |
| H3 | Hold-out | `D10 composable init packs` | Mixed identifier plus concept | all | `wiki/plans/composable-project-init.plan.md`, `wiki/decisions/composable-project-init.decision.md` |
| H4 | Hold-out | `wiki-query search metadata` | Skill integration | all | `wiki/proposals/search-query-interpretation.proposal.md`, `wiki/plans/semantic-hybrid-search.plan.md`, `wiki/specs/wiki-query-skill.spec.md` |
| H5 | Hold-out | `install.partial.json` | Exact file/state retrieval | all | `wiki/plans/binary-path-bootstrap.plan.md` |
| H6 | Hold-out | `Time Machine model indexes` | Backup policy | all | `wiki/proposals/search-query-interpretation.proposal.md`, `wiki/plans/semantic-hybrid-search.plan.md` |
| H7 | Hold-out | `why is reranking optional in hybrid search` | Reranking contract and rejected mandatory-rerank alternative | all | `wiki/proposals/search-query-interpretation.proposal.md`, `wiki/plans/semantic-hybrid-search.plan.md` |
| H8 | Hold-out | `semantic index chunk ordinal source span` | Index metadata retrieval | all | `wiki/proposals/search-query-interpretation.proposal.md`, `wiki/plans/semantic-hybrid-search.plan.md` |
| H9 | Hold-out | `GPU shader compiler roadmap` | No expected match | all | none |
| H10 | Hold-out | `what is the agent allowed to edit` | Agent ownership rule moved from calibration to preserve hold-out coverage after adding calibration no-match | all | `wiki/decisions/agent-owns-wiki.decision.md`, `wiki/specs/documentation-model.spec.md` |
| H11 | Hold-out | `browser automation plugin release checklist` | No expected match; revalidate as plugin docs grow | all | none |
| H12 | Hold-out | `how do accepted proposals become plans` | Documentation model query | all | `wiki/specs/documentation-model.spec.md` |
| H13 | Hold-out | `what command changes the search profile later` | Configure-search contract | all | `wiki/plans/semantic-hybrid-search.plan.md`, `wiki/proposals/search-query-interpretation.proposal.md` |
| H14 | Hold-out | `why should hybrid not silently downgrade to lexical` | Fallback contract | all | `wiki/proposals/search-query-interpretation.proposal.md`, `wiki/plans/semantic-hybrid-search.plan.md` |
| H15 | Hold-out | `how are verbose diagnostics kept out of JSON stdout` | Observability contract | all | `wiki/proposals/cli-observability.proposal.md`, `wiki/plans/cli-observability.plan.md`, `wiki/checklists/observability-contract.checklist.md` |
| H16 | Hold-out | `where are root code folders created for CLI products` | Code pack / CLI blueprint behavior | all | `wiki/decisions/code-pack-cli-blueprint.decision.md`, `wiki/plans/code-pack-cli-blueprint.plan.md` |
| H17 | Hold-out | `what evidence proved D4 through D7` | V1 proof evidence | all | `wiki/evals/v1-proof-run.eval.md`, `wiki/roadmaps/framework-v1.roadmap.md` |
| H18 | Hold-out | `how does the managed binary path avoid relying on PATH` | Managed runtime install behavior | all | `wiki/decisions/binary-path-bootstrap.decision.md`, `wiki/plans/binary-path-bootstrap.plan.md` |
| H19 | Hold-out | `how are crash reports stored for post-parse errors` | New crash proposal retrieval | all | `wiki/proposals/crash-reports.proposal.md` |
| H20 | Hold-out | `CUDA kernel occupancy tuning guide` | No expected match | all | none |

<!-- llm-wiki-search-ignore-end -->

## No-Match Maintenance

The current no-expected-match queries are C10, H9, H11, and H20. C10 is in the
calibration split so threshold proposals can test relevance floors against at
least one known no-match query. H9, H11, and H20 remain hold-out no-match
queries. Before each eval run, verify that the wiki has not gained real content
for these topics. If one becomes a valid project topic, move it to
expected-match status and add a new no-match query before comparing results
with prior runs.

The original battery-technology dogfood failure also needs an external
electric-car fixture replay once that project is made available as a stable
eval corpus. C1 keeps the phrase in this repo's calibration set by targeting
the proposal and plan that record the failure. It applies to lexical, hybrid,
and auto as a paper-trail query, but not to pure semantic mode because the
framework wiki is not a battery-technology corpus. It is not a substitute for
the domain-corpus replay.

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

## Observed Runs

### 2026-05-11 Seeded Local GGUF Run

Harness:

- Rust integration test:
  `cargo test --test natural_language_search_eval -- --ignored --nocapture`.
- Fast default test:
  `cargo test --test natural_language_search_eval` checks the wiki query table
  shape without loading managed model artifacts.
- Generated artifacts:
  `target/evals/natural-language-search-results.json` and
  `target/evals/natural-language-search-summary.md`.

Runtime inputs:

- Project: `llm-wiki-framework-semantic-search`.
- Binary/framework version: `0.1.1`.
- qmd-rs version: `0.3.2`.
- Adapter schema version: `1`.
- Profile: `balanced`.
- Embedding model: `embeddinggemma-300m-q8_0`;
  SHA-256 `b5ce9d77a3fc4b3b39ccb5643c36777911cc4eb46a66962eadfa3f5f60490d63`.
- Query-expansion model: `qmd-query-expansion-1.7b-q4_k_m`;
  SHA-256 `000dfb1c06efa6a049e9f64ba921c3740e2454f62abab6fa10e77bd30bb2bcc0`.
- Reranker: not installed or used for this run.
- Chunking strategy: `qmd-rs-character-v1:3200:480`.
- Source fingerprint:
  `ac89e166c5f46a5bcb42d81d1516833b16083c8bb20b06336249991d47fdc877`.
- Index generated at: `2026-05-11T15:42:56Z`.
- Indexed source files: 61.
- Semantic chunks/vectors: 288.
- Eval generated at: `2026-05-11T16:24:24Z`.

Observed results:

| Mode | Pass | Fail | Not applicable | Failure IDs |
| --- | ---: | ---: | ---: | --- |
| lexical | 16 | 10 | 4 | C2, C3, C5, C9, C10, H12, H15, H16, H18, H19 |
| semantic | 25 | 5 | 0 | C1, C5, C8, H11, H12 |
| hybrid | 23 | 7 | 0 | C5, C9, H9, H10, H11, H12, H20 |
| auto | 23 | 7 | 0 | C5, C9, H9, H10, H11, H12, H20 |

JSON stdout parsed for every query/mode pair, every command exited
successfully, and `auto` selected `hybrid` for all 30 queries because the local
profile and semantic index were ready.

This run is useful evidence but does not promote the seeded threshold table.
Semantic retrieval improves natural-language recall over lexical, and
hybrid/auto preserve many exact and mixed queries, but the current quality is
not yet acceptable for a calibrated gate:

- C5 and H12 show target-label or retrieval weakness for documentation-model
  scale/process questions.
- C9 passes semantic but fails hybrid/auto because fused binary/roadmap
  results push the expected wiki-init evidence out of the required top 5.
- H9, H10, H11, and H20 show no-match weakness: semantic floors suppress three
  of them, but hybrid/auto still return eval/plan pages for unrelated topics.

Implementation notes from this run:

- Hybrid retrieval now keeps the raw user query in both lexical and semantic
  branches before adding query-expansion variants, preventing expansions from
  displacing exact intent.
- Semantic chunking now uses byte spans produced at UTF-8 character
  boundaries, avoiding qmd-rs panics on box-drawing/table characters.
- Model catalog entries are pinned to repository revisions and verified
  artifact hashes observed through the upstream model APIs.

### 2026-05-11 Eval Subcommand Baseline Before C-Split No-Match

Harness:

- Command:
  `cargo run -- eval run --project llm-wiki-framework-semantic-search --candidate-profile balanced --eval-page wiki/evals/natural-language-search.eval.md`.
- Calibration command:
  `cargo run -- eval calibrate --run-report target/evals/20260511T184522Z-53329/eval-run.json`.
- Generated artifacts:
  `target/evals/20260511T184522Z-53329/eval-run.json`,
  `target/evals/20260511T184522Z-53329/eval-run-summary.md`, and
  `target/evals/20260511T184522Z-53329/eval-calibration.json`.

Runtime inputs:

- Run ID: `20260511T184522Z-53329`.
- Candidate/profile: `balanced`.
- Embedding model: `embeddinggemma-300m-q8_0`;
  SHA-256 `b5ce9d77a3fc4b3b39ccb5643c36777911cc4eb46a66962eadfa3f5f60490d63`;
  dimensions `768`.
- Query-expansion model: `qmd-query-expansion-1.7b-q4_k_m`.
- Candidate vectors: 294.
- Index fingerprint:
  `61fca51b21838d1f0532867ac44b7fdc90b53caec1a28e655910a26beb16abd1`.
- Elapsed runtime: 379.772 seconds.

Observed results:

| Mode | Pass | Fail | Not applicable | Notes |
| --- | ---: | ---: | ---: | --- |
| lexical | 14 | 12 | 4 | No-match queries remain not applicable because lexical has no calibrated no-match floor. |
| semantic | 23 | 7 | 0 | C1 and C5 were among the calibration failures. |
| hybrid | 21 | 9 | 0 | No-match hold-outs still returned candidates. |
| auto | 21 | 9 | 0 | Same verdicts as hybrid because the profile selected hybrid. |

Calibration result:

- Status: `blocked_no_calibration_no_match`.
- Promotable: `false`.
- Proposed `semantic_similarity_floor`: `0.328`.
- Proposed `hybrid_pre_fusion_semantic_floor`: `0.020`.
- Calibration expected queries: 10.
- Calibration no-match queries: 0.
- Missing expected targets reported by calibration: C1 and C5.

Finding:

The eval subcommand executed end to end and produced machine-readable report
artifacts, but the then-current split could not honestly calibrate no-match
thresholds because all C1-C10 rows had expected targets. This was a structural
eval-design blocker, independent of whether the proposed numeric floors looked
plausible. Thresholds were not applied.

### 2026-05-11 Eval Subcommand After C10 No-Match Swap

Change before rerun:

- Moved the PostgreSQL-themed no-match query from H10 to C10 as the
  calibration no-match sentinel.
- Moved `what is the agent allowed to edit` from C10 to H10 so the suite still
  has 10 calibration rows and 20 hold-out rows.
- Rebuilt the local project index with
  `cargo run -- index --project llm-wiki-framework-semantic-search --force`.

Harness:

- Command:
  `cargo run -- eval run --project llm-wiki-framework-semantic-search --candidate-profile balanced --eval-page wiki/evals/natural-language-search.eval.md --output-dir target/evals/20260511-c10-no-match-balanced`.
- Calibration command:
  `cargo run -- eval calibrate --run-report target/evals/20260511-c10-no-match-balanced/eval-run.json --output-dir target/evals/20260511-c10-no-match-balanced`.
- Generated artifacts:
  `target/evals/20260511-c10-no-match-balanced/eval-run.json`,
  `target/evals/20260511-c10-no-match-balanced/eval-run-summary.md`, and
  `target/evals/20260511-c10-no-match-balanced/eval-calibration.json`.

Runtime inputs:

- Run ID: `20260511T185341Z-55066`.
- Candidate/profile: `balanced`.
- Embedding model: `embeddinggemma-300m-q8_0`;
  SHA-256 `b5ce9d77a3fc4b3b39ccb5643c36777911cc4eb46a66962eadfa3f5f60490d63`;
  dimensions `768`.
- Candidate vectors: 294.
- Index fingerprint:
  `a73932eeaf9dad3d94a141fa15642a98517d44709229e914293db52a9e9b73d4`.
- Elapsed runtime: 374.967 seconds.

Observed results:

| Mode | Pass | Fail | Not applicable | Notes |
| --- | ---: | ---: | ---: | --- |
| lexical | 14 | 12 | 4 | Aggregate unchanged from the baseline run. |
| semantic | 23 | 7 | 0 | Aggregate unchanged; C10 now fails as a calibration no-match. |
| hybrid | 21 | 9 | 0 | Aggregate unchanged; C10 now fails in calibration instead of H10 failing as a hold-out no-match. |
| auto | 21 | 9 | 0 | Same verdicts as hybrid. |

Calibration result:

- Status: `blocked_missing_expected_targets`.
- Promotable: `false`.
- Proposed `semantic_similarity_floor`: `0.328`.
- Proposed `hybrid_pre_fusion_semantic_floor`: `0.020`.
- Calibration expected queries: 9.
- Calibration no-match queries: 1.
- Missing expected targets reported by calibration: C1 and C5.

Detailed findings:

- The structural no-match blocker is fixed: calibration now sees one C-split
  no-match row.
- Thresholds are still not promotable because calibration expected-match rows
  C1 and C5 do not both satisfy the required semantic evidence.
- C1 passes lexical, hybrid, and auto with an expected target at rank 1, but
  pure semantic misses the expected target in top 10. Its top semantic pages
  were `wiki/references/llm-wiki-ecosystem.reference.md`, `wiki/log.md`,
  `wiki/decisions/composable-project-init.decision.md`,
  `wiki/proposals/blueprint-pack-init.proposal.md`, and
  `wiki/proposals/skills-template-engine.proposal.md`.
- C5 is the strongest expected-match blocker. Lexical returns no candidates,
  semantic ranks project-registry/index/qmd-rs pages above the expected
  documentation-model or qmd-rs-reference pages, and hybrid/auto also miss the
  expected target in top 5.
- C10 now exposes the no-match weakness inside calibration. Semantic returns
  qmd-rs/search-backend pages for the C10 sentinel with top score `0.19449`;
  hybrid/auto return qmd-rs/search-backend pages with top score `0.020492`.
- H10 remains useful as a hold-out expected-match query after the move:
  semantic ranks `wiki/decisions/agent-owns-wiki.decision.md` first, and
  hybrid/auto rank it fourth. Lexical still misses it in top 10.
- No `--apply` was run. The threshold table remains seeded because the
  candidate calibration report is non-promotable.
- Both full eval subcommand runs took a little over six minutes and emitted
  very large GGUF loader logs. That is acceptable for manual evidence capture
  but should be improved before this becomes a polished reusable tuning tool.

### 2026-05-11 Eval Subcommand After Mode Applicability And C5 Label Fix

Change before rerun:

- Added an `Applies` column to the eval table. C1 now applies only to
  lexical, hybrid, and auto because it is a paper-trail query in this
  framework wiki, not a pure semantic battery-technology corpus query.
- Expanded C5's expected targets to include
  `wiki/proposals/search-query-interpretation.proposal.md` and
  `wiki/specs/wiki-query-skill.spec.md`. The earlier label set was too narrow:
  those pages are valid answers for the large-index search-scale query.
- Tightened calibration logic so non-applicable modes are skipped, failed
  low-rank expected hits are not used as floor evidence, and the report exposes
  per-mode expected/no-match score diagnostics.
- Fixed the future threshold-apply path so calibrated semantic and hybrid
  pre-fusion floors would not accidentally zero the seeded final hybrid,
  strong lexical, semantic-only, reranker, or exact-identifier guard settings.

Harness:

- Command:
  `cargo run -- eval run --project llm-wiki-framework-semantic-search --candidate-profile balanced --eval-page wiki/evals/natural-language-search.eval.md --output-dir target/evals/20260511-mode-applicability-c5-balanced`.
- Calibration command:
  `cargo run -- eval calibrate --run-report target/evals/20260511-mode-applicability-c5-balanced/eval-run.json --output-dir target/evals/20260511-mode-applicability-c5-balanced`.
- Generated artifacts:
  `target/evals/20260511-mode-applicability-c5-balanced/eval-run.json`,
  `target/evals/20260511-mode-applicability-c5-balanced/eval-run-summary.md`,
  and
  `target/evals/20260511-mode-applicability-c5-balanced/eval-calibration.json`.

Runtime inputs:

- Run ID: `20260511T192811Z-62109`.
- Candidate/profile: `balanced`.
- Embedding model: `embeddinggemma-300m-q8_0`;
  SHA-256 `b5ce9d77a3fc4b3b39ccb5643c36777911cc4eb46a66962eadfa3f5f60490d63`;
  dimensions `768`.
- Query-expansion model: `qmd-query-expansion-1.7b-q4_k_m`.
- Candidate vectors: 298.
- Index fingerprint:
  `3ce062728d1b79a9fa521f5e4e5ca4496b71889a16301a295488b1ec53962c75`.
- Elapsed runtime: 355.586 seconds.

Observed results:

| Mode | Pass | Fail | Not applicable | Notes |
| --- | ---: | ---: | ---: | --- |
| lexical | 14 | 12 | 4 | No-match rows remain not applicable for lexical relevance floors. |
| semantic | 24 | 5 | 1 | C1 is now not applicable; C5 now passes at rank 4. |
| hybrid | 22 | 8 | 0 | C5 now passes at rank 5; C3 and C9 still fail top 5. |
| auto | 22 | 8 | 0 | Same verdicts as hybrid because the profile selected hybrid. |

Calibration result:

- Status: `blocked_missing_expected_targets`.
- Promotable: `false`.
- Proposed `semantic_similarity_floor`: `0.328`.
- Semantic min expected score: `0.328807`.
- Semantic max calibration no-match score: `0.194490`.
- Proposed `hybrid_pre_fusion_semantic_floor`: `0.027`.
- Hybrid min expected score in the current report score space: `0.027531`.
- Hybrid max calibration no-match score in the current report score space:
  `0.046776`.
- Calibration expected queries: 9.
- Calibration no-match queries: 1.
- Missing expected targets reported by calibration: C3 and C9.

Detailed findings:

- C1 is no longer a threshold-promotion blocker. It passes lexical, hybrid, and
  auto at rank 1, and semantic is explicitly not applicable for this row.
- C5 is no longer a label blocker. Semantic ranks
  `wiki/proposals/search-query-interpretation.proposal.md` fourth and
  `wiki/specs/wiki-query-skill.spec.md` fifth; hybrid/auto rank
  `wiki/proposals/search-query-interpretation.proposal.md` fifth.
- C3 remains a hybrid/auto ranking blocker. Semantic ranks
  `wiki/decisions/search-backend-selection.decision.md` first, but hybrid/auto
  rank log/proposal pages above the expected decision and miss it from the
  required top 5. The target appears at rank 6, so this is a fusion/ranking
  problem rather than a missing semantic-evidence problem.
- C9 remains a hybrid/auto ranking blocker. Semantic ranks
  `wiki/specs/wiki-init-skill.spec.md` first, but hybrid/auto rank roadmap,
  log, binary-plan, and install-proposal pages above the expected wiki-init
  evidence. The target appears below the required top 5.
- C10 still exposes no-match weakness. In the calibration split, semantic can
  separate C10 from the weakest passing semantic expected target
  (`0.194490 < 0.328807`), but hybrid/auto cannot separate the current C10
  fused top score from the weakest passing hybrid target
  (`0.046776 > 0.027531`).
- The corrected calibrator now refuses to promote because C3 and C9 fail the
  required top-K rule. This is safer than the previous behavior, which treated
  low-rank expected hits as usable calibration evidence.
- No `--apply` was run. The threshold table remains seeded because the
  candidate calibration report is non-promotable.

### 2026-05-11 Eval Subcommand After Branch Evidence And C3/C9 Fusion Fix

Change before rerun:

- Added hybrid branch evidence to search JSON and eval outcomes:
  `lexical_rank`, `lexical_score`, `semantic_rank`, and `semantic_score`.
- Changed `eval calibrate` so `hybrid_pre_fusion_semantic_floor` is derived
  from semantic branch scores instead of rank-fused hybrid result scores.
- Added a high-confidence semantic prefix boost in hybrid fusion. The boost is
  deliberately narrow: for non-exact-identifier queries, the top semantic
  result gets an additional fused-score increment only when its semantic score
  clears the configured semantic-only confidence floor.
- Clarified the documentation-model promotion flow so accepted proposals are
  not described as automatically becoming validated truth. Roadmaps coordinate
  deliverables, plans own tactical execution, and specs/decisions receive only
  evidence-backed durable behavior.
- Rebuilt the local project index with
  `cargo run -- index --project llm-wiki-framework-semantic-search --force`.

Harness:

- Command:
  `cargo run -- eval run --project llm-wiki-framework-semantic-search --candidate-profile balanced --eval-page wiki/evals/natural-language-search.eval.md --output-dir target/evals/20260511-branch-evidence-docflow-balanced`.
- Calibration command:
  `cargo run -- eval calibrate --run-report target/evals/20260511-branch-evidence-docflow-balanced/eval-run.json --output-dir target/evals/20260511-branch-evidence-docflow-balanced`.
- Generated artifacts:
  `target/evals/20260511-branch-evidence-docflow-balanced/eval-run.json`,
  `target/evals/20260511-branch-evidence-docflow-balanced/eval-run-summary.md`,
  and
  `target/evals/20260511-branch-evidence-docflow-balanced/eval-calibration.json`.
- Raw evidence bundle:
  `raw/data/eval/natural-language-search/20260511T205231Z-79135/balanced/`.
- Ingested impact summary:
  `wiki/evals/natural-language-search-impact.md`.

Runtime inputs:

- Run ID: `20260511T205231Z-79135`.
- Candidate/profile: `balanced`.
- Embedding model: `embeddinggemma-300m-q8_0`;
  SHA-256 `b5ce9d77a3fc4b3b39ccb5643c36777911cc4eb46a66962eadfa3f5f60490d63`;
  dimensions `768`.
- Query-expansion model: `qmd-query-expansion-1.7b-q4_k_m`;
  SHA-256 `000dfb1c06efa6a049e9f64ba921c3740e2454f62abab6fa10e77bd30bb2bcc0`.
- Candidate vectors: 302.
- Index fingerprint:
  `382c28092e8daecca302a671134188d0b02789232dfe87ea43b97e1f196fdb87`.
- Elapsed runtime: 359.546 seconds.

Observed results:

| Mode | Pass | Fail | Not applicable | Notes |
| --- | ---: | ---: | ---: | --- |
| lexical | 14 | 12 | 4 | Lexical still misses conceptual queries C2, C3, C5, C6, C7, C9, H10, H12, H15, H16, H18, and H19. |
| semantic | 24 | 5 | 1 | C1 remains not applicable; C10, H9, H11, H12, and H20 fail. |
| hybrid | 25 | 5 | 0 | C3 and C9 now pass at rank 1; C10, H9, H11, H12, and H20 fail. |
| auto | 25 | 5 | 0 | Same verdicts as hybrid because the profile selected hybrid. |

Calibration result:

- Status: `blocked_no_feasible_threshold`.
- Promotable: `false`.
- Proposed `semantic_similarity_floor`: `0.328807`.
- Semantic min expected score: `0.328807`.
- Semantic max calibration no-match score: `0.194490`.
- Semantic promotable: `true`.
- Proposed `hybrid_pre_fusion_semantic_floor`: `0.103599`.
- Hybrid min expected semantic branch score: `0.103599`.
- Hybrid max calibration no-match semantic branch score: `0.194490`.
- Hybrid promotable: `false`.
- Calibration expected queries: 9.
- Calibration no-match queries: 1.
- Missing expected targets reported by calibration: none.
- Recomputed calibration report after the Stage 7 reporting-gap fix:
  - Proposed summary: semantic 26 / 3 / 1, hybrid 17 / 13, auto 17 / 13,
    lexical 14 / 12 / 4.
  - Hold-out summary under proposed thresholds: semantic 17 / 3, hybrid
    12 / 8, auto 12 / 8, lexical 11 / 6 / 3.
  - Proposed calibration pass: `false`.
  - Hold-out pass: `false`.
  - Verdict changes: 34.
  - No-match precision under proposed thresholds: hybrid and auto 4 / 4,
    semantic 2 / 4.
  - Exact-identifier preservation under proposed thresholds: lexical 4 / 4,
    hybrid and auto 3 / 4.
  - Model artifact bytes: `1616029856`.
  - Candidate index bytes: `2707024`.

Detailed findings:

- C3 is fixed for hybrid/auto. The search-backend decision is now rank 1 in
  hybrid and auto with semantic branch score `0.553500`.
- C9 is fixed for hybrid/auto. The wiki-init skill spec is now rank 1 in
  hybrid and auto with semantic branch score `0.645496`.
- The old fused-score calibration was false confidence. C10's fused hybrid top
  score is only `0.020492`, but the same candidate's semantic branch score is
  `0.194490`. Hybrid pre-fusion thresholds must be calibrated against branch
  scores because that is the value the production semantic branch filters on.
- C1 proves that hybrid calibration must distinguish lexical-driven paper-trail
  hits from semantic-dependent hits. It passes hybrid at rank 1 with lexical
  score `18.141372`, but its semantic branch score is only `0.103599`; using
  that value as semantic floor evidence would overfit a lexical case.
- H12 still fails all modes after the documentation-model promotion-flow edit.
  The query now retrieves search/model/proposal/plan pages above the expected
  documentation-model and framework-roadmap targets. This is no longer a
  calibration-split blocker, but it remains a hold-out retrieval or target-label
  issue to resolve before calling the behavior mature.
- No-match behavior remains the primary threshold blocker. C10 is separable by
  pure semantic calibration, but H9 and H11 have semantic top scores
  `0.342626` and `0.405622`, respectively, which would survive the proposed
  semantic floor `0.328807`. H20 is lower at `0.255327`.
- The proposed-threshold simulation exposes a second blocker beyond the raw
  C10 branch overlap: the default final hybrid gates would regress several
  calibration expected-match rows after applying the proposed pre-fusion
  floors. Hybrid/auto would fall from 25 / 5 to 17 / 13 in the simulated
  proposal, so `--apply` remains unsafe even if the branch-overlap issue were
  solved.
- The run again took about six minutes and emitted very large GGUF loader logs.
  Query expansion currently reloads the GGUF context repeatedly during the
  hybrid/auto eval loop. That is acceptable for manual calibration evidence but
  is a clear tooling improvement before building a repeated tuning runner.
- No `--apply` was run. The threshold table remains seeded because the
  candidate calibration report is non-promotable.
- The selected run was exported into
  `raw/data/eval/natural-language-search/20260511T205231Z-79135/balanced/`
  with `eval-run.json`, `eval-calibration.json`, and a hash-bearing
  `manifest.toml`. The export was regenerated after the path-anchor simulation
  tuning and after PII redaction, so the committed raw evidence uses the
  corpus/run/candidate layout and contains no absolute home paths.

## Next Actions

1. Treat the framework and electric-car labels as the accepted baseline as of
   2026-05-12. Any future label edit, corpus change, retrieval change, model
   change, or chunking change must rerun `eval run` and `eval calibrate`.
2. Keep threshold scope in future apply/replay work. The current threshold
   store can hold project-scoped records, but future model-profile work should
   keep the same no-cross-corpus-clobbering invariant.
3. Keep C10, H9, H11, and H20 as explicit no-match sentinels and revalidate
   them whenever wiki content or query expansion changes.
4. Compare alternative accepted model/profile bundles with `--candidate-profile`
   or explicit model IDs once threshold scope is understood.
5. Reduce eval tooling cost before deriving a larger self-improving tool:
   suppress or route GGUF loader logs, reuse model contexts where possible, and
   make repeated candidate runs easier to compare.
6. Rerun `llm-wiki eval run` and `llm-wiki eval calibrate` after each retrieval
   or label change, then promote thresholds only after the C-split is
   promotable and the H-split remains acceptable to the human maintainer.

## Electric-Car Domain Corpus Confirmation (2026-05-12)

The electric-car domain corpus replay ran against
`tests/fixtures/eval-corpora/electric-cars/` with the balanced profile and
exported the raw bundle to
`raw/data/eval/electric-cars/20260512T162918Z-86751/balanced/`.

- Run: `20260512T162918Z-86751`
- Candidate: `balanced`
- Status: `promotable`
- Applied: yes, under the `electric-cars` threshold scope
- Current hybrid / auto: `17 / 3`
- Proposed hybrid / auto: `20 / 0`
- Hold-out hybrid / auto: `12 / 0`
- No-match precision, hybrid / auto: `3 / 3`
- Exact-identifier preservation, hybrid / auto: `5 / 5`
- Proposed `semantic_similarity_floor`: `0.571680`
- Proposed `hybrid_pre_fusion_semantic_floor`: `0.571680`
- Proposed `hybrid_final_semantic_floor`: `0.39`

This confirms that the calibration machinery can produce a promotable balanced
candidate on a second, domain-specific corpus. The resulting threshold store
keeps the framework-wiki floors under the `llm-wiki-framework-semantic-search`
scope and the electric-car floors under the `electric-cars` scope, so applying
one no longer clobbers the other.

<!-- llm-wiki-search-ignore-start -->

### 2026-05-12 Eval Calibration Run 20260512T091123Z-10597 (Superseded)

- Source run: `20260512T091123Z-10597`
- Report: `target/evals/20260512T091121Z-final/eval-calibration.json`
- Pre-record source fingerprint: `1e3c1c0f49ccaaacfd77dabb1e289be4cbe6015263571cf75801b35d3b16534f`
- Index stale warning: run `llm-wiki index --force` after this wiki mutation.

- Candidate: `balanced`
- Status: `promotable` in the original replay; superseded by anchor-aware
  validation.
- Promotable: `true` in the original replay; not durable promotion evidence.
- Post-apply validation required: `true`
- Proposed `semantic_similarity_floor`: `0.328807`
- Proposed `hybrid_pre_fusion_semantic_floor`: `0.103599`
- Current summary: `auto 26/4/0/0`, `hybrid 26/4/0/0`, `lexical 17/9/4/0`, `semantic 24/5/1/0`
- Proposed summary: `auto 30/0/0/0`, `hybrid 30/0/0/0`, `lexical 17/9/4/0`, `semantic 26/3/1/0`
- Hold-out summary: `auto 20/0/0/0`, `hybrid 20/0/0/0`, `lexical 13/4/3/0`, `semantic 17/3/0/0`
- Verdict changes: `12`
- Model artifact bytes: `1616029856`
- Candidate index bytes: `2840677`

- Applied: yes

Review follow-up on 2026-05-12: this run is superseded as durable promotion
evidence. Its replay counted path anchors only, while production hybrid search
uses path, title, and snippet anchor evidence when deciding whether a displayed
candidate survives threshold gates. The thresholds remain applied locally as
operational evidence, but the promotion decision must use the later
anchor-aware replay.

## Anchor-Aware Validation Follow-up (2026-05-12)

Post-apply validation reindexed the project with `cargo run -- index --project
llm-wiki-framework-semantic-search --force` and ran the ignored production
regression harness with `cargo test --test natural_language_search_eval --
--ignored --nocapture`. The broad regression harness passed, but the stricter
promotion gate failed: H11 ("browser automation plugin release checklist")
returned `wiki/log.md` in hybrid and auto.

The eval simulator was updated to record top-result anchor matches from the
same production-equivalent haystack: path, title, and snippet. The anchor-aware
run `20260512T145231Z-68005` reports:

- Candidate: `balanced`
- Status: `blocked_holdout_regression`
- Promotable: `false`
- Proposed `semantic_similarity_floor`: `0.328807`
- Proposed `hybrid_pre_fusion_semantic_floor`: `0.103599`
- Current summary: `auto 26/4/0/0`, `hybrid 26/4/0/0`, `lexical 17/9/4/0`, `semantic 24/5/1/0`
- Proposed summary: `auto 29/1/0/0`, `hybrid 29/1/0/0`, `lexical 17/9/4/0`, `semantic 26/3/1/0`
- Hold-out summary: `auto 19/1/0/0`, `hybrid 19/1/0/0`, `lexical 13/4/3/0`, `semantic 17/3/0/0`
- No-match precision: `hybrid 3/4`, `auto 3/4`
- Exact-identifier preservation: `hybrid 4/4`, `auto 4/4`
- Verdict changes: `8`

The electric-car domain-corpus confirmation was deferred until H11 could be
resolved; the follow-up run below unblocks it. A real reranker comparison is
also blocked in the current local model state:
the Qwen3 reranker artifact and accepted-license record are not present, so
`eval run --rerank` correctly remains a readiness failure for that profile.

## Anchor-Leak Final Floor Follow-up (2026-05-12)

The calibrator was updated again so no-match rows that survive only through
production-equivalent anchor evidence raise the proposed final hybrid semantic
floor. The new run `20260512T154322Z-77945` reports:

- Candidate: `balanced`
- Status: `promotable`
- Promotable: `true`
- Applied: yes
- Raw bundle:
  `raw/data/eval/natural-language-search/20260512T154322Z-77945/balanced/`
- Proposed `semantic_similarity_floor`: `0.328807`
- Proposed `hybrid_pre_fusion_semantic_floor`: `0.103599`
- Proposed `hybrid_final_semantic_floor`: `0.399904`
- Proposed `hybrid_semantic_only_floor`: `0.50`
- Proposed `hybrid_strong_lexical_score_floor`: `0.5`
- Current summary: `auto 26/4/0/0`, `hybrid 26/4/0/0`, `lexical 17/9/4/0`, `semantic 24/5/1/0`
- Proposed summary: `auto 30/0/0/0`, `hybrid 30/0/0/0`, `lexical 17/9/4/0`, `semantic 26/3/1/0`
- Hold-out summary: `auto 20/0/0/0`, `hybrid 20/0/0/0`, `lexical 13/4/3/0`, `semantic 17/3/0/0`
- No-match precision: `hybrid 4/4`, `auto 4/4`
- Exact-identifier preservation: `hybrid 4/4`, `auto 4/4`
- Verdict changes: `12`

Post-apply validation: after removing unignored catalog wording that described
the H11 validation itself, `cargo run -- index --project
llm-wiki-framework-semantic-search --force` rebuilt the project index and
`cargo test --test natural_language_search_eval -- --ignored --nocapture`
passed. H11 hybrid and auto both returned zero results with
`zero_result_reason` set to hybrid threshold filtering. Electric-car
domain-corpus confirmation is now the next promotion check. A real reranker
comparison still requires the Qwen3 reranker artifact and accepted-license
record.

<!-- llm-wiki-search-ignore-end -->
