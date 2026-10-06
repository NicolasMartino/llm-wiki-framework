# Plan: Default Hybrid Search and MCP Surface Repair

- Document Class: Plan
- Status: Active
- Date: 2026-06-27
- Updated: 2026-08-01
- Category: Search UX, hybrid retrieval, MCP surface, runtime correctness
- Scope: Make hybrid/semantic search work out of the box on any freshly indexed project (no manual calibration), and fix the genuine MCP-surface defects surfaced by the keto-diet field test — while correcting the wrong root-cause record left by the prior Phase 0.
- Sources:
  - raw/field-test/ketogenic_project_test.md (live keto-diet run, llm-wiki-test instance)
  - Direct CLI vs MCP differential (this investigation)
  - src/search/commands.rs, src/search/semantic.rs, src/search_models.rs, src/mcp/mod.rs, src/instance.rs, src/doctor.rs
- Related:
  - wiki/plans/mcp-first-agent-guidance.plan.md (its Phase 0 diagnoses are superseded here)
  - wiki/plans/semantic-hybrid-search.plan.md
  - wiki/checklists/mcp-field-test.checklist.md

## Problem

The keto-diet field test reported four "blockers" against the MCP surface. A
direct differential (run the CLI directly, then compare to the MCP output) shows
that the earlier diagnoses in `mcp-first-agent-guidance.plan.md` Phase 0 are
wrong, and the real defects are different and fewer.

Verified findings:

1. **The CLI is correct.** `llm-wiki-test search diet --mode lexical
   --project keto-diet --format json` returns 10 hits with non-null
   `document_class` ("Literature Note") and `status` ("Sourced"), snippets, and
   scores. The MCP server only shells out to this CLI and passes its JSON
   through untouched (`run_cli(...).json()`, `src/mcp/mod.rs:608`). So the
   field test's "zero hits", "null class/status", and "missing rerank_reason"
   are **not** code bugs. (They were later traced to an external
   context-compression layer mangling tool results, not to llm-wiki; the CLI
   output is authoritative.)
2. **Hybrid/semantic do not work out of the box.** On a freshly indexed project
   `mode: semantic` and `mode: hybrid` hard-fail with `thresholds_unconfigured`.
   The readiness message says to run `llm-wiki eval calibrate --record`, but
   `eval calibrate` requires a hand-authored `--eval-page` of golden queries
   (`src/cli.rs:90`, required), which a fresh project does not have. The guidance
   is a dead end, and hybrid — a core differentiator of this project — is
   effectively unavailable until a user writes an eval set.
3. **One real cross-project bug.** `search-all` with no filter hard-fails the
   entire run when any one registered project lacks an index (reproduced on the
   CLI): `perform_project_search` uses `bail!` for a missing/corrupt
   index (`src/search/commands.rs:3060`, `:3075`), which escapes the
   per-project `BackendReadinessFailure` catch in `search_all` and aborts.
4. **Secondary surface gaps.** The MCP `status` tool returns only install
   metadata (`src/mcp/mod.rs:782`) — it cannot tell an agent whether the current
   project is registered, indexed, fresh, or search-ready. `raw/` has no
   discovery path through MCP (no list/tree; directory read errors), so an agent
   restricted to MCP cannot find raw files. Read errors leak `canonicalize
   <path>` for both missing-file and traversal cases instead of distinct,
   actionable messages.

## Deliverable

- Any project that has been `register`ed and `index`ed can immediately run
  `mode: semantic` and `mode: hybrid` and get real results, with **no manual
  calibration step**. Per-project `eval calibrate` becomes an optional quality
  refinement that overrides shipped defaults, never a gate.
- `search-all` degrades per-project instead of aborting on one missing index.
- The MCP `status` tool reports per-project readiness; `raw/` is discoverable
  through MCP; read errors are distinct and actionable.
- The wrong Phase 0 diagnostic record is corrected so future work does not chase
  a metadata-parser or profile-rendering bug that does not exist.

## Non-Goals

- Do not weaken calibrated-threshold quality. Shipped defaults are a floor that
  makes hybrid usable everywhere; recorded per-project calibration still wins
  when present.
- Do not turn `llm_wiki_read` into a general file reader outside `wiki/`/`raw/`.
- Do not re-enable generated skill projection or change the document model.
- Do not block this plan on the broader MCP-first guidance push
  (`mcp-first-agent-guidance.plan.md`); that work resumes once this lands.

## Priority and sequencing

Hybrid-always-works (Phase 1) is the headline deliverable and ships first.
Phases 4–6 are independent correctness/UX fixes that can land in any order.

## Implementation progress

Status as of 2026-06-27:

- Implemented: built-in default thresholds for uncalibrated semantic/hybrid
  search, semantic vector indexing without a recorded thresholds file,
  `search-all` per-project degradation for missing/unusable indexes, MCP
  `status` project readiness fields, MCP `llm_wiki_read` directory listings for
  `wiki/` and `raw/`, clearer missing-path read errors, and `thresholds_source`
  provenance in search JSON.
- Verified: `cargo test --workspace` passes with 333 tests passed and 2
  ignored after focused coverage for search defaults, search-all readiness, and
  MCP status/read directory behavior.
- Updated 2026-06-29: a second live keto-diet field test reconfirmed via
  CLI-direct that Phases 0–6 hold — real Markdown and real scored hits with
  non-null `document_class`/`status`. The run surfaced **one genuine CLI bug**
  (reproduced CLI-direct): `mode: hybrid` with `rerank: true` and no reranker
  model hard-failed the whole search with `readiness_reason: "model_missing"`,
  even though hybrid-without-rerank was ready. Fixed by making the reranker
  optional at the readiness gate (`required_model_ids` no longer lists it) and
  degrading `maybe_rerank_results` to a no-op when unconfigured, so the search
  returns hits with `rerank_applied: false` /
  `rerank_reason: "reranker_model_unconfigured"` (Issue 0.4 intent). Regression
  test: `reranker_is_not_a_search_readiness_requirement`.
- Updated 2026-08-01: the production `llm-wiki 0.2.15` Codex MCP field test
  against `/Users/nicolasmartino/Documents/keto_diet` reconfirmed the shipped
  behavior in the ready-index posture: `llm_wiki_read` returned real wiki/raw
  Markdown with stable hashes, lexical/auto/semantic/hybrid search returned
  populated results, default thresholds worked without calibration, class/status
  filters selected matching pages, rerank no-op reporting was explicit, and
  `search-all` degraded per project while preserving ready-project hits. The
  submitted Step 18 `BUG` row was reclassified as a checklist/API contract
  wording issue: `search-all` reports readiness in `projects[]` and returns a
  flat globally ranked `results[]` list with project attribution.
- Updated 2026-08-01: the Claude production `llm-wiki 0.2.15` field test
  independently confirmed the same ready-index MCP surface, this time with all
  19 checklist steps reported as passing and no blocking `BUG` rows. It also
  records one minor API-hardening follow-up: `llm_wiki_search_all` silently
  ignored obsolete `include_projects` / `exclude_projects` keys, while the
  documented `include` / `exclude` keys worked correctly.

## Phase 0 - Correct the diagnostic record

Before fixing anything, stop the codebase from carrying false diagnoses.

Tasks:

1. Supersede Issues 0.1, 0.2, and 0.4 in
   `wiki/plans/mcp-first-agent-guidance.plan.md`: read CCR, null class/status,
   and missing rerank_reason are **not** bugs in `src/instance.rs`,
   `src/search/metadata.rs`, or the serializer — the CLI output is authoritative.
   Mark them resolved-by-this-plan with the verified evidence, or add a
   "Correction" note pointing here. Do not silently delete the prior text.

Verification: the prior plan no longer asserts a metadata-parser or
profile-rendering defect as open; `wiki/log.md` notes the correction.

## Phase 1 - Hybrid search works out of the box (headline)

Make `semantic`/`hybrid` ready on any freshly indexed project by shipping
built-in default thresholds, with calibration as an optional override.

Design:

- Today readiness fails at `SearchThresholdStore::read(...) -> None`
  (`src/search/commands.rs:1298`) → `thresholds_unconfigured`. The store is
  matched to an index by `thresholds_match_index_inputs`
  (`src/search/semantic.rs:533`), which only compares fields that are known at
  index time: `qmd_rs_version`, `adapter_schema_version`, `profile`,
  `chunking_strategy`, `embedding_model`, `embedding_artifact_sha256`,
  `embedding_dimensions`. Nothing in the match requires human-labeled data.
- Therefore a **default `SearchThresholds` can be synthesized at runtime** by
  stamping those fields from the live index metadata + embedding artifact, with
  conservative hand-tuned floor values. The floor defaults already exist as
  named constants (`default_hybrid_final_semantic_floor` 0.39,
  `default_hybrid_semantic_only_floor` 0.50,
  `default_hybrid_strong_lexical_score_floor` 10.0) and serve as the starting
  point; `semantic_similarity_floor`, `hybrid_pre_fusion_semantic_floor`, and
  `reranker_probability_floor` get baked defaults per embedding model.
- Decision note: the shipped defaults intentionally leave
  `semantic_similarity_floor` and `hybrid_pre_fusion_semantic_floor` at `0.0`
  for the initial embedding model so fresh projects are not gated before
  calibration. Hybrid quality is controlled by the final hybrid,
  semantic-only, and strong-lexical floors; recorded project calibration remains
  the precision path.

Tasks:

1. Add a built-in default threshold table keyed by embedding model id (initially
   `embeddinggemma-300m-q8_0`, 768-dim) in `src/search_models.rs`, holding the
   floor values only.
2. In threshold resolution, when no recorded thresholds match the index inputs,
   synthesize a default `SearchThresholds` from the table + the live index
   metadata/artifact (so `thresholds_match_index_inputs` passes), marked as
   `source: default` (vs `recorded`). Recorded per-project thresholds, then
   recorded unscoped, always take precedence over the default.
3. Build the semantic vector index during `index` whenever the embedding model
   is materialized, instead of skipping it when thresholds are unconfigured
   (`update_semantic_index_metadata`, `src/search/commands.rs:225` currently
   returns early when the store is missing). With shipped defaults there is
   always a usable threshold, so the vector index should always build.
4. Surface provenance in the search JSON: add a field indicating thresholds came
   from `default` vs `recorded` calibration, so callers can see when results use
   shipped defaults.
5. Keep `eval calibrate --record` as the override path and fix its readiness
   guidance to say calibration is optional tuning, not a prerequisite.

Touchpoints: `src/search_models.rs` (default table + `SearchThresholds`
synthesis), `src/search/commands.rs` (`readiness_failure`,
`update_semantic_index_metadata`, search JSON), `src/search/semantic.rs`
(default-aware selection), readiness guidance strings.

Verification:

- A focused test: a freshly indexed project with the embedding model present but
  **no** `search-thresholds.toml` resolves to a `default`-sourced threshold and
  `mode: hybrid` / `mode: semantic` return results (not
  `thresholds_unconfigured`).
- A recorded per-project threshold still overrides the default (precedence test).
- Live re-run on keto-diet: `mode: hybrid "ketogenic diet research"` returns
  hits with `thresholds_source: default`.

## Phase 4 - search-all degrades per-project instead of aborting

Tasks:

1. Change `perform_project_search` (`src/search/commands.rs:3043`) so the
   `Missing` and `ForceReindex`/corrupt cases return a typed
   `BackendReadinessFailure` (reasons e.g. `index_missing`, `index_unusable`)
   carrying the backend status, instead of `bail!`.
2. Confirm `search_all`'s existing catch turns those into a per-project
   readiness report (`result_count: 0`, `readiness_reason`) and continues; the
   single-project `search` path already prints a JSON readiness object for
   `BackendReadinessFailure`, so behavior there stays equivalent or improves.

Touchpoints: `src/search/commands.rs` (the `perform_project_search` match;
verify `search` and `search_all` error handling).

Verification: a test where one of several registered projects has no index —
`search-all` returns hits for the indexed projects plus a per-project readiness
entry for the missing one, exit 0; the single-project `search` on the missing
project still reports `index_missing` readiness.

## Phase 5 - MCP status reports project readiness

Tasks:

1. Extend `status_payload` (`src/mcp/mod.rs:782`) / the status tool so, given an
   optional `project` (or the discoverable current project), it reports:
   registered (y/n), index present + freshness, and search readiness
   (lexical-ready always; semantic/hybrid-ready now that Phase 1 ships defaults).
   Keep the existing install fields for backward compatibility.
2. Reuse the existing registry/backend status surfaces (the CLI `projects`
   command already computes `index_status`/`freshness`).

Touchpoints: `src/mcp/mod.rs` (status payload + tool schema), status tests.

Verification: MCP `status` on a registered+indexed project reports registered,
index fresh, and semantic/hybrid ready; on an unregistered project reports not
registered without erroring.

## Phase 6 - raw discoverability and read error clarity

Lower stakes; lands after the P1 work.

Tasks:

1. Give the MCP surface a way to enumerate `wiki/`/`raw/` contents — either a
   list/tree tool or make `llm_wiki_read` on a directory return its children —
   so an MCP-restricted agent can find raw files without the filesystem.
2. Make read errors distinct: not-found vs path-escapes-project vs
   permission-denied, instead of the raw `canonicalize <path>` leak seen in
   field-test steps 8 and 9.

Touchpoints: `src/mcp/mod.rs` (read handler / new list capability), the wiki
read implementation, read tests.

Verification: reading a directory lists children (or a list tool returns the
raw tree); a missing file returns a "not found" error and a traversal attempt
returns a distinct "outside project" rejection.

## Acceptance Criteria

1. A registered+indexed project with no `search-thresholds.toml` returns results
   for `mode: hybrid` and `mode: semantic`, tagged `thresholds_source: default`;
   recorded calibration overrides the default when present.
2. `search-all` with one unindexed registered project returns per-project
   readiness and exits 0.
3. MCP `status` reports per-project registration, index freshness, and search
   readiness.
4. The prior Phase 0 diagnostic record is corrected; `wiki/log.md` records this
   plan and the correction.
5. Each fix ships with a focused Rust test; the keto-diet field test re-runs
   clean for read, lexical, and hybrid.

## Closure

This plan closes when hybrid/semantic work on a freshly indexed project with no
manual calibration, `search-all` degrades per-project, MCP `status` reports
readiness, the raw/read-error gaps are addressed, and a clean keto-diet re-run
plus focused tests prove all of the above. The broader MCP-first guidance push
resumes in `wiki/plans/mcp-first-agent-guidance.plan.md` once this lands.
