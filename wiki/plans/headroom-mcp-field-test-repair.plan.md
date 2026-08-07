# Plan: Headroom MCP Field-Test Repair

- Document Class: Plan
- Status: Active
- Date: 2026-07-18
- Updated: 2026-08-01
- Category: Headroom, MCP surface, host wiring, provenance, search robustness
- Scope: Repair and correctly frame the 2026-07-18 Codex + Headroom field-test
  failures after the `llm-wiki headroom` passthrough launch: hosts must connect
  the requested production MCP server/tool namespace before Phase A; the
  Headroom 0.24.0 Codex/OpenAI-Responses exclude failure must be recorded as
  historical/version-scoped evidence; current Headroom 0.32.0 + managed
  `llm-wiki 0.2.13` field evidence selects Option A; llm-wiki must retain loud
  compression detection, make search usable through compact paged output, keep
  exact large reads outside Headroom, isolate stale `search-all` projects, and
  keep normal search/index output diagnosable.
- Sources:
  - User-supplied 2026-07-18 field-test report for `/Users/nicolasmartino/Documents/keto_diet`
    (`project_id: keto-diet`) launched via `llm-wiki headroom wrap`.
  - User-supplied 2026-07-18 Claude pre-Phase-A blocker: project `.mcp.json`
    exposed only `llm-wiki-test` / `_test` tools backed by `llm-wiki-test 0.2.8`,
    while the task required production unsuffixed `llm_wiki_*` and the production
    binary on disk was `0.2.11` but not connected to Claude.
  - User-supplied 2026-07-18 review of this plan, backed by installed
    `headroom-ai 0.24.0` source inspection.
  - Installed Headroom source read 2026-07-18:
    `headroom/transforms/content_router.py` honors `exclude_tools` only after
    building a tool-call id to name map for Chat-Completions/Anthropic-style
    messages; `headroom/proxy/handlers/openai.py` extracts OpenAI Responses
    tool-output `item["output"]` values for compression and has no
    `exclude_tool` hook, except a hardcoded `headroom_retrieve` call-id
    protection.
  - User-supplied 2026-07-20 review and local verification of installed
    `headroom-ai 0.32.0`.
  - Installed Headroom source read 2026-07-20:
    `headroom/proxy/handlers/openai.py` now defines a 512-byte Responses router
    floor, extracts whole string `output` slots or per text-part slots, builds a
    `function_name_by_call_id` map from Responses `function_call` items,
    computes excluded call ids with `is_tool_excluded`, protects matching
    outputs, and keeps special `headroom_retrieve` output protection;
    `headroom/transforms/compression_units.py` defines `CompressionUnit.min_bytes
    = 512` and skips non-`live` units.
  - `wiki/checklists/headroom-launch-smoke.checklist.md`
  - `wiki/checklists/mcp-field-test.checklist.md`
  - `wiki/plans/headroom-passthrough-launcher.plan.md`
  - `wiki/decisions/headroom-single-posture-mcp-first.decision.md`
  - `wiki/plans/hybrid-default-and-mcp-surface-repair.plan.md`
- Related:
  - `wiki/plans/mcp-onboarding-init-register.plan.md`
  - `wiki/references/headroom-context-compression.reference.md`
  - `wiki/evals/codex-mcp-field-test-pass.eval.md`
  - `wiki/evals/claude-mcp-field-test-pass.eval.md`

## Problem

The local deployment path worked: the managed and Cargo-installed `llm-wiki`
binary reported `0.2.11`, LLM search was enabled with the balanced profile, model
artifacts and licenses were already verified, runtime probes passed, and a direct
managed-binary search of the framework project returned the Headroom passthrough
launcher plan as the top result.

The live Headroom/Codex run did not pass the Headroom launch smoke or the pure
MCP field-test checklist inside that session:

1. `llm_wiki_index` and forced `llm_wiki_index` output were replaced by
   Headroom compression envelopes (`[233 items compressed to 4...]`) even though
   `llm_wiki_*` tools were intended to be excluded.
2. `llm_wiki_read` returned provenance metadata but replaced `content` with CCR
   placeholders such as `<<ccr:619ddffe7a91,string,2.5KB>>` for `wiki/index.md`
   and `<<ccr:acac9764bcf3,string,1.2KB>>` for a raw source file.
3. `llm_wiki_search` returned ready/fresh metadata for lexical, auto, semantic,
   hybrid, rerank, and fallback probes, but omitted the `results` array entirely.
4. Include-filtered `llm_wiki_search_all` reported `result_count:10` for
   `keto-diet` but also omitted actual hits.
5. Unfiltered and exclude-filtered `llm_wiki_search_all` failed globally because
   one registered project had a missing wiki root:
   `/Users/nicolasmartino/Documents/diet/cycling_ketogenic/wiki`.
6. Index/search/doctor runs emitted very large GGUF/Metal stderr logs, making
   failures hard to inspect.

Subsequent source review confirmed the Headroom-side root cause for that
specific Codex payload failure as inspected against Headroom 0.24.0: Headroom
had two separate compression paths. The Chat-Completions/Anthropic path built a
tool-call id to tool-name map and honored `HEADROOM_EXCLUDE_TOOLS`. The
Codex/OpenAI-Responses path instead compressed string `output` values from
`function_call_output`, `custom_tool_call_output`, `local_shell_call_output`, and
`apply_patch_call_output` above its size floor without consulting
`exclude_tools`. The launcher's generated names could be correct and still have
no effect on that version/path. Therefore the 2026-07-18 failure was not
primarily a missing namespace string in llm-wiki.

Current source verification on 2026-07-20 changes the next action. The installed
Headroom is now `headroom-ai 0.32.0`, and its OpenAI-Responses handler appears
to implement the upstream exclusion pattern: it correlates Responses
`function_call` items with output items by `call_id`, resolves function names,
computes excluded call ids with `is_tool_excluded`, and protects matching output
slots. It also protects `headroom_retrieve` outputs. The fixed Responses
compression floor is 512 bytes, applies to the whole `item["output"]` string
when the output is a string, and only `live` units are eligible. That makes the
old "Responses ignores excludes" diagnosis historical for 0.24.0, not a safe
statement about the current installed proxy. A fresh Headroom 0.32.0 live field
test on 2026-07-20 classified the current Codex/OpenAI path and still failed the
full-payload gate: `llm_wiki_read` returned CCR placeholders, search/search-all
omitted hit arrays, and index stderr was CCR-compressed despite generated
`llm_wiki_*` route-key exclusions.

The field report's compressed `headroom_retrieve` payload remains an open
reproduction target. Current 0.32.0 source protects retrieve outputs, so the old
result was likely version skew or a nested-output/list-compression path. If
retrieval cannot round-trip on the current proxy, "compress and retrieve" is not
a viable fallback for wiki/raw correctness.

The reported `0.2.10` vs `0.2.11` mismatch is not a product defect for this
repair: the deployed binary under test was intentionally updated to `0.2.11`.
Future field-test reports should state the actual version under test.

A separate Claude Code launch stopped before Phase A for a different reason: the
session had only `llm-wiki-test` connected, exposing
`mcp__llm-wiki-test__llm_wiki_*_test` tools backed by `llm-wiki-test 0.2.8`. The
task required production unsuffixed `llm_wiki_*` tools, and the production
`llm-wiki 0.2.11` binary on disk was not registered in that project's `.mcp.json`
or connected to the running Claude session. Because Claude loads MCP servers at
startup, this is a relaunch/configuration blocker, not something a running field
test can repair by silently substituting `_test` tools.

## Decision

Create this plan rather than reopening the passthrough launcher implementation
plan. The passthrough plan proved the CLI process model and env injection. The
2026-07-18 failure was outside that launcher for the Headroom 0.24.0 path that
was inspected, while the current installed Headroom 0.32.0 source appears to
have added the missing Responses exclude support. The repair therefore starts by
recording exact Headroom version/provider behavior and proving route-key
matching in a fresh live run, not by assuming the historical failure still
applies.

This plan does not restore the retired Headroom proxy apparatus. The governing
posture still holds: `HEADROOM_EXCLUDE_TOOLS` is best-effort and not a provenance
boundary. The repair target is split:

1. Live Headroom runs must record the exact Headroom version and provider path,
   then verify whether the active path honors tool exclusions for the actual
   `function_call.name` values emitted by the host.
2. The current Headroom 0.32 + Codex path does not preserve full read/search
   payloads even with the managed `llm-wiki 0.2.13` binary, so the chosen product
   posture is Option A: compact search is supported under Headroom, exact large
   reads are outside Headroom, and read pagination is not added in this repair.
3. When the path compresses or strips required payloads, llm-wiki uses
   host-agnostic mitigations it owns: fail-fast instance preflight,
   compression-marker detection, and compact small-output search surfaces that
   stay under measured Headroom compression floors.
4. llm-wiki should separately fix Headroom-independent product defects:
   `search-all` stale-project isolation and excessive GGUF diagnostics.

## Deliverable

A repeatable Headroom/Codex field test can run `llm-wiki headroom -- wrap codex`
and produce an honest result:

- Before Phase A, the host proves it has connected the intended server instance:
  production tests expose unsuffixed `llm_wiki_*` tools backed by the expected
  production binary/version; test-instance runs explicitly opt into `_test`
  tools. A mismatch fails fast.
- If the active Headroom version/provider path honors tool excludes,
  full-payload `llm_wiki_read` and `llm_wiki_search` results remain real and
  uncompressed.
- If the active Headroom version/provider path mutates full outputs,
  Headroom-launched field tests use compact search for discovery and fail loudly
  on read CCR placeholders, compression envelopes, hash/length mismatches, or
  silently omitted hit arrays. Exact large reads are performed outside Headroom.
- `llm_wiki_search_all` reports per-project readiness/backend state in
  `projects[]`, returns flat globally ranked top-level hits with project
  attribution, and does not fail globally because one registered project is
  stale or missing.
- `llm_wiki_index` output is small, structured, and recoverable enough for field
  tests to confirm indexed-file counts and force rebuilds.
- Normal install/index/search/doctor output is usable for diagnosis without
  thousands of GGUF backend log lines.
- Read pagination work is out of scope for the current product posture. A future
  exact-read-under-Headroom design would require a separate decision that accepts
  the UX/API cost and accounts for the 512-byte floor applying to the entire
  JSON/MCP output string, not just the content field.

## Non-Goals

- Do not make Headroom required for llm-wiki correctness.
- Do not claim `HEADROOM_EXCLUDE_TOOLS` is a provenance guarantee.
- Do not reintroduce managed Headroom profiles, proxy health checks, version
  pins, `install --with-headroom`, or Headroom-specific installed assets.
- Do not weaken the `headroom_read` ban for `wiki/` and `raw/`.
- Do not hide real search/read failures by treating missing payload fields as
  success.

## Phase 0 - Capture The Failure As A Gate

Tasks:

1. Add the 2026-07-18 Headroom/Codex failure signals to the Headroom launch smoke
   checklist as known failing evidence.
2. Add the 2026-07-18 Claude setup blocker to the same gate: production
   Headroom field tests must not proceed when only the `_test` MCP server/tools
   are connected, or when the connected server binary version does not match the
   version under test.
3. Keep the pure MCP checklist unchanged as the correctness gate, but clarify in
   the Headroom smoke that a Headroom-launched run fails if content is a CCR
   placeholder, search hits are omitted, or the requested MCP instance is not
   connected.
4. Decide whether to promote the pasted transcripts into separate eval pages
   once a stable raw copy exists in the repo.

Verification:

- The wiki index and log point to this plan.
- The passthrough launcher plan no longer says only "optional live launch smoke
  pending"; it names this repair as the active follow-up.

## Phase 1 - Host MCP Instance Wiring Preflight

Tasks:

1. Define a required pre-Phase-A check for each host:
   - production field test: callable tools are unsuffixed `llm_wiki_status`,
     `llm_wiki_register`, `llm_wiki_index`, `llm_wiki_read`,
     `llm_wiki_search`, and `llm_wiki_search_all`;
   - test-instance field test: callable tools are the `_test` variants and the
     run header says so explicitly.
2. Require `llm_wiki_status` to report the expected binary name and version
   before any read/search/index step is trusted.
3. For Claude Code, document that MCP server changes require a new Claude
   session. A run with only `llm-wiki-test` connected must stop before Phase A
   rather than substituting `_test` tools for a production test.
4. Reuse the active MCP onboarding plan for the durable fix: `init`/`register`
   should make it hard for a project `.mcp.json` to point at an obsolete test
   instance when the operator expects production.

Verification:

- A Claude launch whose `.mcp.json` exposes only `llm-wiki-test` is classified as
  a setup blocker with no Phase A results.
- A correctly wired Claude relaunch exposes production unsuffixed tools and
  reports the expected production version before the checklist continues.

## Phase 2 - Re-Scope Headroom Provider-Path Evidence

Tasks:

1. Record the confirmed Headroom 0.24.0 historical source finding: the
   Chat-Completions/Anthropic content-router path honored `exclude_tools`, while
   the Codex/OpenAI-Responses handler compressed tool-output `output` strings
   without consulting `exclude_tools`.
2. Record the current Headroom 0.32.0 source finding: the Responses handler now
   builds a `call_id -> function name` map, computes excluded call ids with
   `is_tool_excluded`, protects matching output slots, protects
   `headroom_retrieve`, and uses a fixed 512-byte minimum for live Responses
   units.
3. Keep the launcher-generated `HEADROOM_EXCLUDE_TOOLS` coverage tests, because
   route keys matter again on Headroom versions that honor Responses excludes.
   Verify the emitted plain and qualified llm-wiki tool names match the
   `function_call.name` values the current Codex path sends.
4. Re-run a minimal live Codex + Headroom smoke under the current installed
   Headroom before adding pagination: status, index, read `wiki/index.md`, one
   raw read, lexical search, hybrid search, include-filtered `search-all`, and
   `headroom_retrieve`.
5. Draft an upstream Headroom issue or patch proposal only if the current live
   path still fails despite source-level exclude support. If it does fail, the
   issue must include exact Headroom version, function names, excluded env value,
   and whether the failure is normal output compression, lossless excluded-output
   compaction, or nested retrieve-output compression.

Verification:

- A field-test note records the active provider path and whether it honors
  `exclude_tools`; for Headroom 0.24.0 + Codex Responses, the historical answer
  is "does not honor," while Headroom 0.32.0 must be tested live.
- The fake-Headroom env test still asserts all production and test route keys
  expected by llm-wiki are present in the launched Headroom environment.
- The live 0.32.0 run either proves full-payload preservation or captures enough
  evidence to classify the remaining failure accurately before pagination work
  begins.

## Phase 3 - Detect Compression; Do Not Add Headroom Read Pagination

Tasks:

Implementation update 2026-07-21: the narrowed repair slice ships
compression-marker, read hash/length, and search-omission detection, but does
not add read pagination. Option A is now the chosen product posture: compact
search is supported for discovery under Headroom, while exact large `wiki/` and
`raw/` reads should be performed outside Headroom. Full file content is not
meaningfully shrinkable under the current Headroom floor and would require many
small chunks.

1. Keep full-file `llm_wiki_read` as the normal pure-MCP contract. It should
   continue to return real UTF-8 content, `sha256`, `byte_len`, and clear
   traversal/missing-file errors when no Headroom proxy mutates the payload.
2. First ship or reuse compression-marker detection for live field tests and
   consumers: `<<ccr:...>>`, `[N items compressed ...]`, and missing `content`
   when a page was requested are failures with an explicit remediation path.
3. Do not add a compression-aware read pagination surface in this repair. A
   Headroom-mutated full read is a failed delivery check, not a partially
   supported exact-read mode.
4. If exact page/raw content is required, rerun outside Headroom or otherwise
   use a host/provider path whose delivery integrity passes `sha256`/`byte_len`
   validation.
5. Reopen exact reads under Headroom only through a separate decision that
   accepts the UX/API cost of chunked reads and defines new acceptance criteria.

Verification:

- In a pure MCP session, full-file `llm_wiki_read` still returns real content for
  `wiki/index.md` and a raw source.
- Under `llm-wiki headroom -- wrap codex`, full-read CCR markers,
  compression envelopes, omitted content, or hash/length mismatches are recorded
  as failed delivery. The supported recovery path is to perform exact reads
  outside Headroom, not to page reads inside Headroom.
- Traversal and missing-file errors remain explicit and do not leak file content.

## Phase 4 - Detect Search Omission; Add Compact Search Mode

Tasks:

Implementation update 2026-07-21: the narrowed repair slice implements compact
search for CLI and MCP with `--compact`, `--page-size`, and `--offset`
(`compact`, `page_size`, and `offset` in MCP arguments). Full search JSON stays
the default. Compact mode emits essential hit fields only and pages over ranked
hits; it does not add read pagination or a separate `headroom_safe` read path.

Field-test update 2026-07-21: `wiki/evals/headroom-0-32-compact-search-field-test.eval.md`
shows the post-fix Headroom 0.32 Codex path still fails the overall smoke for
read and full search payloads, but compact `llm_wiki_search` and compact
include-filtered `llm_wiki_search_all` both returned real inspectable hits under
Headroom. Compact search is therefore the current viable Headroom search
mitigation; read pagination is out of scope under Option A.

Wildcard-exclude update 2026-07-21:
`wiki/evals/headroom-0-32-wildcard-exclude-field-test.eval.md` shows adding
`*llm_wiki*` to `HEADROOM_EXCLUDE_TOOLS` is useful defensive matching but does
not preserve full payloads on the live Headroom 0.32 Codex path: read still
returned a CCR marker and full search still omitted hit fields.

Clean managed-version update 2026-07-21:
`wiki/evals/headroom-0-32-managed-0-2-13-field-test.eval.md` removes the prior
managed-binary version caveat. A fresh Headroom 0.32 Codex run with nested MCP
status reporting `llm-wiki 0.2.13` still returned a CCR marker for full read and
omitted hit fields for full search, while compact search and compact
include-filtered `search-all` returned real hit arrays.

1. Keep full-result `llm_wiki_search` and `llm_wiki_search_all` as the normal
   pure-MCP contracts. They must include explicit `results` arrays or explicit
   zero/omission reasons.
2. Treat missing `results` when the response reports a ready backend,
   non-null `result_count`, or no zero-result reason as a live compression or
   serialization failure, not a successful search.
3. Use the implemented compact search surface when Headroom mutates full search
   output: `compact=true`, `page_size`, and `offset`. This is pagination over a
   small ranked hit list, not read/content pagination.
4. Keep compact search output to the smallest useful hit shape first:
   path/title/class/status/score/mode/backend and no snippet when needed to stay
   below the measured safe byte budget.
5. Ensure `result_count` without returned hits always has an explicit reason and
   a supported next call. Silent omission remains a bug.
6. Apply the same pagination/omission rules to `search-all` project entries.

Verification:

- In a pure MCP session, lexical, auto, semantic, hybrid, rerank true, rerank
  false, and fallback probes return inspectable hits on `keto-diet`.
- Under `llm-wiki headroom -- wrap codex`, full search payload mutation is
  recorded as a failed delivery check. Compact search then returns inspectable
  hits for the same probes, or fails explicitly with compression-marker evidence.
- Hits include class/status metadata where the target pages have metadata
  headers.
- Include-filtered compact `llm_wiki_search_all` returns actual flat,
  project-attributed hits for `keto-diet`, not only `result_count`.

## Phase 5 - Isolate Stale Projects In `search-all`

Tasks:

1. Extend `search-all` per-project degradation to cover missing project roots and
   missing wiki roots, not just missing or unusable index stores.
2. Return a per-project readiness/error object for stale projects such as a
   missing `/Users/nicolasmartino/Documents/diet/cycling_ketogenic/wiki`.
3. Keep include/exclude filtering semantics unchanged while making the remaining
   searched set robust.

Verification:

- An unfiltered `search-all` with one stale registered project exits 0 and
  returns ready results for healthy projects plus a per-project stale-project
  readiness entry.
- Excluding `keto-diet` no longer converts a stale remaining project into a
  top-level error.

## Phase 6 - Make Index/Search Diagnostics Usable

Tasks:

1. Suppress or redirect normal GGUF/Metal backend logs during install, doctor,
   index, and search unless verbose diagnostics explicitly request them.
2. Preserve enough runtime-probe detail in structured files or concise stderr to
   diagnose backend selection, fallback, and failures.
3. Keep stdout stable for JSON/MCP consumers.

Verification:

- Normal `llm-wiki search`, `index`, and `doctor` output is concise.
- Verbose mode still exposes useful paths, backend choice, and failure context.
- MCP outputs are not drowned out by GGUF initialization logs.

## Phase 7 - Re-Run The Field Test

Tasks:

1. Install the repaired binary locally with LLM search enabled:
   `llm-wiki install --non-interactive --enable-llm-search --profile balanced --confirm-model-downloads --accept-profile-licenses`.
2. For Claude, ensure the target project's `.mcp.json` connects production
   `llm-wiki` when the run requests production, then relaunch Claude so MCP
   startup sees the new config.
3. Launch Codex through `llm-wiki headroom -- wrap codex`; launch Claude through
   the equivalent Headroom wrap once its production MCP config is wired.
4. Run `wiki/checklists/headroom-launch-smoke.checklist.md` and its referenced
   pure MCP checklist against `keto-diet`. First classify the current Headroom
   version/path with full excluded-tool payloads. If the run uses Headroom 0.24.0
   on Codex Responses, full-payload read/search compression is expected; if it
   uses Headroom 0.32.0, source inspection says exclusions should be honored but
   live behavior is the authority.
5. Use compact search when full search payloads fail. Do not use read pagination
   in this posture; perform exact large wiki/raw reads outside Headroom.
6. Record the raw results as an eval if the transcript is complete enough to
   support future regression comparison.

Verification:

- The Headroom launch smoke has no ambiguous rows: it either passes full-payload
  integrity on a Headroom path that preserves outputs, passes the Option A
  supported surface with compact search plus loud read/full-search failed
  delivery evidence, or fails before Phase A with an explicit setup reason.
- `llm_wiki_status` reports the repaired version under test.

## Acceptance Criteria

1. Headroom-launched Codex and Claude runs fail fast before Phase A unless the
   requested production/test MCP instance, tool suffixes, and binary version are
   actually connected.
2. The plan and checklist distinguish the confirmed Headroom 0.24.0 Codex/
   OpenAI-Responses limitation from the current Headroom 0.32.0 source state,
   where Responses exclusions appear implemented and must be verified in a live
   run before pagination is treated as necessary.
3. Pure MCP sessions can read `wiki/index.md` and at least one raw source through
   `llm_wiki_read` with real content and no CCR placeholder.
4. Headroom-launched Codex first proves whether full excluded-tool payloads
   survive on the active version/path; when they do not, compact search must
   return inspectable hits and exact large reads are performed outside Headroom.
5. `llm_wiki_index` and forced index output are compact/structured enough for
   field tests to confirm indexed-file counts and rebuild status directly, or
   expose a compact status/readiness path for that confirmation.
6. `search-all` degrades missing project roots as per-project readiness/errors
   and continues searching healthy projects.
7. Normal index/search/doctor diagnostics are concise enough for live field
   tests; verbose detail remains available intentionally.
8. Focused Rust tests cover any code changes, and live Headroom/Codex plus
   Headroom/Claude field-test reruns record passing or explicitly
   preflight-blocked results.

## Closure

This plan closes only after live Headroom-launched Codex and Claude runs either
pass the instance/version preflight and complete the Headroom launch smoke plus
pure MCP field-test checklist on a Headroom path that preserves full outputs,
pass the Option A supported surface with compact search and loud read/full-search
failed-delivery evidence on a path that mutates full outputs, or fail before
Phase A with an explicit configuration mismatch that the operator can fix and
relaunch. Passing Option A runs do not need exact read reconstruction under
Headroom; exact large reads remain a pure-MCP/outside-Headroom operation. The
passthrough launcher remains implemented, but its Codex full-payload behavior is
version/provider dependent: Headroom 0.24.0 failed by source inspection and
field evidence, and Headroom 0.32.0 + managed `llm-wiki 0.2.13` fails full-read
and full-search preservation while compact search succeeds.
