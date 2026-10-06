# Eval: Headroom 0.32 Compact Search Field Test

- Document Class: Eval
- Status: Candidate
- Date: 2026-07-21
- Category: Headroom, Codex host, compact search, MCP surface, provenance
- Scope: Records the post-fix Headroom-launched Codex field test after compact
  search and loud payload-integrity detection landed.
- Sources:
  - Commit: `a04a270` (`Implement Headroom-safe compact search`)
  - Nested command: `rtk /Users/nicolasmartino/.llm_wiki/bin/llm-wiki headroom -- wrap codex --port 8792 -a never exec -C /Users/nicolasmartino/Documents/local_llm_wiki/llm_wiki_framework_headroom_impl --sandbox read-only --output-last-message /tmp/headroom-compact-search-field-test-last.md ...`
  - Field-test result artifact: `/tmp/headroom-compact-search-field-test-last.md`
  - Related checklist: `wiki/checklists/headroom-launch-smoke.checklist.md`
  - Related plan: `wiki/plans/headroom-mcp-field-test-repair.plan.md`

## Verdict

Mixed. The overall Headroom launch smoke still fails because `llm_wiki_read`
returned a CCR placeholder and non-compact `llm_wiki_search` lost its result
payload. The compact-search mitigation succeeded: both compact
`llm_wiki_search` and compact include-filtered `llm_wiki_search_all` returned
real inspectable hit arrays under Headroom 0.32.0.

This validates compact paged search as the current Headroom-safe search path,
while keeping read pagination deferred and treating full read payloads as loud
failures when Headroom mutates them.

## Setup

- Headroom proxy: `http://127.0.0.1:8792/livez`.
- Headroom version: `0.32.0`.
- llm-wiki version: `0.2.12`.
- Managed binary under test: `/Users/nicolasmartino/.llm_wiki/bin/llm-wiki`.
- Host: nested read-only Codex launched through `llm-wiki headroom -- wrap
  codex --port 8792`.
- Safety: no files edited in the nested run; `headroom_read` was not used.

The default port `8787` still had an older Headroom 0.24.0 proxy, so this run
used port `8792` to isolate Headroom 0.32.0.

## Results

| Probe | Classification | Evidence |
| --- | --- | --- |
| `/livez` | `full_payload` | Healthy Headroom proxy on raw retry; the nested `rtk curl` first reported `FAILED: curl`. |
| `llm_wiki_status` | `full_payload` | Installed/project/index readiness JSON survived. |
| `llm_wiki_read wiki/index.md` | `ccr_placeholder` | `content` was `<<ccr:0a005237b58f,html,37.1KB>>`, not real wiki text. |
| Compact `llm_wiki_search` | `compact_payload_survived` | `results` array present, `result_count:4`, first hit `wiki/checklists/headroom-launch-smoke.checklist.md`. |
| Compact `llm_wiki_search_all` include-filtered to this project | `compact_payload_survived` | `results` array present, `result_count:4`, first hit `wiki/checklists/headroom-launch-smoke.checklist.md`. |
| Full `llm_wiki_search` | `omitted_payload` | Metadata JSON survived, but `results` and `result_count` were missing. |

## Conclusion

Compact search is usable under the active Headroom 0.32.0 Codex path. Full
search and read payloads are still not safe under this path, so field-test
procedure should use compact search for Headroom-launched retrieval and continue
to classify read CCR markers or hash/length mismatches as failed delivery.

Next work should not start with read pagination. If search needs more Headroom
margin, reduce the compact envelope further or add an even smaller diagnostic
shape. If exact wiki/raw reads are required under Headroom, treat that as a
separate design decision with explicit cost, not as part of the compact-search
repair.
