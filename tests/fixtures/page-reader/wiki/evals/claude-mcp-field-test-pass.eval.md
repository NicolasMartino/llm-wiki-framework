# Eval: Claude MCP Field Test Pass

- Document Class: Eval
- Status: Accepted
- Date: 2026-07-07
- Updated: 2026-08-01
- Category: MCP surface, Claude host, field test
- Scope: Records clean Claude runs of the rewritten pure-MCP
  `wiki/checklists/mcp-field-test.checklist.md` against the `keto_diet`
  project: the original 2026-07-07 `llm-wiki-test` MCP run and the
  2026-08-01 production `llm-wiki 0.2.15` MCP run. These are the
  Claude-host counterparts to the Codex passes: no Headroom, no proxy, no CCR
  framing - only the `llm_wiki_*` surface exercised end-to-end.
- Sources:
  - User-provided Claude results file pasted 2026-07-07
    (workspace `/Users/nicolasmartino/Documents/keto_diet`).
  - User-provided production Claude MCP field-test results pasted 2026-08-01
    (production `llm-wiki 0.2.15`, workspace
    `/Users/nicolasmartino/Documents/keto_diet`).
  - wiki/checklists/mcp-field-test.checklist.md
- Related:
  - wiki/checklists/mcp-field-test.checklist.md
  - wiki/evals/codex-mcp-field-test-pass.eval.md
  - wiki/references/headroom-context-compression.reference.md

## Verdict

Accepted pure-MCP correctness pass for Claude in an existing already-registered
project posture.

Updated 2026-08-01: the production unsuffixed MCP instance also passed the
field-test checklist against the same external project using `llm-wiki 0.2.15`.
The run reports all 19 steps as passing, with no blocking `BUG` rows. Its only
new follow-up is minor: `llm_wiki_search_all` silently ignored unknown
`include_projects` / `exclude_projects` keys, while the documented
`include` / `exclude` keys worked correctly.

Unlike an earlier Claude run where a Headroom proxy was active and
`llm_wiki_read_test` returned `<<ccr:...>>` (Rejected, since removed), this run
has no proxy in the path: reads return real Markdown and every search mode
returns populated payloads.

The run satisfies every load-bearing condition of the pure-MCP field test:

- `llm_wiki_read_test` returned real wiki and raw Markdown with stable
  `sha256`/`byte_len` (relative and absolute paths produced an identical hash).
- Traversal escapes and a missing file were rejected with clear, distinct
  errors — outside-tree vs not-found — not file contents.
- Lexical, auto, semantic, hybrid, class/status-filtered search, rerank
  diagnostics, and `search-all` include/exclude all behaved as expected.
- Semantic and hybrid worked on shipped `thresholds_source:"default"` after
  indexing, with no `thresholds_unconfigured` hard failure.
- The rerank no-op was explicit (`reranker_model_unconfigured`), never silent.
- No `BUG` rows.

The 2026-07-07 report author scored Step 1 as `OK` (a healthy re-run posture)
rather than `expected-fresh`; the project was already registered and indexed
before Step 1. That run left the not-ready fallback branch (Step 17) and
broader multi-project `search-all` narrowing behavior (Steps 18-19) as
non-blocking follow-ups. The 2026-08-01 production run now covers
cross-project readiness, mixed stale/missing-root project handling, and
include/exclude narrowing. Not-ready fallback remains intentionally unforced.
Updated 2026-08-01: `search-all` grouping is no longer expected; the accepted
contract is per-project readiness in `projects[]` plus flat top-level results
carrying project attribution.

## Production 0.2.15 Addendum (2026-08-01)

### Verdict

Accepted production MCP correctness pass for Claude in the existing ready-index
posture. This run used the installed production instance, not the `_test`
server:

- Instance: production MCP instance (`llm_wiki_*` tools)
- Binary stem/version: `llm-wiki 0.2.15`
- Managed home: `/Users/nicolasmartino/.llm_wiki`
- Host/platform: Darwin 25.5.0 / macOS
- Workspace: `/Users/nicolasmartino/Documents/keto_diet`
- Project: `keto-diet` / `keto diet`
- Store backend: qmd-rs read-only immutable store
- Models: `embeddinggemma-300m-q8_0` and `qmd-query-expansion-1.7b-q4_k_m`

Load-bearing results:

- `llm_wiki_status` reported the project registered, fresh, ready, and
  searchable across lexical, semantic, and hybrid modes with 10 indexed files.
- `llm_wiki_register` rejected the wrong parameter key (`project_root`) with a
  clear CLI error, then accepted the documented `path` key idempotently.
- Normal and forced indexing both completed with `Indexed project: keto-diet
  (10 files)`.
- `llm_wiki_read` returned real Markdown for `wiki/index.md` and the raw
  research manifest with the expected hashes:
  `619ddffe7a9135b3d56c01d9c54aaa71fc2b7c005b2a3dcd1bd73a31cde833a4` and
  `26a7d46b443e5de17d353d6dd71883edfb9a0da2b6502b3a728791392a9c98c7`.
- Relative/absolute reads matched, traversal attempts were rejected as
  outside-tree, and missing in-tree files returned a distinct not-found error.
- Lexical, auto, semantic, hybrid, class/status-filtered search, and
  `allow_lexical_fallback` in a ready posture returned the expected parseable
  payloads. Auto selected hybrid; semantic/hybrid used
  `thresholds_source:"default"` without calibration.
- `rerank:true` returned `rerank_applied:false` with
  `rerank_reason:"reranker_model_unconfigured"`; `rerank:false` returned no
  spurious reason.
- `llm_wiki_search_all` reported mixed project readiness in `projects[]`,
  returned flat top-level attributed hits, warned about stale/missing-root
  projects, and honored the documented `include` / `exclude` filters.

### Production 0.2.15 Report Summary

| Step | Verdict | Notes |
| --- | --- | --- |
| 1 status | OK | Already registered, fresh, ready, and indexed. |
| 2 register | OK | Wrong `project_root` key produced a clear error; correct `path` key was idempotent. |
| 3 index / --force | OK | Both calls reported `Indexed project: keto-diet (10 files)`. |
| 4 calibrate | expected-fresh | Optional tuning skipped; default thresholds were tested. |
| 5 wiki read | OK | Real Markdown, `byte_len:2593`, matching hash. |
| 6 raw read | OK | Real raw Markdown, `byte_len:2689`, matching hash. |
| 7 abs/rel path | OK | Both reads normalized and matched. |
| 8 traversal | OK | Escapes rejected as outside wiki/raw. |
| 9 missing file | OK | Clear not-found error. |
| 10 lexical | OK | Populated lexical hits with metadata where applicable. |
| 11 auto | OK | Auto selected hybrid with default thresholds. |
| 12 semantic | OK | Populated semantic hits with default thresholds. |
| 13 hybrid | OK | Populated hybrid hits with default thresholds. |
| 14 class/status filters | OK | Filters narrowed to matching pages. |
| 15 rerank true | OK | Explicit `reranker_model_unconfigured` reason. |
| 16 rerank false | OK | Rerank disabled cleanly. |
| 17 allow_lexical_fallback | OK / expected-fresh | Ready project stayed semantic; not-ready fallback was not manufactured. |
| 18 search-all | OK | Per-project readiness plus flat attributed top-level results. |
| 19 include/exclude | OK | Correct `include` / `exclude` keys changed the searched set. |

### Follow-Up

Issue A: `llm_wiki_search_all` silently accepts unknown project-filter keys.
The initial `include_projects` / `exclude_projects` attempts were ignored and
returned unfiltered results; the documented `include` / `exclude` keys worked.
This is minor because the intended API is functional, but it can mask caller
typos. Preferred repair: reject unknown arguments or echo ignored-argument
warnings in the tool result.

## Run Header

- Date: `2026-07-07`
- Instance: test instance (`_test` tools), server `mcp__llm-wiki-test`
- Binary stem: `llm-wiki-test`
- Version: `0.2.8`
- Managed home: `/Users/nicolasmartino/.llm_wiki-test`
- Manifest: `/Users/nicolasmartino/.llm_wiki-test/manifest.json`
- Host: Darwin 25.2.0 arm64 (Metal / llama.cpp `MTL0` backend)
- Embedding model: `embeddinggemma-300m-q8_0`
- Query-expansion model: `qmd-query-expansion-1.7b-q4_k_m`
- Index store: `/Users/nicolasmartino/.llm_wiki-test/indexes/keto-diet/qmd-rs.sqlite`
- Workspace root: `/Users/nicolasmartino/Documents/keto_diet`
- Project: `keto-diet` / `keto diet`, 10 indexed files
- Posture tested: existing project already registered and indexed on the `_test`
  instance from a prior run; register and index were exercised idempotently.
- Content access rule: all wiki/raw content was accessed exclusively through the
  `llm-wiki-test` MCP tools; no shell/filesystem reads.

## Per-Step Results

### Step 1 - Status

- Tool: `llm_wiki_status_test`
- Arguments: `{}`
- Verdict: `OK`

Raw signal:

```json
{
  "installed": true,
  "binary_stem": "llm-wiki-test",
  "version": "0.2.8",
  "project": {
    "registered": true,
    "project_id": "keto-diet",
    "project_name": "keto diet",
    "project_root": "/Users/nicolasmartino/Documents/keto_diet",
    "index_present": true,
    "index_freshness": "fresh",
    "index_state": "ready",
    "indexed_files": 10,
    "lexical_ready": true,
    "semantic_ready": true,
    "hybrid_ready": true,
    "readiness_reason": null
  }
}
```

Install/registration/readiness surfaces all report cleanly. The project was
already registered with a fresh ready index — consistent with a re-run posture,
scored `OK`.

### Step 2 - Register

- Tool: `llm_wiki_register_test`
- Arguments: `{"path":"/Users/nicolasmartino/Documents/keto_diet"}`
- Verdict: `OK`

Raw signal:

```json
{"stdout":"Project already registered: keto-diet\n","stderr":""}
```

Register was idempotent; the follow-up status still reported `registered:true`.

### Step 3 - Index / Force Index

- Tool: `llm_wiki_index_test`
- Arguments: `{}` and `{"force":true}`
- Verdict: `OK`

Both calls returned:

```text
Indexed project: keto-diet (10 files)
```

`force:true` re-ran the full build (fresh llama.cpp Metal pipeline compile and
reserve visible in stderr, new buffer addresses `0x70ce…` → `0x95ae…`). The
stderr is embedding-model runtime noise behind a host omission marker
(`[58297 bytes of earlier stderr omitted]`), not an error; the indexed-file
stdout was present and stable.

### Step 4 - Calibrate Thresholds

- Tool: not run (CLI step)
- Verdict: `expected-fresh` (skipped by design)

Calibration is optional quality tuning, not a readiness gate. It was not run, so
recorded thresholds were intentionally not exercised. Steps 11-13 and 17 confirm
default thresholds suffice (`thresholds_source:"default"`).

### Step 5 - Wiki Read

- Tool: `llm_wiki_read_test`
- Arguments: `{"path":"wiki/index.md"}`
- Verdict: `OK`

Raw signal:

```json
{
  "path": "wiki/index.md",
  "tree": "wiki",
  "scope": "wiki",
  "byte_len": 2593,
  "bytes": 2593,
  "sha256": "619ddffe7a9135b3d56c01d9c54aaa71fc2b7c005b2a3dcd1bd73a31cde833a4",
  "encoding": "utf-8",
  "content_prefix": "# Wiki Index\n\nProject: keto diet\nStage: Evidence intake\nUpdated: 2026-06-25"
}
```

Real Markdown with full provenance (`wiki_root`/`raw_root`/`tree`/`scope`) and a
concrete `sha256`/`byte_len` — not an empty or paraphrased body.

### Step 6 - Raw Read

- Tool: `llm_wiki_read_test`
- Arguments:
  `{"path":"raw/research/2026-06-25-keto-review-meta-analysis-first-pass/manifest.md"}`
- Verdict: `OK`

Raw signal:

```json
{
  "path": "raw/research/2026-06-25-keto-review-meta-analysis-first-pass/manifest.md",
  "tree": "raw",
  "scope": "raw",
  "byte_len": 2689,
  "bytes": 2689,
  "sha256": "26a7d46b443e5de17d353d6dd71883edfb9a0da2b6502b3a728791392a9c98c7",
  "encoding": "utf-8",
  "content_prefix": "# Research Manifest: Keto Review And Meta-Analysis First Pass"
}
```

Real raw-source content under `raw/`, including a fenced PubMed query block and a
Markdown table.

### Step 7 - Absolute Vs Relative Path

- Tool: `llm_wiki_read_test`
- Relative arguments: `{"path":"wiki/index.md"}`
- Absolute arguments:
  `{"path":"/Users/nicolasmartino/Documents/keto_diet/wiki/index.md"}`
- Verdict: `OK`

Both forms succeeded and were byte-identical (`byte_len:2593`,
`sha256:619ddffe…833a4`). The absolute path normalized back to
`path:"wiki/index.md"`.

### Step 8 - Traversal Rejection

- Tool: `llm_wiki_read_test`
- Arguments: `{"path":"../../etc/hosts"}` and `{"path":"/etc/hosts"}`
- Verdict: `OK`

Raw signals:

```json
{"error":{"message":"../../etc/hosts is outside the project wiki/ or raw/ trees"}}
{"error":{"message":"/etc/hosts is outside the project wiki/ or raw/ trees"}}
```

Both escapes returned an explicit outside-tree message, not file contents and
not a generic not-found. The relative `../../etc/hosts` case specifically
returned the outside-tree rejection (the exact condition Issue 8 guards against).

### Step 9 - Missing File Error

- Tool: `llm_wiki_read_test`
- Arguments: `{"path":"wiki/does-not-exist.md"}`
- Verdict: `OK`

Raw signal:

```json
{"error":{"message":"wiki/does-not-exist.md not found"}}
```

A path inside the wiki tree but absent returned a clear not-found error, distinct
from the outside-tree message in Step 8.

### Step 10 - Lexical Search

- Tool: `llm_wiki_search_test`
- Arguments: `{"query":"epilepsy","mode":"lexical"}`
- Verdict: `OK`

Raw signal:

```json
{
  "selected_mode": "lexical",
  "mode_selection_reason": "explicit_lexical",
  "backend_status": {"state": "ready", "freshness": "fresh", "indexed_files": 10},
  "thresholds_source": null,
  "results_count": 6,
  "results": [
    {"path": "wiki/literature/dietary-therapies-childhood-drug-resistant-epilepsy.literature-note.md", "document_class": "Literature Note", "status": "Sourced"},
    {"path": "wiki/index.md", "document_class": null, "status": null},
    {"path": "wiki/log.md", "document_class": null, "status": null},
    {"path": "wiki/checklists/review-source-screening.checklist.md", "document_class": "Checklist", "status": "Active"},
    {"path": "wiki/roadmaps/keto-evidence-and-app-foundation.roadmap.md", "document_class": "Roadmap", "status": "Active"},
    {"path": "wiki/literature/keto-health-outcomes-umbrella-review.literature-note.md", "document_class": "Literature Note", "status": "Sourced"}
  ]
}
```

Populated 6-hit array on a ready index. Header-bearing pages return non-null
`document_class`/`status` (`Literature Note`/`Sourced`, `Checklist`/`Active`,
`Roadmap`/`Active`); header-less pages (`wiki/index.md`, `wiki/log.md`) correctly
return null. `thresholds_source:null` is expected for pure lexical BM25, and
`runtime_backend_*` are null because no runtime embedding backend is selected.

### Step 11 - Auto Search

- Tool: `llm_wiki_search_test`
- Arguments:
  `{"query":"What does the evidence say about ketogenic diets for type 2 diabetes remission?","mode":"auto"}`
- Verdict: `OK`

Raw signal:

```json
{
  "requested_mode": "auto",
  "selected_mode": "hybrid",
  "mode_selection_reason": "enabled_profile",
  "runtime_backend_used": "auto",
  "thresholds_source": "default",
  "runtime_backend_fallback": false,
  "zero_result_reason": null,
  "results_count": 8,
  "top_hit": {"path": "wiki/literature/low-carb-type-2-diabetes-remission-meta-analysis.literature-note.md", "semantic_rank": 0, "semantic_score": 0.6791427135467529}
}
```

Auto resolved to hybrid with default thresholds — no `thresholds_unconfigured`
error — and the most on-topic remission note ranked first.

### Step 12 - Semantic Search

- Tool: `llm_wiki_search_test`
- Arguments:
  `{"query":"ketogenic diet effects on weight loss and glycemic control","mode":"semantic"}`
- Verdict: `OK`

Raw signal:

```json
{
  "requested_mode": "semantic",
  "selected_mode": "semantic",
  "backend": "qmd-rs-semantic",
  "thresholds_source": "default",
  "runtime_backend_fallback": false,
  "zero_result_reason": null,
  "results_count": 10
}
```

Semantic search returned a populated 10-hit array with sensible ordering
(overweight-T2D and umbrella-review notes on top for a weight/glycemic query) on
default thresholds; no `thresholds_unconfigured` failure.

### Step 13 - Hybrid Search

- Tool: `llm_wiki_search_test`
- Arguments:
  `{"query":"LDL cholesterol response to low carbohydrate diets","mode":"hybrid"}`
- Verdict: `OK`

Raw signal:

```json
{
  "requested_mode": "hybrid",
  "selected_mode": "hybrid",
  "backend": "qmd-rs-hybrid",
  "thresholds_source": "default",
  "results_count": 2,
  "top_hit": {"path": "wiki/literature/low-carb-ldl-bmi-meta-analysis.literature-note.md", "lexical_rank": 0, "lexical_score": 2.5661298387045344, "semantic_rank": 0, "semantic_score": 0.6012542843818665}
}
```

The exact LDL-C/BMI note ranked first with both `lexical_rank:0` and
`semantic_rank:0` — genuine lexical+semantic fusion. Hybrid returned fewer hits
than semantic because default relevance thresholds drop weak-overlap documents
(expected filtering, not a bug).

### Step 14 - Class And Status Filters

- Tool: `llm_wiki_search_test`
- Arguments:
  `{"query":"ketogenic diet type 2 diabetes","mode":"hybrid","class":"Literature Note","status":"Sourced"}`
- Verdict: `OK`

Raw signal:

```json
{
  "selected_mode": "hybrid",
  "thresholds_source": "default",
  "results_count": 5,
  "all_hits_document_class": "Literature Note",
  "all_hits_status": "Sourced"
}
```

Filter values were taken from real header values observed in Steps 10-13. The
result set is exactly the 5 matching Literature-Note/Sourced pages; the
`Roadmap`/`Active` page and the header-less `wiki/index.md` that appeared in the
unfiltered Step 11 are correctly excluded. The filter selects matching pages
rather than emptying the set.

### Step 15 - Rerank True

- Tool: `llm_wiki_search_test`
- Arguments:
  `{"query":"LDL cholesterol response to low carbohydrate diets","mode":"hybrid","rerank":true}`
- Verdict: `OK`

Raw signal:

```json
{
  "rerank_requested": true,
  "rerank_applied": false,
  "rerank_reason": "reranker_model_unconfigured",
  "reranker_model": null
}
```

No reranker configured on this instance, so the server declined with a named
reason rather than silently no-op'ing. Results were identical to un-reranked
Step 13.

### Step 16 - Rerank False

- Tool: `llm_wiki_search_test`
- Arguments:
  `{"query":"LDL cholesterol response to low carbohydrate diets","mode":"hybrid","rerank":false}`
- Verdict: `OK`

Raw signal:

```json
{
  "rerank_requested": false,
  "rerank_applied": false,
  "rerank_reason": null
}
```

No reason emitted when reranking was not requested (correct). Results matched
Steps 13/15.

### Step 17 - Allow Lexical Fallback

- Tool: `llm_wiki_search_test`
- Arguments:
  `{"query":"ketogenic diet effects on weight loss and glycemic control","mode":"semantic","allow_lexical_fallback":true}`
- Verdict: `OK` (ready-posture branch; not-ready branch not reproduced)

Raw signal:

```json
{
  "requested_mode": "semantic",
  "selected_mode": "semantic",
  "backend": "qmd-rs-semantic",
  "fallback_reason": null,
  "runtime_backend_fallback": false,
  "thresholds_source": "default",
  "results_count": 10
}
```

Despite `allow_lexical_fallback:true`, the ready index served true semantic
results and did not force a lexical fallback merely because calibration was
absent. The not-ready fallback branch was out of scope for this ready instance
(see Follow-Up F1).

### Step 18 - Search-All

- Tool: `llm_wiki_search_all_test`
- Arguments: `{"query":"ketogenic diet type 2 diabetes remission","mode":"hybrid"}`
- Verdict: `OK`

Raw signal:

```json
{
  "project_id": null,
  "project_name": null,
  "projects": [
    {
      "project_id": "keto-diet",
      "selected_mode": "hybrid",
      "thresholds_source": "default",
      "result_count": 6,
      "readiness_reason": null,
      "zero_result_reason": null,
      "backend_status": {"state": "ready", "freshness": "fresh", "indexed_files": 10}
    }
  ],
  "results_count": 6
}
```

Correct cross-project shape: a null top-level aggregate envelope, a per-project
`projects[]` entry for `keto-diet` carrying its own `backend_status`/readiness,
and a merged 6-hit `results` array each tagged `project_id:"keto-diet"`.
Readiness is reported per project (single group; only one project registered).

### Step 19 - Include / Exclude

- Tool: `llm_wiki_search_all_test`
- Include arguments:
  `{"query":"ketogenic diet type 2 diabetes remission","mode":"hybrid","include":["keto-diet"]}`
- Exclude arguments:
  `{"query":"ketogenic diet type 2 diabetes remission","mode":"hybrid","exclude":["keto-diet"]}`
- Verdict: `OK`

Include kept the single registered project (identical 6-hit result set to Step
18). Exclude removed the only registered project:

```json
{
  "error": {
    "message": "llm-wiki search-all ketogenic diet type 2 diabetes remission --format json --mode hybrid --exclude keto-diet failed with status exit status: 1: Error: no registered projects selected\n"
  }
}
```

The filters demonstrably changed the searched set. Exclude returned an explicit
empty-selection error rather than silently returning stale results.

## Report

| Step | Tool | Verdict | Notes |
| --- | --- | --- | --- |
| 1 status | `llm_wiki_status_test` | OK | Registered, fresh, ready, 10 indexed files, all modes ready. |
| 2 register | `llm_wiki_register_test` | OK | Idempotent `Project already registered: keto-diet`; status re-check registered. |
| 3 index / --force | `llm_wiki_index_test` | OK | Both calls `Indexed project: keto-diet (10 files)`; force re-ran full Metal pipeline. |
| 4 calibrate | not run | expected-fresh | Optional tuning skipped; defaults verified via `thresholds_source:"default"`. |
| 5 wiki read | `llm_wiki_read_test` | OK | Real Markdown, `byte_len:2593`, `sha256:619ddffe...`. |
| 6 raw read | `llm_wiki_read_test` | OK | Real raw content, `byte_len:2689`, `sha256:26a7d46b...`. |
| 7 abs/rel path | `llm_wiki_read_test` | OK | Both succeed; identical `sha256`/`byte_len`; abs normalized to `wiki/index.md`. |
| 8 traversal | `llm_wiki_read_test` | OK | Relative and absolute escapes rejected as outside wiki/raw. |
| 9 missing file | `llm_wiki_read_test` | OK | Clear not-found, distinct from outside-tree. |
| 10 lexical | `llm_wiki_search_test` | OK | 6 hits; header pages non-null class/status, `index.md`/`log.md` null. |
| 11 auto | `llm_wiki_search_test` | OK | `enabled_profile` → hybrid, default thresholds, 8 hits. |
| 12 semantic | `llm_wiki_search_test` | OK | `qmd-rs-semantic`, default thresholds, 10 hits. |
| 13 hybrid | `llm_wiki_search_test` | OK | `qmd-rs-hybrid`, default thresholds, 2 hits with lexical+semantic ranks. |
| 14 class/status filters | `llm_wiki_search_test` | OK | Literature Note/Sourced → exactly 5 matching pages; Roadmap + index.md excluded. |
| 15 rerank true | `llm_wiki_search_test` | OK | `rerank_applied:false`, `rerank_reason:"reranker_model_unconfigured"`. |
| 16 rerank false | `llm_wiki_search_test` | OK | `rerank_applied:false`, `rerank_reason:null`. |
| 17 allow_lexical_fallback | `llm_wiki_search_test` | OK / ready-branch | Stayed semantic, no fallback; not-ready branch not reproduced. |
| 18 search-all | `llm_wiki_search_all_test` | OK | Per-project group for `keto-diet` (`result_count:6`) + merged 6-hit results. |
| 19 include/exclude | `llm_wiki_search_all_test` | OK | Include → same 6 hits; exclude → `no registered projects selected`. |

## Pass Criteria

- Issue 0.1 (read fidelity): PASS. Steps 5-6 return real Markdown/source with
  concrete `sha256`/`byte_len`; step 7 confirms relative and absolute reads
  produce an identical hash.
- Issue 0.2 (class/status): PASS. Step 10 shows non-null class/status for
  header-bearing pages (and correct nulls for `index.md`/`log.md`); step 14
  filters select exactly the matching Literature-Note/Sourced pages.
- Issue 0.3 (auto/thresholds): PASS. Step 11 returns a populated array (no
  `thresholds_unconfigured`); steps 12-13 work post-index with
  `thresholds_source:"default"`.
- Issue 0.4 (rerank): PASS. Step 15 reports `rerank_applied:false` with the named
  reason `reranker_model_unconfigured` (no silent no-op).

Overall verdict: PASS for the pure-MCP correctness gate. No `BUG` rows; step 4
was intentionally skipped.

## Issues

No `BUG` rows were found.

## Follow-Ups And Limitations

1. **F1 — allow_lexical_fallback not-ready branch uncovered.** Step 17's
   not-ready fallback path (semantic → lexical results with a readiness reason
   recorded) could not be exercised because the index was ready/fresh and `force`
   re-indexing keeps it ready. Raw signal: `selected_mode:"semantic"`,
   `fallback_reason:null`, `runtime_backend_fallback:false`. Re-run on a
   deliberately cold/not-ready instance to confirm lexical hits with a populated
   `readiness_reason`.
2. **F2 — search-all include/exclude verified with a single project only.**
   Steps 18-19 ran against a registry containing only `keto-diet`, so
   `exclude:["keto-diet"]` emptied the selection entirely (`Error: no registered
   projects selected`) rather than showing partial narrowing. Register a second
   project and re-run to confirm multi-project readiness reporting and
   include/exclude narrowing with flat attributed top-level results.
3. Step 3 raw stderr included a host omission marker for earlier model-loader
   bytes; the correctness-relevant indexed-file stdout was present and stable, so
   this is a capture limitation, not a correctness bug.

## Posture

- [ ] New project (fresh `init`)
- [x] Existing project already registered and indexed
  - Already registered and indexed on the `_test` instance from a prior run;
    this pass exercised register + index idempotently as a regression run rather
    than from a cold state.
- [ ] Migration candidate (an external project moving onto the MCP)
