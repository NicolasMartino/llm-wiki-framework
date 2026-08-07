# LLM Wiki MCP Field Test Results

Date: 2026-06-27
Project: `/Users/nicolasmartino/Documents/keto_diet`
Test instance: `llm-wiki-test`

Constraint followed: wiki/raw content was accessed only through the LLM Wiki MCP tools.

## Phase A - Setup

### Step 1 - status

Tool and arguments:

```json
llm_wiki_status_test({})
```

Raw output:

```text
Wall time: 0.0094 seconds
Output:
[{"type":"text","text":"{\"installed\":true,\"manifest_path\":\"/Users/nicolasmartino/.llm_wiki-test/manifest.json\",\"managed_home\":\"/Users/nicolasmartino/.llm_wiki-test\",\"binary_stem\":\"llm-wiki-test\",\"profile_name\":\"llm-wiki-test.profile.env\",\"version\":\"0.2.4\"}"}]
```

Verdict: `BUG: status only reports install metadata; project registration, index freshness, and search readiness are not exposed`

### Step 2 - register

Tool and arguments:

```json
llm_wiki_register_test({
  "path": "/Users/nicolasmartino/Documents/keto_diet"
})
```

Raw output:

```text
Wall time: 0.0186 seconds
Output:
[{"type":"text","text":"{\"stdout\":\"Project already registered: keto-diet\\n\",\"stderr\":\"\"}"}]
```

Tool and arguments:

```json
llm_wiki_status_test({})
```

Raw output:

```text
Wall time: 0.0022 seconds
Output:
[{"type":"text","text":"{\"installed\":true,\"manifest_path\":\"/Users/nicolasmartino/.llm_wiki-test/manifest.json\",\"managed_home\":\"/Users/nicolasmartino/.llm_wiki-test\",\"binary_stem\":\"llm-wiki-test\",\"profile_name\":\"llm-wiki-test.profile.env\",\"version\":\"0.2.4\"}"}]
```

Verdict: `OK for registration command; BUG: status still does not confirm registered project state`

### Step 3 - index / force

Tool and arguments:

```json
llm_wiki_index_test({
  "project": "keto-diet"
})
```

Raw output:

```text
Wall time: 11.0914 seconds
Output:
[{"type":"text","text":"{\"stdout\":\"Indexed project: keto-diet (10 files)\\n\",\"stderr\":\"\"}"}]
```

Tool and arguments:

```json
llm_wiki_index_test({
  "project": "keto-diet",
  "force": true
})
```

Raw output:

```text
Wall time: 11.0679 seconds
Output:
[{"type":"text","text":"{\"stdout\":\"Indexed project: keto-diet (10 files)\\n\",\"stderr\":\"\"}"}]
```

Verdict: `OK`

### Step 4 - calibrate thresholds

Tool and arguments:

```text
Skipped. No MCP tool call was made and no CLI calibration was run.
```

Raw output:

```text
N/A
```

Verdict: `expected-fresh`

## Phase B - Read

### Step 5 - wiki read

Tool and arguments:

```json
llm_wiki_read_test({
  "project": "keto-diet",
  "path": "wiki/index.md"
})
```

Raw output:

```text
Wall time: 0.0034 seconds
Output:

[{"type":"text","text":"{\"project_id\":\"keto-diet\",\"project_name\":\"Keto Diet\",\"project_root\":\"/Users/nicolasmartino/Documents/keto_diet\",\"wiki_root\":\"/Users/nicolasmartino/Documents/keto_diet/wiki\",\"raw_root\":\"/Users/nicolasmartino/Documents/keto_diet/raw\",\"path\":\"wiki/index.md\",\"tree\":\"wiki\",\"scope\":\"wiki\",\"byte_len\":2593,\"bytes\":2593,\"sha256\":\"619ddffe7a9135b3d56c01d9c54aaa71fc2b7c005b2a3dcd1bd73a31cde833a4\",\"encoding\":\"utf-8\",\"content\":\"<<ccr:619ddffe7a91,string,2.5KB>>\"}"}]
```

Verdict: `BUG: Issue 0.1 - wiki read returns <<ccr:...>> placeholder instead of real Markdown`

### Step 6 - raw read

Tool and arguments:

```json
llm_wiki_read_test({
  "project": "keto-diet",
  "path": "raw/manifest.md"
})
```

Raw output:

```text
Wall time: 0.0048 seconds
Output:
[{"type":"text","text":"{\"error\":{\"message\":\"canonicalize /Users/nicolasmartino/Documents/keto_diet/raw/manifest.md\"}}"}]
```

Tool and arguments:

```json
llm_wiki_read_test({
  "project": "keto-diet",
  "path": "raw/README.md"
})
```

Raw output:

```text
Wall time: 0.0020 seconds
Output:
[{"type":"text","text":"{\"error\":{\"message\":\"canonicalize /Users/nicolasmartino/Documents/keto_diet/raw/README.md\"}}"}]
```

Tool and arguments:

```json
llm_wiki_read_test({
  "project": "keto-diet",
  "path": "raw"
})
```

Raw output:

```text
Wall time: 0.0020 seconds
Output:
[{"type":"text","text":"{\"error\":{\"message\":\"raw is not a readable file\"}}"}]
```

Verdict: `BUG: raw read could not be completed through MCP because no discoverable raw file path was exposed and directory reads do not list raw children`

### Step 7 - absolute vs relative path

Relative path call was step 5.

Tool and arguments:

```json
llm_wiki_read_test({
  "project": "keto-diet",
  "path": "/Users/nicolasmartino/Documents/keto_diet/wiki/index.md"
})
```

Raw output:

```text
Wall time: 0.0050 seconds
Output:

[{"type":"text","text":"{\"project_id\":\"keto-diet\",\"project_name\":\"Keto Diet\",\"project_root\":\"/Users/nicolasmartino/Documents/keto_diet\",\"wiki_root\":\"/Users/nicolasmartino/Documents/keto_diet/wiki\",\"raw_root\":\"/Users/nicolasmartino/Documents/keto_diet/raw\",\"path\":\"wiki/index.md\",\"tree\":\"wiki\",\"scope\":\"wiki\",\"byte_len\":2593,\"bytes\":2593,\"sha256\":\"619ddffe7a9135b3d56c01d9c54aaa71fc2b7c005b2a3dcd1bd73a31cde833a4\",\"encoding\":\"utf-8\",\"content\":\"<<ccr:619ddffe7a91,string,2.5KB>>\"}"}]
```

Verdict: `OK for absolute/relative path resolution and matching hash; BUG remains for placeholder content`

### Step 8 - traversal rejection

Tool and arguments:

```json
llm_wiki_read_test({
  "project": "keto-diet",
  "path": "../../etc/hosts"
})
```

Raw output:

```text
Wall time: 0.0028 seconds
Output:
[{"type":"text","text":"{\"error\":{\"message\":\"canonicalize /Users/nicolasmartino/Documents/keto_diet/../../etc/hosts\"}}"}]
```

Verdict: `OK`

### Step 9 - missing-file error

Tool and arguments:

```json
llm_wiki_read_test({
  "project": "keto-diet",
  "path": "wiki/does-not-exist.md"
})
```

Raw output:

```text
Wall time: 0.0029 seconds
Output:
[{"type":"text","text":"{\"error\":{\"message\":\"canonicalize /Users/nicolasmartino/Documents/keto_diet/wiki/does-not-exist.md\"}}"}]
```

Verdict: `OK`

## Phase C - Search

### Step 10 - lexical

Tool and arguments:

```json
llm_wiki_search_test({
  "project": "keto-diet",
  "query": "ketogenic",
  "mode": "lexical",
  "limit": 10
})
```

Raw output:

```text
Wall time: 0.0312 seconds
Output:

[{"type":"text","text":"{\"backend_status\":{\"freshness\":\"fresh\",\"indexed_files\":10,\"message\":null,\"open_mode\":\"read_only_immutable\",\"state\":\"ready\",\"store_path\":\"/Users/nicolasmartino/.llm_wiki-test/indexes/keto-diet/qmd-rs.sqlite\"},\"embedding_model\":\"embeddinggemma-300m-q8_0\",\"fallback_reason\":null,\"mode_selection_reason\":\"explicit_lexical\",\"profile\":\"balanced\",\"project_id\":\"keto-diet\",\"project_name\":\"Keto Diet\",\"projects\":[],\"query\":\"ketogenic\",\"readiness_reason\":null,\"requested_mode\":\"lexical\",\"rerank_applied\":false,\"rerank_reason\":null,\"rerank_requested\":false,\"reranker_model\":null,\"runtime_backend_fallback\":null,\"runtime_backend_requested\":null,\"runtime_backend_used\":null,\"runtime_error_kind\":null,\"runtime_failure_stage\":null,\"selected_mode\":\"lexical\",\"warning\":null,\"warnings\":[],\"zero_result_reason\":null}"}]
```

Retry with broader query.

Tool and arguments:

```json
llm_wiki_search_test({
  "project": "keto-diet",
  "query": "diet",
  "mode": "lexical",
  "limit": 10
})
```

Raw output:

```text
Wall time: 0.0249 seconds
Output:

[{"type":"text","text":"{\"backend_status\":{\"freshness\":\"fresh\",\"indexed_files\":10,\"message\":null,\"open_mode\":\"read_only_immutable\",\"state\":\"ready\",\"store_path\":\"/Users/nicolasmartino/.llm_wiki-test/indexes/keto-diet/qmd-rs.sqlite\"},\"embedding_model\":\"embeddinggemma-300m-q8_0\",\"fallback_reason\":null,\"mode_selection_reason\":\"explicit_lexical\",\"profile\":\"balanced\",\"project_id\":\"keto-diet\",\"project_name\":\"Keto Diet\",\"projects\":[],\"query\":\"diet\",\"readiness_reason\":null,\"requested_mode\":\"lexical\",\"rerank_applied\":false,\"rerank_reason\":null,\"rerank_requested\":false,\"reranker_model\":null,\"runtime_backend_fallback\":null,\"runtime_backend_requested\":null,\"runtime_backend_used\":null,\"runtime_error_kind\":null,\"runtime_failure_stage\":null,\"selected_mode\":\"lexical\",\"warning\":null,\"warnings\":[],\"zero_result_reason\":null}"}]
```

Retry with metadata phrase.

Tool and arguments:

```json
llm_wiki_search_test({
  "project": "keto-diet",
  "query": "Document Class",
  "mode": "lexical",
  "limit": 10
})
```

Raw output:

```text
Wall time: 0.0213 seconds
Output:

[{"type":"text","text":"{\"backend_status\":{\"freshness\":\"fresh\",\"indexed_files\":10,\"message\":null,\"open_mode\":\"read_only_immutable\",\"state\":\"ready\",\"store_path\":\"/Users/nicolasmartino/.llm_wiki-test/indexes/keto-diet/qmd-rs.sqlite\"},\"embedding_model\":\"embeddinggemma-300m-q8_0\",\"fallback_reason\":null,\"mode_selection_reason\":\"explicit_lexical\",\"profile\":\"balanced\",\"project_id\":\"keto-diet\",\"project_name\":\"Keto Diet\",\"projects\":[],\"query\":\"Document Class\",\"readiness_reason\":null,\"requested_mode\":\"lexical\",\"rerank_applied\":false,\"rerank_reason\":null,\"rerank_requested\":false,\"reranker_model\":null,\"runtime_backend_fallback\":null,\"runtime_backend_requested\":null,\"runtime_backend_used\":null,\"runtime_error_kind\":null,\"runtime_failure_stage\":null,\"selected_mode\":\"lexical\",\"warning\":null,\"warnings\":[],\"zero_result_reason\":null}"}]
```

Retry with default project.

Tool and arguments:

```json
llm_wiki_search_test({
  "query": "diet",
  "mode": "lexical",
  "limit": 10
})
```

Raw output:

```text
Wall time: 0.0229 seconds
Output:

[{"type":"text","text":"{\"backend_status\":{\"freshness\":\"fresh\",\"indexed_files\":10,\"message\":null,\"open_mode\":\"read_only_immutable\",\"state\":\"ready\",\"store_path\":\"/Users/nicolasmartino/.llm_wiki-test/indexes/keto-diet/qmd-rs.sqlite\"},\"embedding_model\":\"embeddinggemma-300m-q8_0\",\"fallback_reason\":null,\"mode_selection_reason\":\"explicit_lexical\",\"profile\":\"balanced\",\"project_id\":\"keto-diet\",\"project_name\":\"Keto Diet\",\"projects\":[],\"query\":\"diet\",\"readiness_reason\":null,\"requested_mode\":\"lexical\",\"rerank_applied\":false,\"rerank_reason\":null,\"rerank_requested\":false,\"reranker_model\":null,\"runtime_backend_fallback\":null,\"runtime_backend_requested\":null,\"runtime_backend_used\":null,\"runtime_error_kind\":null,\"runtime_failure_stage\":null,\"selected_mode\":\"lexical\",\"warning\":null,\"warnings\":[],\"zero_result_reason\":null}"}]
```

Verdict: `BUG: lexical search returns no hits and sometimes no results array/zero_result_reason despite fresh index with 10 indexed files`

### Step 11 - auto

Tool and arguments:

```json
llm_wiki_search_test({
  "project": "keto-diet",
  "query": "what does this project say about ketogenic diet research",
  "mode": "auto",
  "limit": 10
})
```

Raw output:

```text
Wall time: 0.0247 seconds
Output:
[{"type":"text","text":"{\"backend_status\":{\"freshness\":\"fresh\",\"indexed_files\":10,\"message\":null,\"open_mode\":\"read_only_immutable\",\"state\":\"ready\",\"store_path\":\"/Users/nicolasmartino/.llm_wiki-test/indexes/keto-diet/qmd-rs.sqlite\"},\"embedding_model\":\"embeddinggemma-300m-q8_0\",\"fallback_reason\":\"thresholds_unconfigured\",\"mode_selection_reason\":\"auto_lexical_fallback\",\"profile\":\"balanced\",\"project_id\":\"keto-diet\",\"project_name\":\"Keto Diet\",\"projects\":[],\"query\":\"what does this project say about ketogenic diet research\",\"query_expansion_model\":\"qmd-query-expansion-1.7b-q4_k_m\",\"readiness_reason\":null,\"requested_mode\":\"auto\",\"rerank_applied\":false,\"rerank_reason\":null,\"rerank_requested\":false,\"reranker_model\":null,\"results\":[],\"runtime_backend_fallback\":null,\"runtime_backend_requested\":null,\"runtime_backend_used\":null,\"runtime_error_kind\":null,\"runtime_failure_stage\":null,\"selected_mode\":\"lexical\",\"warning\":null,\"warnings\":[],\"zero_result_reason\":\"backend returned zero hits before filters\"}"}]
```

Verdict: `OK for Issue 0.3 fallback shape; BUG/follow-up: fallback lexical returns zero hits`

### Step 12 - semantic

Tool and arguments:

```json
llm_wiki_search_test({
  "project": "keto-diet",
  "query": "ketogenic diet research",
  "mode": "semantic",
  "limit": 10
})
```

Raw output:

```text
Wall time: 0.0158 seconds
Output:
[{"type":"text","text":"{\"error\":{\"message\":\"llm-wiki search ketogenic diet research --format json --mode semantic --project keto-diet --limit 10 failed with status exit status: 1: Error: search readiness failure: thresholds_unconfigured (run `llm-wiki eval calibrate --record` to calibrate and record semantic/hybrid thresholds, then retry (or use mode `auto`/`lexical`, which do not require calibration))\\n{\\n  \\\"query\\\": \\\"ketogenic diet research\\\",\\n  \\\"project_id\\\": \\\"keto-diet\\\",\\n  \\\"project_name\\\": \\\"Keto Diet\\\",\\n  \\\"requested_mode\\\": \\\"semantic\\\",\\n  \\\"selected_mode\\\": null,\\n  \\\"mode_selection_reason\\\": null,\\n  \\\"fallback_reason\\\": null,\\n  \\\"readiness_reason\\\": \\\"thresholds_unconfigured\\\",\\n  \\\"zero_result_reason\\\": null,\\n  \\\"profile\\\": \\\"balanced\\\",\\n  \\\"embedding_model\\\": \\\"embeddinggemma-300m-q8_0\\\",\\n  \\\"query_expansion_model\\\": \\\"qmd-query-expansion-1.7b-q4_k_m\\\",\\n  \\\"reranker_model\\\": null,\\n  \\\"rerank_requested\\\": false,\\n  \\\"rerank_applied\\\": false,\\n  \\\"rerank_reason\\\": null,\\n  \\\"runtime_backend_requested\\\": null,\\n  \\\"runtime_backend_used\\\": null,\\n  \\\"runtime_backend_fallback\\\": null,\\n  \\\"runtime_failure_stage\\\": null,\\n  \\\"runtime_error_kind\\\": null,\\n  \\\"warning\\\": null,\\n  \\\"warnings\\\": [],\\n  \\\"backend_status\\\": null,\\n  \\\"projects\\\": [],\\n  \\\"results\\\": []\\n}\\n\"}}]
```

Verdict: `expected-fresh`

### Step 13 - hybrid

Tool and arguments:

```json
llm_wiki_search_test({
  "project": "keto-diet",
  "query": "ketogenic diet research",
  "mode": "hybrid",
  "limit": 10
})
```

Raw output:

```text
Wall time: 0.0124 seconds
Output:
[{"type":"text","text":"{\"error\":{\"message\":\"llm-wiki search ketogenic diet research --format json --mode hybrid --project keto-diet --limit 10 failed with status exit status: 1: Error: search readiness failure: thresholds_unconfigured (run `llm-wiki eval calibrate --record` to calibrate and record semantic/hybrid thresholds, then retry (or use mode `auto`/`lexical`, which do not require calibration))\\n{\\n  \\\"query\\\": \\\"ketogenic diet research\\\",\\n  \\\"project_id\\\": \\\"keto-diet\\\",\\n  \\\"project_name\\\": \\\"Keto Diet\\\",\\n  \\\"requested_mode\\\": \\\"hybrid\\\",\\n  \\\"selected_mode\\\": null,\\n  \\\"mode_selection_reason\\\": null,\\n  \\\"fallback_reason\\\": null,\\n  \\\"readiness_reason\\\": \\\"thresholds_unconfigured\\\",\\n  \\\"zero_result_reason\\\": null,\\n  \\\"profile\\\": \\\"balanced\\\",\\n  \\\"embedding_model\\\": \\\"embeddinggemma-300m-q8_0\\\",\\n  \\\"query_expansion_model\\\": \\\"qmd-query-expansion-1.7b-q4_k_m\\\",\\n  \\\"reranker_model\\\": null,\\n  \\\"rerank_requested\\\": false,\\n  \\\"rerank_applied\\\": false,\\n  \\\"rerank_reason\\\": null,\\n  \\\"runtime_backend_requested\\\": null,\\n  \\\"runtime_backend_used\\\": null,\\n  \\\"runtime_backend_fallback\\\": null,\\n  \\\"runtime_failure_stage\\\": null,\\n  \\\"runtime_error_kind\\\": null,\\n  \\\"warning\\\": null,\\n  \\\"warnings\\\": [],\\n  \\\"backend_status\\\": null,\\n  \\\"projects\\\": [],\\n  \\\"results\\\": []\\n}\\n\"}}]
```

Verdict: `expected-fresh`

### Step 14 - class + status filters

Tool and arguments:

```json
llm_wiki_search_test({
  "project": "keto-diet",
  "query": "diet",
  "mode": "lexical",
  "class": "Decision",
  "status": "Accepted",
  "limit": 10
})
```

Raw output:

```text
Wall time: 0.0453 seconds
Output:
[{"type":"text","text":"{\"backend_status\":{\"freshness\":\"fresh\",\"indexed_files\":10,\"message\":null,\"open_mode\":\"read_only_immutable\",\"state\":\"ready\",\"store_path\":\"/Users/nicolasmartino/.llm_wiki-test/indexes/keto-diet/qmd-rs.sqlite\"},\"embedding_model\":\"embeddinggemma-300m-q8_0\",\"fallback_reason\":null,\"mode_selection_reason\":\"explicit_lexical\",\"profile\":\"balanced\",\"project_id\":\"keto-diet\",\"project_name\":\"Keto Diet\",\"projects\":[],\"query\":\"diet\",\"query_expansion_model\":\"qmd-query-expansion-1.7b-q4_k_m\",\"readiness_reason\":null,\"requested_mode\":\"lexical\",\"rerank_applied\":false,\"rerank_reason\":null,\"rerank_requested\":false,\"reranker_model\":null,\"results\":[],\"runtime_backend_fallback\":null,\"runtime_backend_requested\":null,\"runtime_backend_used\":null,\"runtime_error_kind\":null,\"runtime_failure_stage\":null,\"selected_mode\":\"lexical\",\"warning\":null,\"warnings\":[],\"zero_result_reason\":\"filters excluded all matched hits\"}"}]
```

Verdict: `BUG/follow-up: cannot verify filters because base lexical hit path is broken; output claims filters excluded matched hits but returns no inspectable hits`

### Step 15 - rerank true

Tool and arguments:

```json
llm_wiki_search_test({
  "project": "keto-diet",
  "query": "diet",
  "mode": "lexical",
  "rerank": true,
  "limit": 10
})
```

Raw output:

```text
Wall time: 0.0242 seconds
Output:

[{"type":"text","text":"{\"backend_status\":{\"freshness\":\"fresh\",\"indexed_files\":10,\"message\":null,\"open_mode\":\"read_only_immutable\",\"state\":\"ready\",\"store_path\":\"/Users/nicolasmartino/.llm_wiki-test/indexes/keto-diet/qmd-rs.sqlite\"},\"embedding_model\":\"embeddinggemma-300m-q8_0\",\"fallback_reason\":null,\"mode_selection_reason\":\"explicit_lexical\",\"profile\":\"balanced\",\"project_id\":\"keto-diet\",\"project_name\":\"Keto Diet\",\"projects\":[],\"query\":\"diet\",\"readiness_reason\":null,\"requested_mode\":\"lexical\",\"rerank_applied\":false,\"rerank_requested\":true,\"reranker_model\":null,\"runtime_backend_fallback\":null,\"runtime_backend_requested\":null,\"runtime_backend_used\":null,\"runtime_error_kind\":null,\"runtime_failure_stage\":null,\"selected_mode\":\"lexical\",\"warning\":null,\"warnings\":[],\"zero_result_reason\":null}"}]
```

Verdict: `BUG: rerank_requested true with rerank_applied false and reranker_model null, but no rerank_reason`

### Step 16 - rerank false

Tool and arguments:

```json
llm_wiki_search_test({
  "project": "keto-diet",
  "query": "diet",
  "mode": "lexical",
  "rerank": false,
  "limit": 10
})
```

Raw output:

```text
Wall time: 0.0237 seconds
Output:

[{"type":"text","text":"{\"backend_status\":{\"freshness\":\"fresh\",\"indexed_files\":10,\"message\":null,\"open_mode\":\"read_only_immutable\",\"state\":\"ready\",\"store_path\":\"/Users/nicolasmartino/.llm_wiki-test/indexes/keto-diet/qmd-rs.sqlite\"},\"embedding_model\":\"embeddinggemma-300m-q8_0\",\"fallback_reason\":null,\"mode_selection_reason\":\"explicit_lexical\",\"profile\":\"balanced\",\"project_id\":\"keto-diet\",\"project_name\":\"Keto Diet\",\"projects\":[],\"query\":\"diet\",\"readiness_reason\":null,\"requested_mode\":\"lexical\",\"rerank_applied\":false,\"rerank_reason\":null,\"rerank_requested\":false,\"reranker_model\":null,\"runtime_backend_fallback\":null,\"runtime_backend_requested\":null,\"runtime_backend_used\":null,\"runtime_error_kind\":null,\"runtime_failure_stage\":null,\"selected_mode\":\"lexical\",\"warning\":null,\"warnings\":[],\"zero_result_reason\":null}"}]
```

Verdict: `OK`

### Step 17 - allow_lexical_fallback

Tool and arguments:

```json
llm_wiki_search_test({
  "project": "keto-diet",
  "query": "ketogenic diet research",
  "mode": "semantic",
  "allow_lexical_fallback": true,
  "limit": 10
})
```

Raw output:

```text
Wall time: 0.0344 seconds
Output:

[{"type":"text","text":"{\"backend_status\":{\"freshness\":\"fresh\",\"indexed_files\":10,\"message\":null,\"open_mode\":\"read_only_immutable\",\"state\":\"ready\",\"store_path\":\"/Users/nicolasmartino/.llm_wiki-test/indexes/keto-diet/qmd-rs.sqlite\"},\"embedding_model\":\"embeddinggemma-300m-q8_0\",\"fallback_reason\":\"thresholds_unconfigured\",\"mode_selection_reason\":\"lexical_fallback\",\"profile\":\"balanced\",\"project_id\":\"keto-diet\",\"project_name\":\"Keto Diet\",\"projects\":[],\"query\":\"ketogenic diet research\",\"readiness_reason\":null,\"requested_mode\":\"semantic\",\"rerank_applied\":false,\"rerank_reason\":null,\"rerank_requested\":false,\"reranker_model\":null,\"runtime_backend_fallback\":null,\"runtime_backend_requested\":null,\"runtime_backend_used\":null,\"runtime_error_kind\":null,\"runtime_failure_stage\":null,\"selected_mode\":\"lexical\",\"warning\":null,\"warnings\":[],\"zero_result_reason\":null}"}]
```

Verdict: `OK for fallback metadata; BUG/follow-up: no lexical results surfaced`

## Phase D - Cross-project

### Step 18 - search-all

Tool and arguments:

```json
llm_wiki_search_all_test({
  "query": "diet",
  "mode": "auto",
  "limit": 10
})
```

Raw output:

```text
Wall time: 0.0890 seconds
Output:
[{"type":"text","text":"{\"error\":{\"message\":\"llm-wiki search-all diet --format json --mode auto --limit 10 failed with status exit status: 1: Error: search index missing for project llm-wiki-framework-headroom-impl; run `llm-wiki index --project llm-wiki-framework-headroom-impl`\\n\"}}]
```

Verdict: `BUG: search-all hard-fails on one missing project index instead of returning per-project readiness`

### Step 19 - include / exclude

Include filter.

Tool and arguments:

```json
llm_wiki_search_all_test({
  "query": "diet",
  "mode": "auto",
  "include": ["keto-diet"],
  "limit": 10
})
```

Raw output:

```text
Wall time: 0.0308 seconds
Output:

[{"type":"text","text":"{\"backend_status\":null,\"embedding_model\":\"embeddinggemma-300m-q8_0\",\"fallback_reason\":\"thresholds_unconfigured\",\"mode_selection_reason\":\"auto_lexical_fallback\",\"profile\":\"balanced\",\"project_id\":null,\"project_name\":null,\"projects\":[{\"backend_status\":{\"freshness\":\"fresh\",\"indexed_files\":10,\"message\":null,\"open_mode\":\"read_only_immutable\",\"state\":\"ready\",\"store_path\":\"/Users/nicolasmartino/.llm_wiki-test/indexes/keto-diet/qmd-rs.sqlite\"},\"fallback_reason\":\"thresholds_unconfigured\",\"mode_selection_reason\":\"auto_lexical_fallback\",\"project_id\":\"keto-diet\",\"readiness_reason\":null,\"requested_mode\":\"auto\",\"rerank_applied\":false,\"rerank_reason\":null,\"rerank_requested\":false,\"result_count\":10,\"runtime_backend_fallback\":null,\"runtime_backend_requested\":null,\"runtime_backend_used\":null,\"runtime_error_kind\":null,\"runtime_failure_stage\":null,\"selected_mode\":\"lexical\",\"zero_result_reason\":null}],\"query\":\"diet\",\"readiness_reason\":null,\"requested_mode\":\"auto\",\"rerank_applied\":false,\"rerank_reason\":null,\"rerank_requested\":false,\"reranker_model\":null,\"runtime_backend_fallback\":null,\"runtime_backend_requested\":null,\"runtime_backend_used\":null,\"runtime_error_kind\":null,\"runtime_failure_stage\":null,\"selected_mode\":\"lexical\",\"warning\":null,\"warnings\":[],\"zero_result_reason\":null}"}]
```

Exclude filter.

Tool and arguments:

```json
llm_wiki_search_all_test({
  "query": "diet",
  "mode": "auto",
  "exclude": ["llm-wiki-framework-headroom-impl"],
  "limit": 10
})
```

Raw output:

```text
Wall time: 0.0289 seconds
Output:

[{"type":"text","text":"{\"backend_status\":null,\"embedding_model\":\"embeddinggemma-300m-q8_0\",\"fallback_reason\":\"thresholds_unconfigured\",\"mode_selection_reason\":\"auto_lexical_fallback\",\"profile\":\"balanced\",\"project_id\":null,\"project_name\":null,\"projects\":[{\"backend_status\":{\"freshness\":\"fresh\",\"indexed_files\":10,\"message\":null,\"open_mode\":\"read_only_immutable\",\"state\":\"ready\",\"store_path\":\"/Users/nicolasmartino/.llm_wiki-test/indexes/keto-diet/qmd-rs.sqlite\"},\"fallback_reason\":\"thresholds_unconfigured\",\"mode_selection_reason\":\"auto_lexical_fallback\",\"project_id\":\"keto-diet\",\"readiness_reason\":null,\"requested_mode\":\"auto\",\"rerank_applied\":false,\"rerank_reason\":null,\"rerank_requested\":false,\"result_count\":10,\"runtime_backend_fallback\":null,\"runtime_backend_requested\":null,\"runtime_backend_used\":null,\"runtime_error_kind\":null,\"runtime_failure_stage\":null,\"selected_mode\":\"lexical\",\"zero_result_reason\":null}],\"query\":\"diet\",\"readiness_reason\":null,\"requested_mode\":\"auto\",\"rerank_applied\":false,\"rerank_reason\":null,\"rerank_requested\":false,\"reranker_model\":null,\"runtime_backend_fallback\":null,\"runtime_backend_requested\":null,\"runtime_backend_used\":null,\"runtime_error_kind\":null,\"runtime_failure_stage\":null,\"selected_mode\":\"lexical\",\"warning\":null,\"warnings\":[],\"zero_result_reason\":null}"}]
```

Verdict: `OK for include/exclude changing searched project set; follow-up: project-level result_count appears but no top-level hits are pasted/visible`

## Report

| Step | Tool | Verdict | Notes (raw signal) |
| --- | --- | --- | --- |
| 1 status | `llm_wiki_status_test` | BUG | Status reports installed/version/paths only; no project registration, index freshness, or search readiness. |
| 2 register | `llm_wiki_register_test`, `llm_wiki_status_test` | OK / BUG | Register says `Project already registered: keto-diet`; status still does not confirm registered project state. |
| 3 index / --force | `llm_wiki_index_test` | OK | Both normal and forced index report `Indexed project: keto-diet (10 files)`. |
| 4 calibrate | N/A | expected-fresh | Calibration skipped; semantic/hybrid threshold failures are expected freshness/readiness state. |
| 5 wiki read | `llm_wiki_read_test` | BUG | `content` is `<<ccr:619ddffe7a91,string,2.5KB>>`, not real Markdown. |
| 6 raw read | `llm_wiki_read_test` | BUG | Conventional raw files not found; reading `raw` says `raw is not a readable file`; MCP did not expose a way to discover a raw file path. |
| 7 abs/rel path | `llm_wiki_read_test` | OK / BUG | Absolute and relative wiki path both succeed and match same sha256, but both return CCR placeholder content. |
| 8 traversal | `llm_wiki_read_test` | OK | Traversal returns canonicalization error, not escaped file contents. |
| 9 missing file | `llm_wiki_read_test` | OK | Missing wiki page returns canonicalization error, not empty success. |
| 10 lexical | `llm_wiki_search_test` | BUG | Fresh index with 10 files, but lexical queries return no hits; some outputs omit `results` and have `zero_result_reason: null`. |
| 11 auto | `llm_wiki_search_test` | OK / follow-up | Auto returns lexical fallback with `fallback_reason: thresholds_unconfigured` and `mode_selection_reason: auto_lexical_fallback`; returned `results: []`. |
| 12 semantic | `llm_wiki_search_test` | expected-fresh | Hard readiness failure has `readiness_reason: thresholds_unconfigured` and names `llm-wiki eval calibrate --record`. |
| 13 hybrid | `llm_wiki_search_test` | expected-fresh | Same readiness behavior and calibration guidance as semantic. |
| 14 class/status filters | `llm_wiki_search_test` | BUG/follow-up | Output says `filters excluded all matched hits`; base lexical hit path is broken, so class/status metadata cannot be verified. |
| 15 rerank true | `llm_wiki_search_test` | BUG | `rerank_requested: true`, `rerank_applied: false`, `reranker_model: null`, but `rerank_reason` is missing/null. |
| 16 rerank false | `llm_wiki_search_test` | OK | `rerank_requested: false`, `rerank_applied: false`, `rerank_reason: null`. |
| 17 allow_lexical_fallback | `llm_wiki_search_test` | OK / follow-up | Semantic with fallback returns `selected_mode: lexical`, `fallback_reason: thresholds_unconfigured`; no hits surfaced. |
| 18 search-all | `llm_wiki_search_all_test` | BUG | Search-all hard-fails on missing index for `llm-wiki-framework-headroom-impl` instead of per-project readiness. |
| 19 include/exclude | `llm_wiki_search_all_test` | OK / follow-up | Include/exclude changes searched project set; project-level `result_count: 10` appears but top-level hits are not visible in returned payload. |

## Pass Criteria Mapping

- Issue 0.1 (read/CCR): FAIL. Step 5 returns `<<ccr:...>>`; step 6 could not verify raw read because raw file discovery is unavailable through MCP read.
- Issue 0.2 (class/status): BLOCKED/FAIL. Step 10 has no hits to inspect; step 14 cannot verify filters because base lexical search returns no inspectable hits.
- Issue 0.3 (auto/thresholds): PASS for readiness/fallback contract. Step 11 falls back to lexical with explicit `thresholds_unconfigured`; step 12 names `llm-wiki eval calibrate --record`.
- Issue 0.4 (rerank): FAIL. Step 15 has no `rerank_reason` despite requested rerank not applying and no reranker model configured.

## Posture

- [ ] New project (fresh `init`)
- [x] Existing project not yet migrated (register + index an existing wiki)
- [ ] Migration candidate (an external project moving onto the MCP)

## High-level Classification

This run looks like one major blocker plus independent Phase 0 follow-ups:

- Blocker: MCP read/search result surfacing is not reliable enough for wiki usage. `read` returns CCR placeholders and lexical search returns no inspectable hits despite a fresh 10-file index.
- Independent failure: rerank reporting is incomplete when rerank is requested but unavailable.
- Cross-project failure: `search-all` hard-fails on one missing project index instead of reporting per-project readiness.
