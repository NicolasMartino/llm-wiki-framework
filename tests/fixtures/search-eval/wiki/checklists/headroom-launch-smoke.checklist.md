# Headroom Launch Smoke

- Document Class: Checklist
- Status: Active; Option A supported path validated for Headroom 0.32 + managed
  `llm-wiki 0.2.13`; full read/search payload preservation remains unsupported
- Date: 2026-07-18
- Category: Agent runtime, Headroom, provenance smoke
- Scope: Optional smoke for users who deliberately run Headroom in front of an
  agent while keeping LLM Wiki wiki/raw access routed through framework MCP
  tools, detecting host/proxy compression, failing loudly on mutated read/search
  payloads, and using compact paged search when full search outputs cannot
  survive the active Headroom path.
- Sources:
  - wiki/decisions/headroom-single-posture-mcp-first.decision.md
  - wiki/references/headroom-context-compression.reference.md
  - wiki/plans/headroom-passthrough-launcher.plan.md
  - wiki/plans/headroom-mcp-field-test-repair.plan.md
- Related:
  - wiki/checklists/mcp-field-test.checklist.md
  - wiki/plans/headroom-mcp-field-test-repair.plan.md

## Purpose

This checklist is not the MCP correctness gate. The authoritative field-test gate
stays `wiki/checklists/mcp-field-test.checklist.md`, which is pure MCP and has no
proxy or launch assumptions.

Use this smoke only when a user explicitly wants to run Headroom. It checks the
two gated provenance facts and records the exclude-list behavior only as
observational evidence.

## Known Field-Test Failure (2026-07-18)

A Codex session launched via `llm-wiki headroom wrap` against `keto-diet`
reported `HEADROOM_MCP_READ=off` and a deployed `llm-wiki 0.2.11`, but did not
pass this smoke:

1. `llm_wiki_index` output was Headroom-compressed.
2. `llm_wiki_read` returned CCR placeholders in `content` instead of real
   wiki/raw text.
3. `llm_wiki_search` and include-filtered `llm_wiki_search_all` omitted actual
   result arrays despite ready metadata and result counts.
4. Unfiltered/exclude-filtered `search-all` failed globally on a stale
   registered project with a missing wiki root.

Track the repair in `wiki/plans/headroom-mcp-field-test-repair.plan.md`. The
2026-07-18 run remains a failing historical gate for Headroom 0.24.0-style
full-payload behavior, but current Headroom versions must be classified by a
fresh run rather than by assuming the old provider-path behavior still applies.

## Known Field-Test Failure (2026-07-20)

A nested read-only Codex probe launched through
`llm-wiki headroom -- wrap codex --port 8791` isolated a current Headroom 0.32.0
proxy while an older 0.24.0 proxy remained active on port 8787. Preflight passed:
`/livez` reported Headroom 0.32.0, `headroom --version` reported 0.32.0,
`llm-wiki --version` reported 0.2.12, `llm_wiki_status` returned full project
readiness, and verbose launcher output included production/test `llm_wiki_*`
route-key exclusions.

The full-payload gate still failed:

1. `llm_wiki_read` for `wiki/index.md` returned
   `<<ccr:72660c4ccf86,html,36.9KB>>`.
2. `llm_wiki_read` for this checklist returned
   `<<ccr:d1df4dfd3e66,string,6.7KB>>`.
3. `llm_wiki_search` reported hybrid metadata but omitted `results` with no
   `zero_result_reason`.
4. Include-filtered `llm_wiki_search_all` reported `result_count:6` but omitted
   hit arrays.
5. `llm_wiki_index` stdout reported 104 indexed files, but stderr was replaced
   by `<<ccr:b9a24f0f5457,string,2.0KB>>`.

Record: `wiki/evals/headroom-0-32-codex-route-key-field-test.eval.md`.

## Compact Search Field-Test Result (2026-07-21)

A post-fix nested Codex probe launched through
`llm-wiki headroom -- wrap codex --port 8792` against Headroom 0.32.0 recorded
that compact search survives while full read/search payloads still do not:

1. `llm_wiki_status` returned installed/project/index readiness JSON.
2. `llm_wiki_read` for `wiki/index.md` returned
   `<<ccr:0a005237b58f,html,37.1KB>>`.
3. Compact `llm_wiki_search` with `compact:true`, `page_size:1` returned a
   `results` array with first hit `wiki/checklists/headroom-launch-smoke.checklist.md`.
4. Compact include-filtered `llm_wiki_search_all` returned the same first hit.
5. Full non-compact `llm_wiki_search` still lost `results` and `result_count`.

Record: `wiki/evals/headroom-0-32-compact-search-field-test.eval.md`.

Wildcard exclude follow-up 2026-07-21:
`HEADROOM_EXCLUDE_TOOLS='*llm_wiki*' cargo run --quiet -- headroom -v -- wrap
codex --port 8794 ...` still returned a CCR marker for `llm_wiki_read
wiki/index.md` and full `llm_wiki_search` still omitted `result_count` and
`results`. Record:
`wiki/evals/headroom-0-32-wildcard-exclude-field-test.eval.md`.

Managed 0.2.13 follow-up 2026-07-21:
`/Users/nicolasmartino/.llm_wiki/bin/llm-wiki headroom -v -- wrap codex --port
8795 ...` reported nested MCP status `0.2.13`. Full read still returned a CCR
marker and full search still omitted hit fields, while compact search and
compact include-filtered `search-all` both returned real hits. Record:
`wiki/evals/headroom-0-32-managed-0-2-13-field-test.eval.md`.

Follow-up source review of installed `headroom-ai 0.24.0` confirmed why this
happened on that Codex path: Headroom's Chat-Completions/Anthropic content-router
path honored `HEADROOM_EXCLUDE_TOOLS`, but its OpenAI-Responses handler
compressed tool-output `output` strings without consulting that exclude list. The
llm-wiki launcher could set the right environment and still be ineffective for
Codex full-payload protection on that Headroom version.

Current source verification on 2026-07-20 found installed `headroom-ai 0.32.0`.
Its OpenAI-Responses handler appears to build a call-id to function-name map,
check `is_tool_excluded`, protect excluded output slots, and protect
`headroom_retrieve` outputs. The compression floor is 512 bytes for live
Responses units and applies to the whole output string when the output is a
string. Therefore this checklist must record the exact Headroom version and
first test full excluded-tool payloads before using any pagination workaround.

The same date's Claude attempt stopped before Phase A because the running
session had only `llm-wiki-test` connected (`mcp__llm-wiki-test__llm_wiki_*_test`,
backed by `llm-wiki-test 0.2.8`) while the requested test posture required
production unsuffixed `llm_wiki_*` tools. The production `llm-wiki 0.2.11`
binary existed on disk but was not connected to Claude's project MCP config, and
Claude requires a relaunch after MCP config changes. This is a valid setup
blocker, not a usable production field-test run.

## Preconditions

1. Headroom is installed and the desired agent (`codex` or `claude`) is available.
2. The target project has the llm-wiki MCP server configured for the agent.
3. No existing conflicting Headroom proxy is already bound to the requested port.
4. The exact Headroom package/binary version is recorded before interpreting
   payload behavior.
5. The connected MCP instance matches the run target:
   - production runs expose unsuffixed `llm_wiki_*` tools and `llm_wiki_status`
     reports the expected production version;
   - test-instance runs are explicitly labeled as `_test` and expose only the
     `_test` tool names.
   A mismatch stops the run before Phase A. On Claude Code, fix `.mcp.json` and
   relaunch the session; MCP servers are loaded at startup.

## Procedure

1. Launch the agent through the passthrough command:

   ```bash
   llm-wiki headroom -- wrap codex
   ```

   Use `--headroom-bin <PATH>` if `headroom` is not on `PATH`.

2. In the launched session, verify that `HEADROOM_MCP_READ=off` is inherited by
   the Headroom process tree.

3. In the same session, perform wiki/raw access through the `llm_wiki_*` MCP
   tools. At minimum, read `wiki/index.md` with `llm_wiki_read`, verify that
   `content` is real text rather than `<<ccr:...>>`, and recompute `byte_len`
   plus `sha256` when content is present.

4. Run the pure MCP correctness checklist by reference:
   `wiki/checklists/mcp-field-test.checklist.md`.

5. Observe whether Headroom proxy output leaves full `llm_wiki_*` tool output
   uncompressed under `HEADROOM_EXCLUDE_TOOLS` (`*llm_wiki*` plus explicit
   route-key entries). For Headroom 0.24.0 on
   Codex/OpenAI-Responses, expect compression. For Headroom 0.32.0, source
   inspection indicates Responses excludes should be honored, but the live run is
   authoritative.

6. If full search/search-all payloads fail, retry search with compact paging:
   `compact=true`, `page_size`, and `offset` in MCP calls (or `--compact`,
   `--page-size`, and `--offset` in CLI calls). Verify compact result pages
   contain real inspectable hits and always include `results`.

7. Do not use read pagination for this smoke. A read `content` hash/length
   mismatch, CCR marker, or compressed envelope is a loud failure to record,
   not a condition this checklist repairs.

## Pass Criteria

The smoke passes only if:

1. `HEADROOM_MCP_READ=off` is inherited by the Headroom process tree.
2. wiki/raw reads in the session route through `llm_wiki_*` MCP tools.
3. The run records the exact Headroom version and whether the active provider
   path honors tool excludes for the actual llm-wiki tool names.
4. The referenced pure-MCP field-test steps pass inside the launched session when
   the active Headroom provider path honors tool excludes; otherwise full-payload
   compression is recorded as a version/path limitation.
5. On provider paths that do not preserve full payloads, compact paged search
   retrieves real inspectable search hits, and read payload corruption fails
   loudly with explicit CCR/compression/hash evidence. Exact large reads are
   performed outside Headroom.

The smoke does not pass merely because proxy output looks uncompressed.
`HEADROOM_EXCLUDE_TOOLS` is best-effort and not a provenance boundary.

## Record

Record the run as an eval only when there is a durable transcript with:

1. exact launch command and Headroom version;
2. host and agent name/version;
3. evidence for `HEADROOM_MCP_READ=off` inheritance;
4. evidence that wiki/raw access used `llm_wiki_*` MCP tools; and
5. the pure-MCP field-test result.
