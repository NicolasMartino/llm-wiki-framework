# Eval: Codex MCP Field Test Pass

- Document Class: Eval
- Status: Accepted
- Date: 2026-07-07
- Updated: 2026-08-01
- Category: MCP surface, Codex host, field test
- Scope: Records clean Codex runs of the rewritten pure-MCP
  `wiki/checklists/mcp-field-test.checklist.md` against the `keto_diet`
  project: the original 2026-07-07 `_test` MCP run, the 2026-07-31
  production `llm-wiki 0.2.14` MCP run, and the 2026-08-01 production
  `llm-wiki 0.2.15` MCP review run. These runs use only the `llm_wiki_*`
  surface, with no Headroom, no proxy, and no CCR framing.
- Sources:
  - User-provided Codex results file pasted 2026-07-07
    (`mcp-field-test-results.md`, workspace `/Users/nicolasmartino/Documents/keto_diet`).
  - User-provided production Codex MCP field-test results pasted 2026-07-31
    (production `llm-wiki 0.2.14`, workspace
    `/Users/nicolasmartino/Documents/keto_diet`).
  - User-provided production Codex MCP field-test results pasted 2026-08-01
    (production `llm-wiki 0.2.15`, workspace
    `/Users/nicolasmartino/Documents/keto_diet`).
  - wiki/checklists/mcp-field-test.checklist.md
- Related:
  - wiki/checklists/mcp-field-test.checklist.md
  - wiki/evals/claude-mcp-field-test-pass.eval.md
  - wiki/evals/mcp-first-host-parity.eval.md

## Verdict

Accepted pure-MCP correctness pass for Codex in an existing already-migrated
project posture.

Updated 2026-07-31: the production MCP instance also passed the same core
correctness gate in the ready-index posture. That run used unsuffixed
`llm_wiki_*` tools, `llm-wiki 0.2.14`, managed home `/Users/nicolasmartino/.llm_wiki`,
manifest `/Users/nicolasmartino/.llm_wiki/manifest.json`, and workspace
`/Users/nicolasmartino/Documents/keto_diet`. It confirms that the older `_test`
0.2.8 proof now holds on the production MCP instance as well.

Updated 2026-08-01: the production MCP instance passed again on
`llm-wiki 0.2.15` with the same ready-index `keto-diet` posture. The submitted
report had one `BUG` row for `search-all` because the checklist said results
should be grouped per project, while the actual API returns per-project
readiness in `projects[]` and a top-level globally ranked flat `results[]` list
with `project_id`/`project_name` attribution on each hit. That is a contract
wording issue, not a product defect, unless a future schema intentionally adds a
grouped result view. The field-test checklist was clarified accordingly.

The run satisfies every load-bearing condition of the pure-MCP field test:

- `llm_wiki_read` / `llm_wiki_read_test` returned real wiki and raw Markdown
  with stable `sha256`/`byte_len` metadata (relative and absolute paths
  normalized and matched).
- Traversal escapes and a missing file were rejected with clear, distinct
  errors — not file contents and not a generic not-found for the escape case.
- Lexical, auto, semantic, hybrid, class/status-filtered search, rerank
  diagnostics, and `search-all` include/exclude all behaved as expected.
- Semantic and hybrid worked on shipped `thresholds_source:"default"` after
  indexing, with no `thresholds_unconfigured` hard failure.
- The rerank no-op was explicit (`reranker_model_unconfigured`), never silent.
- No product `BUG` rows. The 2026-08-01 Step 18 row is preserved as a
  checklist-contract correction, not a product defect.

This eval proves the pure-MCP correctness gate for the observed Codex session.
Two steps were recorded as `expected-fresh` rather than `OK` because the
project was already registered and indexed before Step 1 (see Follow-Ups); this
is a test-posture mismatch, not an MCP behavior defect.

## Production 0.2.15 Addendum (2026-08-01)

### Verdict

Accepted production MCP correctness pass for Codex in the existing ready-index
posture, with one checklist-contract clarification for `search-all`.

The run used:

- Instance: production MCP instance (`mcp__llm_wiki`)
- Tool names: unsuffixed `llm_wiki_status`, `llm_wiki_register`,
  `llm_wiki_index`, `llm_wiki_read`, `llm_wiki_search`, and
  `llm_wiki_search_all`
- Binary stem/version: `llm-wiki 0.2.15`
- Managed home: `/Users/nicolasmartino/.llm_wiki`
- Manifest: `/Users/nicolasmartino/.llm_wiki/manifest.json`
- Workspace: `/Users/nicolasmartino/Documents/keto_diet`
- Project: `keto-diet` / `keto diet`

Load-bearing results:

- `llm_wiki_status` reported the project already registered, fresh, ready, and
  searchable across lexical, semantic, and hybrid modes with 10 indexed files.
- `llm_wiki_register` was idempotent and rewired Claude/Codex MCP config without
  disturbing the ready project state.
- Normal and forced indexing both completed with `Indexed project: keto-diet
  (10 files)`.
- `llm_wiki_read` returned real Markdown for `wiki/index.md` and the raw
  research manifest, with the same hashes as the prior production run:
  `619ddffe7a9135b3d56c01d9c54aaa71fc2b7c005b2a3dcd1bd73a31cde833a4` for the
  wiki index and `26a7d46b443e5de17d353d6dd71883edfb9a0da2b6502b3a728791392a9c98c7`
  for the raw manifest.
- Relative and absolute reads normalized to `wiki/index.md` and matched by
  hash/byte length.
- Relative and absolute outside-tree reads were rejected as outside the project
  `wiki/` or `raw/` trees, and a missing wiki page returned a distinct
  not-found error.
- Lexical, auto, semantic, and hybrid searches returned populated result arrays.
  Auto selected hybrid with `thresholds_source:"default"`, and explicit
  semantic/hybrid searches also used default thresholds without a
  `thresholds_unconfigured` failure.
- Class/status filtering returned matching `Literature Note` / `Sourced` hits.
- `rerank:true` returned `rerank_requested:true`, `rerank_applied:false`,
  `rerank_reason:"reranker_model_unconfigured"`, and `reranker_model:null`.
  `rerank:false` returned the disabled state with no reason.
- `search-all` degraded per project, reported missing/stale projects in
  `projects[]`/`warnings[]`, preserved ready-project hits, and honored include
  and exclude filters.

The only submitted `BUG` row was Step 18's expectation that cross-project
results be grouped per project. Current and previously accepted behavior is a
flat top-level relevance-ranked `results[]` array with explicit project
attribution, plus separate per-project readiness in `projects[]`. The corrected
contract is now tracked in `wiki/checklists/mcp-field-test.checklist.md`.

### Production 0.2.15 Report Summary

| Step | Verdict | Notes |
| --- | --- | --- |
| 1 status | expected-fresh | Project was already registered, fresh, ready, and indexed before the run. |
| 2 register | OK | Idempotent register preserved `registered:true`. |
| 3 index / --force | OK | Both calls reported `Indexed project: keto-diet (10 files)`. |
| 4 calibrate | OK | Optional step skipped; semantic/hybrid used default thresholds. |
| 5 wiki read | OK | Real Markdown, `byte_len:2593`, matching prior hash. |
| 6 raw read | OK | Real raw Markdown, `byte_len:2689`, matching prior hash. |
| 7 abs/rel path | OK | Both reads normalized and matched. |
| 8 traversal | OK | Escapes rejected as outside wiki/raw. |
| 9 missing file | OK | Clear not-found error. |
| 10 lexical | OK | Populated lexical hits with metadata where applicable. |
| 11 auto | OK | Auto selected hybrid with default thresholds. |
| 12 semantic | OK | Populated semantic hits with default thresholds. |
| 13 hybrid | OK | Populated hybrid hits with default thresholds. |
| 14 class/status filters | OK | Filtered hits all matched `Literature Note` / `Sourced`. |
| 15 rerank true | OK | Explicit `reranker_model_unconfigured` reason. |
| 16 rerank false | OK | Rerank disabled cleanly. |
| 17 allow_lexical_fallback | expected-fresh | Not-ready fallback branch still unexercised. |
| 18 search-all | OK after checklist clarification | Flat attributed results plus grouped readiness is the current contract. |
| 19 include/exclude | OK | Include/exclude changed the searched project set and resulting hits. |

Follow-ups remain unchanged:

1. A truly fresh unregistered project posture still needs a separate run.
2. Deliberately not-ready semantic fallback still needs a separate run.

## Production 0.2.14 Addendum (2026-07-31)

### Verdict

Accepted production MCP correctness pass for Codex in the existing ready-index
posture.

The production run is significant because it used the real installed MCP
instance rather than the namespaced test instance:

- Instance: production MCP instance (`mcp__llm_wiki`)
- Tool names: unsuffixed `llm_wiki_status`, `llm_wiki_register`,
  `llm_wiki_index`, `llm_wiki_read`, `llm_wiki_search`, and
  `llm_wiki_search_all`
- Binary stem/version: `llm-wiki 0.2.14`
- Managed home: `/Users/nicolasmartino/.llm_wiki`
- Manifest: `/Users/nicolasmartino/.llm_wiki/manifest.json`
- Workspace: `/Users/nicolasmartino/Documents/keto_diet`
- Project: `keto-diet` / `keto diet`

The run has no `BUG` rows. It proves the pure-MCP correctness gate for the
production `0.2.14` instance in the observed ready-index posture. It does not
cover a truly fresh unregistered project or a deliberately not-ready semantic
fallback posture.

### Load-Bearing Results

- Read fidelity passed. `llm_wiki_read {"path":"wiki/index.md"}` returned real
  Markdown beginning `# Wiki Index` with `byte_len:2593` and
  `sha256:619ddffe7a9135b3d56c01d9c54aaa71fc2b7c005b2a3dcd1bd73a31cde833a4`.
  `llm_wiki_read` on the raw research manifest returned real Markdown with
  `byte_len:2689` and
  `sha256:26a7d46b443e5de17d353d6dd71883edfb9a0da2b6502b3a728791392a9c98c7`.
- Path handling passed. Relative and absolute reads of `wiki/index.md`
  normalized to `path:"wiki/index.md"` and matched by hash and byte length.
  `../../etc/hosts` and `/etc/hosts` were rejected as outside the project
  `wiki/` or `raw/` trees, while `wiki/does-not-exist.md` returned a distinct
  not-found error.
- Lexical search passed. `llm_wiki_search` with `mode:"lexical"` and query
  `ketogenic` returned five hits on a ready immutable qmd-rs index; metadata
  pages carried non-null `document_class:"Literature Note"` and
  `status:"Sourced"`, while `wiki/log.md` correctly had null metadata.
- Auto, semantic, and hybrid search passed. Auto selected `hybrid` with
  `mode_selection_reason:"enabled_profile"` and `thresholds_source:"default"`.
  Explicit semantic and hybrid searches returned populated result arrays with
  default thresholds and no `thresholds_unconfigured` failure.
- Class/status filtering passed. A direct read of
  `wiki/literature/keto-health-outcomes-umbrella-review.literature-note.md`
  showed `Document Class: Literature Note` and `Status: Sourced`; the filtered
  lexical search returned four matching hits, all with `document_class:"Literature Note"`
  and `status:"Sourced"`.
- Rerank diagnostics passed. `rerank:true` returned `rerank_requested:true`,
  `rerank_applied:false`, `reranker_model:null`, and
  `rerank_reason:"reranker_model_unconfigured"`. `rerank:false` returned
  `rerank_applied:false` with `rerank_reason:null`.
- Ready-posture lexical fallback behavior passed. With
  `allow_lexical_fallback:true`, explicit semantic search stayed semantic with
  `fallback_reason:null`, `runtime_backend_fallback:false`, and
  `thresholds_source:"default"` because the project was already ready.
- `search-all` passed the correctness gate. It returned a top-level aggregate
  with per-project readiness/backend entries, warnings for missing roots and
  stale indexes, and populated hits. Include filtering to `keto-diet` searched
  only that project; excluding `keto-diet` removed it from the project set and
  returned hits from other projects.

### Production Report

| Step | Tool | Verdict | Notes |
| --- | --- | --- | --- |
| 1 status | `llm_wiki_status` | expected-fresh | Already registered, fresh, ready, 10 indexed files, all modes ready. |
| 2 register | `llm_wiki_register` | OK | Idempotent `Project already registered: keto-diet`; follow-up status retained registration. |
| 3 index / --force | `llm_wiki_index` | OK | Normal and forced index both returned `Indexed project: keto-diet (10 files)`. |
| 4 calibrate | not run | OK | Optional tuning skipped; later semantic/hybrid calls used `thresholds_source:"default"`. |
| 5 wiki read | `llm_wiki_read` | OK | Real Markdown, `byte_len:2593`, `sha256:619ddffe...`. |
| 6 raw read | `llm_wiki_read` | OK | Real raw Markdown, `byte_len:2689`, `sha256:26a7d46...`. |
| 7 abs/rel path | `llm_wiki_read` | OK | Relative and absolute reads normalized and matched. |
| 8 traversal | `llm_wiki_read` | OK | Relative and absolute escapes rejected as outside wiki/raw. |
| 9 missing file | `llm_wiki_read` | OK | Clear not-found error. |
| 10 lexical | `llm_wiki_search` | OK | Populated lexical results; metadata-bearing hits have class/status. |
| 11 auto | `llm_wiki_search` | OK | Auto selected hybrid with default thresholds and populated results. |
| 12 semantic | `llm_wiki_search` | OK | Semantic search returned populated results with default thresholds. |
| 13 hybrid | `llm_wiki_search` | OK | Hybrid search returned populated results with default thresholds. |
| 14 class/status filters | `llm_wiki_read` + `llm_wiki_search` | OK | Inspected real Literature Note/Sourced page; filter returned matching hits. |
| 15 rerank true | `llm_wiki_search` | OK | Explicit `reranker_model_unconfigured` reason. |
| 16 rerank false | `llm_wiki_search` | OK | Rerank disabled cleanly with no reason. |
| 17 allow_lexical_fallback | `llm_wiki_search` | expected-fresh | Ready project stayed semantic; not-ready fallback branch not exercised. |
| 18 search-all | `llm_wiki_search_all` | OK | Per-project readiness and warnings reported; hits populated. |
| 19 include/exclude | `llm_wiki_search_all` | OK | Include selected only `keto-diet`; exclude removed it from the searched set. |

### Follow-Ups

1. Fresh registration remains uncovered by this production run because Step 1
   already reported `registered:true`, `index_freshness:"fresh"`, and
   `index_state:"ready"`.
2. Deliberately not-ready semantic fallback remains uncovered because Step 17
   ran against a ready semantic index. The observed ready behavior is correct:
   `allow_lexical_fallback:true` did not force lexical fallback.
3. Cross-project search correctness passed, but Step 18 took about 216 seconds
   because the registry contains stale or missing projects. The response
   correctly degraded per project and emitted warnings instead of failing
   globally; day-to-day queries should prune dead registry entries or use
   `include` when cross-project search is not needed.

## Run Header

- Date: `2026-07-07`
- Instance: test instance (`_test` tools), namespace `mcp__llm_wiki_test`
- Binary stem: `llm-wiki-test`
- Version: `0.2.8`
- Manifest: `/Users/nicolasmartino/.llm_wiki-test/manifest.json`
- Managed home: `/Users/nicolasmartino/.llm_wiki-test`
- Workspace root: `/Users/nicolasmartino/Documents/keto_diet`
- Project: `keto-diet` / `keto diet`
- Posture tested: existing project already registered and indexed before Step 1;
  register and index were re-run as idempotent setup.
- Content access rule: wiki/raw content was accessed through
  `llm_wiki_read_test`, `llm_wiki_search_test`, and `llm_wiki_search_all_test`,
  not shell reads.

## Per-Step Results

### Step 1 - Status

- Tool: `llm_wiki_status_test`
- Arguments: `{}`
- Verdict: `expected-fresh`

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

The checklist expects a fresh project to be unregistered and/or unindexed. The
project was already registered with a fresh ready index and ten indexed files,
so the server was healthy but the starting posture was not fresh. Marked
`expected-fresh`, not `BUG`.

### Step 2 - Register

- Tool: `llm_wiki_register_test`
- Arguments: `{"path":"/Users/nicolasmartino/Documents/keto_diet"}`
- Verdict: `OK`

Raw signal:

```json
{"stdout":"Project already registered: keto-diet\n","stderr":""}
```

Status re-check confirmed `registered:true` for `project_id:"keto-diet"`.
Registration was idempotent.

### Step 3 - Index / Force Index

- Tool: `llm_wiki_index_test`
- Arguments: `{}` and `{"force":true}`
- Verdict: `OK`

Both the normal and forced index calls returned:

```text
Indexed project: keto-diet (10 files)
```

The tool display included a host-side omission marker for earlier model-loader
stderr bytes (`[58297 bytes of earlier stderr omitted]`, followed by embedder
`ggml_metal`/`graph_reserve` noise). The indexed-file stdout was present and
stable, so this is a capture-side stderr artifact, not an llm-wiki correctness
bug.

### Step 4 - Calibrate Thresholds

- Tool: not run
- Verdict: `OK` (optional step intentionally skipped)

Calibration is optional quality tuning. It was skipped so later semantic and
hybrid searches would test the shipped default threshold path. Steps 11-13
confirm `thresholds_source:"default"`.

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
  "content_prefix": "# Wiki Index\n\nProject: keto diet\nStage: Evidence intake"
}
```

The output was real Markdown beginning with `# Wiki Index`.

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

The output was real raw-source Markdown.

### Step 7 - Absolute Vs Relative Path

- Tool: `llm_wiki_read_test`
- Relative arguments: `{"path":"wiki/index.md"}`
- Absolute arguments:
  `{"path":"/Users/nicolasmartino/Documents/keto_diet/wiki/index.md"}`
- Verdict: `OK`

Both reads succeeded, normalized to `path:"wiki/index.md"`, and returned
identical `sha256`
(`619ddffe7a9135b3d56c01d9c54aaa71fc2b7c005b2a3dcd1bd73a31cde833a4`) and
`byte_len` (`2593`).

### Step 8 - Traversal Rejection

- Tool: `llm_wiki_read_test`
- Arguments: `{"path":"../../etc/hosts"}` and `{"path":"/etc/hosts"}`
- Verdict: `OK`

Raw signals:

```json
{"error":{"message":"../../etc/hosts is outside the project wiki/ or raw/ trees"}}
{"error":{"message":"/etc/hosts is outside the project wiki/ or raw/ trees"}}
```

Both the relative escape and the absolute outside-project path returned explicit
outside-tree rejections and no file contents. The relative escape returned the
outside-tree message, not a generic not-found (Issue 8 satisfied).

### Step 9 - Missing File Error

- Tool: `llm_wiki_read_test`
- Arguments: `{"path":"wiki/does-not-exist.md"}`
- Verdict: `OK`

Raw signal:

```json
{"error":{"message":"wiki/does-not-exist.md not found"}}
```

Clear not-found error, not an empty success.

### Step 10 - Lexical Search

- Tool: `llm_wiki_search_test`
- Arguments: `{"mode":"lexical","query":"ketogenic","limit":5}`
- Verdict: `OK`

Raw signal:

```json
{
  "selected_mode": "lexical",
  "mode_selection_reason": "explicit_lexical",
  "backend_status": {"state": "ready", "freshness": "fresh", "indexed_files": 10},
  "results_count": 5,
  "results[0]": {"document_class": "Literature Note", "status": "Sourced"}
}
```

The ready lexical index returned a populated `results` array. Literature hits
carried non-null `document_class:"Literature Note"` and `status:"Sourced"`;
`wiki/log.md` returned null metadata because it is a log page without a
metadata block (expected, not a bug).

### Step 11 - Auto Search

- Tool: `llm_wiki_search_test`
- Arguments:
  `{"mode":"auto","query":"What evidence exists for ketogenic diet and type 2 diabetes remission?","limit":5}`
- Verdict: `OK`

Raw signal:

```json
{
  "requested_mode": "auto",
  "selected_mode": "hybrid",
  "mode_selection_reason": "enabled_profile",
  "thresholds_source": "default",
  "runtime_backend_fallback": false,
  "zero_result_reason": null,
  "results_count": 5
}
```

Auto selected hybrid-capable search with default thresholds and no
`thresholds_unconfigured` error.

### Step 12 - Semantic Search

- Tool: `llm_wiki_search_test`
- Arguments:
  `{"mode":"semantic","query":"What evidence exists for ketogenic diet and type 2 diabetes remission?","limit":5}`
- Verdict: `OK`

Raw signal:

```json
{
  "requested_mode": "semantic",
  "selected_mode": "semantic",
  "thresholds_source": "default",
  "runtime_backend_fallback": false,
  "zero_result_reason": null,
  "results_count": 5
}
```

Semantic search returned hits after indexing on default thresholds and did not
hard-fail with `thresholds_unconfigured`.

### Step 13 - Hybrid Search

- Tool: `llm_wiki_search_test`
- Arguments:
  `{"mode":"hybrid","query":"What evidence exists for ketogenic diet and type 2 diabetes remission?","limit":5}`
- Verdict: `OK`

Raw signal:

```json
{
  "requested_mode": "hybrid",
  "selected_mode": "hybrid",
  "thresholds_source": "default",
  "runtime_backend_fallback": false,
  "zero_result_reason": null,
  "results_count": 5
}
```

Hybrid search returned hits after indexing on default thresholds.

### Step 14 - Class And Status Filters

- Tool: `llm_wiki_search_test`
- Arguments:
  `{"mode":"hybrid","query":"What evidence exists for ketogenic diet and type 2 diabetes remission?","class":"Literature Note","status":"Sourced","limit":5}`
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

The filter values were chosen from observed hits. The filtered search returned
matching literature-note pages rather than excluding everything.

### Step 15 - Rerank True

- Tool: `llm_wiki_search_test`
- Arguments:
  `{"mode":"hybrid","query":"What evidence exists for ketogenic diet and type 2 diabetes remission?","rerank":true,"limit":5}`
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

The disabled reranker state was explicit and explained — not a silent no-op.

### Step 16 - Rerank False

- Tool: `llm_wiki_search_test`
- Arguments:
  `{"mode":"hybrid","query":"What evidence exists for ketogenic diet and type 2 diabetes remission?","rerank":false,"limit":5}`
- Verdict: `OK`

Raw signal:

```json
{
  "rerank_requested": false,
  "rerank_applied": false,
  "rerank_reason": null
}
```

### Step 17 - Allow Lexical Fallback

- Tool: `llm_wiki_search_test`
- Arguments:
  `{"mode":"semantic","query":"What evidence exists for ketogenic diet and type 2 diabetes remission?","allow_lexical_fallback":true,"limit":5}`
- Verdict: `expected-fresh`

Raw signal:

```json
{
  "requested_mode": "semantic",
  "selected_mode": "semantic",
  "thresholds_source": "default",
  "fallback_reason": null,
  "runtime_backend_fallback": false,
  "results_count": 5
}
```

The deliberately not-ready fallback branch could not be exercised because the
project was ready before Step 1 and remained ready after indexing. The
ready-posture requirement was confirmed: `allow_lexical_fallback:true` did not
force a fallback; the response stayed semantic with default thresholds.

### Step 18 - Search-All

- Tool: `llm_wiki_search_all_test`
- Arguments: `{"mode":"auto","query":"ketogenic diet evidence","limit":5}`
- Verdict: `OK`

Raw signal:

```json
{
  "requested_mode": "auto",
  "selected_mode": "hybrid",
  "thresholds_source": "default",
  "projects": [
    {
      "project_id": "keto-diet",
      "selected_mode": "hybrid",
      "result_count": 10,
      "readiness_reason": null,
      "zero_result_reason": null
    }
  ],
  "results_count": 5
}
```

The aggregate response reported per-project readiness for `keto-diet`, and every
top-level hit carried `project_id:"keto-diet"` / `project_name:"keto diet"`.

### Step 19 - Include / Exclude

- Tool: `llm_wiki_search_all_test`
- Include arguments:
  `{"mode":"auto","query":"ketogenic diet evidence","include":["keto-diet"],"limit":5}`
- Exclude arguments:
  `{"mode":"auto","query":"ketogenic diet evidence","exclude":["keto-diet"],"limit":5}`
- Verdict: `OK`

Include returned the same single project with populated results. Exclude removed
the only registered project:

```json
{
  "error": {
    "message": "llm-wiki search-all ketogenic diet evidence --format json --mode auto --exclude keto-diet --limit 5 failed with status exit status: 1: Error: no registered projects selected\n"
  }
}
```

The searched set changed with the filter, confirming include/exclude are honored.

## Report

| Step | Tool | Verdict | Notes |
| --- | --- | --- | --- |
| 1 status | `llm_wiki_status_test` | expected-fresh | Already registered/fresh/ready, 10 indexed files, all modes ready. |
| 2 register | `llm_wiki_register_test` | OK | Idempotent `Project already registered: keto-diet`; status re-check registered. |
| 3 index / --force | `llm_wiki_index_test` | OK | Normal and forced index report `Indexed project: keto-diet (10 files)`. |
| 4 calibrate | not run | OK | Optional tuning skipped; semantic/hybrid later used default thresholds. |
| 5 wiki read | `llm_wiki_read_test` | OK | Real Markdown, `byte_len:2593`, `sha256:619ddffe...`. |
| 6 raw read | `llm_wiki_read_test` | OK | Real raw content, `byte_len:2689`, `sha256:26a7d46...`. |
| 7 abs/rel path | `llm_wiki_read_test` | OK | Relative and absolute reads match and normalize. |
| 8 traversal | `llm_wiki_read_test` | OK | Relative and absolute escapes rejected as outside wiki/raw. |
| 9 missing file | `llm_wiki_read_test` | OK | Clear not-found error. |
| 10 lexical | `llm_wiki_search_test` | OK | Populated lexical results; metadata-bearing pages have class/status. |
| 11 auto | `llm_wiki_search_test` | OK | Auto selected hybrid with default thresholds and populated results. |
| 12 semantic | `llm_wiki_search_test` | OK | Semantic results with default thresholds. |
| 13 hybrid | `llm_wiki_search_test` | OK | Hybrid results with default thresholds. |
| 14 class/status filters | `llm_wiki_search_test` | OK | Literature Note/Sourced filters returned matching hits. |
| 15 rerank true | `llm_wiki_search_test` | OK | Explicit `reranker_model_unconfigured` reason. |
| 16 rerank false | `llm_wiki_search_test` | OK | Rerank disabled cleanly. |
| 17 allow_lexical_fallback | `llm_wiki_search_test` | expected-fresh | Ready project stayed semantic; not-ready fallback branch not exercised. |
| 18 search-all | `llm_wiki_search_all_test` | OK | Per-project readiness and attributed hits. |
| 19 include/exclude | `llm_wiki_search_all_test` | OK | Include selects `keto-diet`; exclude returns `no registered projects selected`. |

## Pass Criteria

- Issue 0.1 (read fidelity): PASS. Steps 5-6 returned real Markdown/source
  content with matching `sha256` and `byte_len`.
- Issue 0.2 (class/status): PASS. Steps 10 and 14 showed non-null class/status
  for pages with metadata headers, and filters selected matching pages.
- Issue 0.3 (auto/thresholds): PASS. Step 11 returned populated results with
  `thresholds_source:"default"`; steps 12-13 also worked after indexing with
  default thresholds.
- Issue 0.4 (rerank): PASS. Step 15 reported `rerank_applied:false` plus
  `rerank_reason:"reranker_model_unconfigured"`.

Overall verdict: PASS for the pure-MCP correctness gate in the existing
already-migrated posture.

## Issues

No `BUG` rows were found.

## Follow-Ups And Limitations

1. Step 1 fresh-project posture was not available: the first status call already
   showed a registered ready project with ten indexed files. This is a
   test-posture mismatch, not an MCP behavior bug. To exercise the fresh path,
   run this checklist in a project not yet registered with the `_test` instance.
2. Step 17's deliberately not-ready fallback branch was not exercised because
   the project was ready before setup and remained ready after indexing. A
   separate run should select a deliberately not-ready project before indexing
   to validate lexical fallback under unavailable semantic artifacts.
3. Step 3 raw output included a host/tool-display omission marker for earlier
   model-loader stderr bytes. The correctness-relevant indexed-file stdout was
   present and stable, so this is a capture limitation, not a correctness bug.
4. `search-all` was exercised with a single registered project (`keto-diet`), so
   include/exclude behavior is necessarily single-project.

## Posture

- [ ] New project (fresh `init`)
- [x] Existing project already registered and indexed
  - Observed outside the checklist's fresh-project expectation: the project was
    already registered/indexed at Step 1, then register/index were re-run
    idempotently.
- [ ] Migration candidate (an external project moving onto the MCP)
