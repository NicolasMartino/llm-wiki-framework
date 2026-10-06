# Eval: Headroom 0.32 Wildcard Exclude Field Test

- Document Class: Eval
- Status: Rejected
- Date: 2026-07-21
- Category: Headroom, Codex host, MCP surface, provenance
- Scope: Live Codex smoke using `HEADROOM_EXCLUDE_TOOLS=*llm_wiki*` plus the
  source wrapper's generated explicit route-key exclusions.
- Sources:
  - Source package version: `llm-wiki 0.2.13`.
  - Managed MCP binary version reported by nested Codex: `llm-wiki 0.2.12`.
  - Headroom version: `0.32.0`.
  - Command:
    `HEADROOM_EXCLUDE_TOOLS='*llm_wiki*' cargo run --quiet -- headroom -v -- wrap codex --port 8794 -a never exec ...`.

## Verdict

Rejected. The broad wildcard exclude did not preserve full `llm_wiki_*` MCP
payloads under the live Codex/OpenAI-Responses path. `llm_wiki_read` still
arrived as a CCR marker, and full `llm_wiki_search` still omitted both
`result_count` and `results`.

## Observed Output

Nested Codex returned:

```json
{
  "status_version": "0.2.12",
  "read_content_kind": "ccr_marker",
  "read_content_prefix": "<<ccr:587da98b165c,html,37.6KB>>",
  "read_byte_len": 38543,
  "read_sha256_present": true,
  "search_result_count_present": false,
  "search_results_present": false,
  "first_search_path": null
}
```

## Implication

`*llm_wiki*` is useful defensive matching and should remain in the launcher, but
it is still not a provenance boundary. The active mitigation remains compact
search plus loud payload-integrity checks. Exact full reads under Headroom remain
unproven and should be treated as failed delivery when CCR markers or
hash/length mismatches appear.

The managed-binary version caveat was removed by the follow-up clean run in
`wiki/evals/headroom-0-32-managed-0-2-13-field-test.eval.md`, which reproduced
the same full read/search failures with nested MCP status reporting `0.2.13`.
