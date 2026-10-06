# Eval: Headroom 0.32 Managed 0.2.13 Field Test

- Document Class: Eval
- Status: Rejected
- Date: 2026-07-21
- Category: Headroom, Codex host, MCP surface, compact search, provenance
- Scope: Clean live Codex smoke after the managed llm-wiki binary and manifest
  were refreshed to `0.2.13`.
- Sources:
  - Managed binary: `/Users/nicolasmartino/.llm_wiki/bin/llm-wiki`.
  - Headroom version: `0.32.0`.
  - Command:
    `/Users/nicolasmartino/.llm_wiki/bin/llm-wiki headroom -v -- wrap codex --port 8795 -a never exec --output-last-message /tmp/headroom-0-32-managed-0213-smoke.md ...`.
  - Related evals:
    `wiki/evals/headroom-0-32-compact-search-field-test.eval.md`,
    `wiki/evals/headroom-0-32-wildcard-exclude-field-test.eval.md`.

## Verdict

Rejected for full-payload preservation. The version caveat from the prior
wildcard run is gone: `llm_wiki_status` reported `0.2.13` from the nested MCP
server. Full `llm_wiki_read` still arrived as a CCR marker, and full
`llm_wiki_search` still omitted both `result_count` and `results`.

The compact-search mitigation is confirmed again on the clean managed `0.2.13`
run. Compact `llm_wiki_search` and compact include-filtered
`llm_wiki_search_all` both returned real inspectable results under Headroom
0.32.0.

## Observed Output

Nested Codex returned:

```json
{
  "status_version": "0.2.13",
  "read_content_kind": "ccr_marker",
  "read_content_prefix": "<<ccr:8969776157a9,html,38.1KB>>",
  "read_byte_len": 38977,
  "read_sha256_present": true,
  "full_search_result_count_present": false,
  "full_search_results_present": false,
  "full_search_first_path": null,
  "compact_search_attempted": true,
  "compact_search_result_count_present": true,
  "compact_search_results_present": true,
  "compact_search_first_path": "wiki/checklists/headroom-launch-smoke.checklist.md",
  "compact_search_error": null,
  "compact_search_all_attempted": true,
  "compact_search_all_result_count_present": true,
  "compact_search_all_results_present": true,
  "compact_search_all_first_path": "wiki/checklists/headroom-launch-smoke.checklist.md",
  "compact_search_all_error": null
}
```

## Implication

`*llm_wiki*` and explicit route-key exclusions are not enough to preserve full
payloads under the current Headroom 0.32.0 Codex/OpenAI-Responses path, even
with the managed MCP server on `llm-wiki 0.2.13`. Compact search remains the
working Headroom search path. Exact full reads under Headroom remain failed
delivery when CCR markers or hash/length mismatches appear; read pagination
remains a separate design decision, not part of this repair slice.
