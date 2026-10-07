# Plan: MCP-First Agent Guidance Hardening

- Document Class: Plan
- Status: Active
- Date: 2026-06-25
- Category: Agent runtime, MCP surface, project guidance, host parity
- Scope: First fix the MCP correctness bugs that make the framework-owned tools untrustworthy, then prove them with a reusable field-test checklist across new/existing/migration projects, and only then strengthen generated LLM Wiki project guidance so ordinary agents use the MCP tools and resources as the primary surface for wiki operations, including instance-aware tool names, explicit operation routing, search readiness handling, honest research workflow guidance, tests, and host parity evidence.
- Sources:
  - wiki/plans/mcp-first-agent-surface.plan.md
  - wiki/evals/mcp-first-host-parity.eval.md
  - wiki/plans/harness-independent-wiki-read-tool.plan.md
  - templates/base/agents.md
  - templates/base/project_guidelines.md
- Related:
  - wiki/proposals/mcp-first-surface.proposal.md
  - wiki/proposals/harness-independent-wiki-read-tool.proposal.md
  - wiki/references/headroom-context-compression.reference.md
  - wiki/decisions/test-instance-namespaced-binary.decision.md
  - wiki/checklists/mcp-field-test.checklist.md (produced by Phase 1)
  - wiki/decisions/headroom-single-posture-mcp-first.decision.md (the posture this
    routing serves: the MCP tools are the wiki/raw surface, so AGENTS.md must
    route agents to them)
  - wiki/plans/mcp-onboarding-init-register.plan.md (sibling gap: that plan
    *wires* the MCP server into init/register; this plan makes generated
    AGENTS.md *route* agents to the MCP tools — the keto-diet field test hit
    both. The 2026-06-29 template edits to templates/base/agents.md and
    templates/packs/qmd-rs-scale/agents.md are a down payment on this plan's
    AGENTS.md-routing phases.)
  - wiki/plans/init-rerun-pack-drift.plan.md (Completed: rerun refresh of an
    existing project's framework-owned AGENTS.md after these templates change)
  - src/search/metadata.rs
  - src/search/commands.rs
  - src/instance.rs

## Problem

The MCP tool surface exists, but ordinary agents are still under-directed.
Generated `AGENTS.md` remains the first file an agent reads, yet it does not
make the MCP-first route explicit enough. The stronger MCP guidance is buried in
`project_guidelines.md`, which many host/model combinations will not consult
deeply before choosing a tool path.

Observed symptoms:

- Agents often read `wiki/` with shell or filesystem tools instead of
  `llm_wiki_read`.
- Agents often skip `llm_wiki_search` unless a skill or prompt specifically
  tells them to run search.
- Legacy user/project skills can intercept ordinary-language requests before
  the MCP route is chosen.
- Fresh projects may not have a search index yet, causing search fallback to
  look like search is optional instead of a readiness state to report.
- The test instance exposes suffixed MCP names such as `llm_wiki_read_test`, but
  generated guidance is not currently shaped around instance-aware names.
- Research is a workflow, not a deterministic MCP tool today, so agents need
  clear instructions that research means source collection into `raw/research/`
  followed by ingest, not an invisible `llm_wiki_research` tool.

More fundamentally, a live MCP test pass on a fresh external project (the
`keto-diet` testbed, run against the `llm-wiki-test` instance) showed the tool
surface itself returns wrong or confusing results. Nudging agents toward the MCP
is premature while these hold:

- `llm_wiki_read` returns Headroom CCR placeholders such as
  `<<ccr:ba84c5387ca6,string,1.8KB>>` instead of Markdown, even though the
  underlying CLI read returns the real body. The read tool is supposed to be
  excluded from Headroom compression but the exclusion is not reaching the
  active instance-qualified tool name.
- Search hits carry `class: null` and `status: null` even when the wiki pages
  have valid `Document Class:` and `Status:` header lines, so class/status
  filters exclude every match.
- `semantic` and `auto` modes hard-fail with `thresholds_unconfigured` on a
  fresh project, and `hybrid`'s lexical fallback can still return zero hits for
  natural-language queries, so search readiness reads as "broken" rather than
  "needs calibration".
- `rerank: true` is accepted but `reranker_model` stays null with no warning, so
  callers cannot tell whether reranking happened.

These correctness gaps are a hard prerequisite: the guidance-push phases below
must not begin until Phase 0 lands and Phase 1 proves the fixes in the field.

## Correction: Phase 0 Diagnoses Superseded

Status as of 2026-06-27: the live keto-diet differential in
`wiki/plans/hybrid-default-and-mcp-surface-repair.plan.md` supersedes this
plan's original Phase 0 diagnoses for Issues 0.1, 0.2, and 0.4.

- Issue 0.1 was not a stale installed-profile or instance-name rendering bug.
- Issue 0.2 was not proven to be a metadata parser defect by the field test.
  Direct CLI lexical search returned hits with non-null class/status; the null
  fields observed earlier were not code bugs (the CLI output is authoritative).
- Issue 0.4 was not a serializer omission in the CLI path. Direct CLI output
  included `rerank_reason`; the missing field observed earlier was not a code
  bug (the CLI output is authoritative).

The guidance-push work in this plan can continue only after the operative repair
plan handles default hybrid readiness, `search-all`, MCP status, and
raw/read-error surfaces.

## Deliverable

Freshly initialized LLM Wiki projects carry a top-level `AGENTS.md` that acts as
the MCP router for the project. An ordinary agent should learn, from the first
guidance file it sees, how to use the configured MCP server for query, read,
search, indexing, status, ingest support, lint support, and research handoff.

The generated guidance must render the active instance's actual names:

- production: `llm_wiki_read`, `llm_wiki_search`, `llm_wiki_search_all`,
  `llm_wiki_status`, `llm_wiki_register`, `llm_wiki_index`
- test instance: `llm_wiki_read_test`, `llm_wiki_search_test`,
  `llm_wiki_search_all_test`, `llm_wiki_status_test`,
  `llm_wiki_register_test`, `llm_wiki_index_test`

The plan closes when generated guidance, MCP initialize instructions/resources,
tests, and host parity evidence agree on the same operation routing.

## Non-Goals

- Do not weaken the `headroom_read` ban.
- Do not make MCP prompts load-bearing; prompts remain optional convenience
  because host support is uneven.
- Do not reintroduce generated skill projection as a permanent fallback.
- Do not turn `llm_wiki_read` into a general file reader outside `wiki/` and
  `raw/`.
- Do not claim research is deterministic MCP automation until a real
  `llm_wiki_research_*` tool is designed and implemented.
- Do not change the document model, typed document roles, or agent-owned wiki
  contract.
- Do not weaken semantic/hybrid correctness to make `thresholds_unconfigured`
  disappear; the Phase 0 fix is readiness/UX (graceful `auto` degradation plus an
  actionable message), not lowering or skipping calibrated thresholds.
- Do not begin the guidance-push phases (instance-aware rendering onward) until
  the Phase 0 bug-fix gate lands and the Phase 1 field test proves the fixes.

## Phase 0 - MCP Correctness Prerequisites (Bug-Fix Gate)

Fix the correctness bugs surfaced by the live `keto-diet` test pass before any
guidance work. Each fix ships with a focused Rust test so the symptom cannot
silently return. This phase is a gate: later phases do not start until all four
fixes land and Phase 1 proves them in the field.

### Issue 0.1 - `llm_wiki_read` returns Headroom CCR placeholders

- Correction: superseded (see "Correction: Phase 0 Diagnoses Superseded" above).
  This was not a code bug — the CLI output is authoritative, and the CCR
  placeholders came from an external compression layer, not from llm-wiki.
- Current invariant: `llm_wiki_read[_test]` returns real Markdown through the
  framework-owned MCP route. If a session runs Headroom, provenance still comes
  from routing wiki/raw access through the `llm_wiki_*` tools with
  `HEADROOM_MCP_READ=off`; no installed profile, proxy carve-out, or
  exclude-list asset is part of this plan after the 2026-07-07 no-legacy
  reduction.
- No live fix remains here. The retired profile/exclude remediation text was
  removed so this active plan no longer points at deleted assets or tests.

### Issue 0.2 - Search results carry `class: null` / `status: null`

- Symptom: every hit reports null class/status even though pages have
  `Document Class:` and `Status:` lines; class/status filters then exclude all
  matches.
- Diagnosis: `parse_wiki_metadata` (`src/search/metadata.rs`) only accepts a
  contiguous `- Key: Value` block immediately after the H1. Indexing and
  serialization (`src/search/qmd_rs.rs`, `src/search/semantic.rs`,
  `src/search/commands.rs`) are correct; externally authored notes likely used a
  looser header format.
- Fix: loosen the parser to accept common real-world variants (bare
  `Document Class: X` without the bullet, `**Document Class:** X`, and a leading
  `---` YAML frontmatter block) while keeping the canonical bullet format as the
  emitted standard. Keep the metadata block anchored near the H1 to avoid
  matching prose.
- Touchpoints: `src/search/metadata.rs` and its tests.
- Verification: pages using each accepted variant index with non-null
  class/status and are selected by class/status filters; the canonical format and
  the existing "ignore later bullets" behavior still parse correctly.

### Issue 0.3 - `semantic`/`auto` hard-fail with `thresholds_unconfigured`

- Symptom: `semantic` and `auto` error with `thresholds_unconfigured` on a fresh
  project; `hybrid` falls back to lexical but natural-language queries may still
  return zero hits.
- Correction: this issue is superseded by
  `wiki/plans/hybrid-default-and-mcp-surface-repair.plan.md`. Fresh indexed
  projects now synthesize default semantic/hybrid thresholds, so calibration is
  optional quality tuning rather than a readiness prerequisite.
- Fix: after `index`, `semantic`, `hybrid`, and `auto` should run with
  `thresholds_source: default` unless a recorded calibration overrides it.
  Lexical fallback remains a readiness path for genuinely unavailable states
  such as missing index data, disabled LLM search, missing model artifacts, or
  unaccepted licenses.
- Touchpoints: `src/search/commands.rs`, `src/search_models.rs`, guidance
  templates.
- Verification: `mode:auto`, `mode:semantic`, and `mode:hybrid` on a freshly
  indexed project return results tagged `thresholds_source: default`; explicit
  readiness failures remain clear for non-indexed or unavailable search states.

### Issue 0.4 - `rerank: true` silently no-ops

- Symptom: rerank is accepted but `reranker_model` stays null with no warning, so
  callers cannot tell whether reranking happened.
- Fix: when rerank is requested but no reranker model is configured/available,
  emit explicit flags in the result/readiness JSON (e.g. `rerank_requested:true`,
  `rerank_applied:false`, `reason`); when a model is present, report
  `rerank_applied:true` plus the model name.
- Touchpoints: `src/search/commands.rs` (rerank path), search result/readiness
  serialization.
- Verification: rerank requested without a model returns
  `rerank_applied:false` + reason; with a model returns `rerank_applied:true` and
  the model name.

Phase gate verification:

- All four fixes covered by focused Rust tests (`cargo test`).
- A no-skill live re-run of the original `keto-diet`-style pass shows read
  returns real content, class/status populated, `auto` returns lexical results
  with explicit readiness, and rerank reports applied/not-applied.

## Phase 1 - MCP Field-Test Checklist and Multi-Project Proof

Produce a reusable, paste-ready agent test protocol and run it for real across
the three project postures the MCP must serve, so we test in depth and in the
field before pushing agents toward the surface.

Tasks:

1. Author `wiki/checklists/mcp-field-test.checklist.md` (Document Class:
   Checklist, Status: Active): a paste-ready prompt an agent runs from inside a
   target project against the installed MCP. It must:
   - Capture raw tool output verbatim so CCR placeholders, null metadata, and
     readiness states stay visible (no paraphrasing).
   - Include the setup steps the prior pass skipped: `status` -> `register` ->
     `index` -> calibrate/record semantic thresholds, before search-mode testing.
   - Exercise every tool: `status`; `register`; `index` (incl. `--force`);
     `read` (wiki + raw, absolute + relative paths, traversal rejection,
     missing-file error); `search` (lexical/semantic/hybrid/auto, class+status
     filters, rerank true/false, `allow_lexical_fallback`); `search-all`
     (include/exclude).
   - For each step record action, raw output, expected vs failure signal, and
     explicitly mark "expected in a fresh project" vs "bug".
   - End with a structured pass/fail report keyed to the Phase 0 issues so
     regressions are obvious.
2. Run the checklist across three postures and record each transcript + verdict:
   - New project (fresh `init`).
   - Existing project not yet migrated (register + index an existing wiki).
   - Migration candidate (a real external project we want to move onto the MCP,
     e.g. `keto-diet`).
3. File any reproductions back as new issues before the guidance-push phases
   begin.

Verification:

- `wiki/checklists/mcp-field-test.checklist.md` exists and parses with non-null
  class/status (a self-test of the Phase 0.2 parser fix).
- At least one accepted run per posture (new / existing / migration) shows read,
  class/status, `auto` readiness, and rerank reporting all behaving per Phase 0.
- Any reproductions are logged as follow-up issues before guidance work starts.

## Phase 2 - Instance-Aware Guidance Rendering

Thread instance-derived names into init guidance rendering so templates do not
hardcode production MCP tool names.

Tasks:

1. Extend the init template context with the active binary name and MCP tool
   names from the central instance derivation API.
2. Render both production and `LLM_WIKI_INSTANCE=test` guidance with the correct
   names.
3. Keep production output stable except for the intentional new guidance block.
4. Ensure test-instance output contains only suffixed MCP names where tool names
   are shown.

Touchpoints:

- `src/init/template.rs`
- `src/init/compose.rs`
- `src/instance.rs`
- `templates/base/agents.md`
- `templates/base/project_guidelines.md`
- init snapshot tests

Verification:

- Production init snapshots contain production MCP names.
- Test-instance init snapshots contain `_test` MCP names and no unsuffixed
  production tool names in the MCP routing block.
- Existing identity lint still prevents hardcoded test names outside the
  derivation path.

## Phase 3 - Promote MCP Routing Into `AGENTS.md`

Make `AGENTS.md` explicit enough to drive ordinary-language routing without a
skill or prompt.

Tasks:

1. Add a top-level `MCP-First Workflow` section before `How To Orient`.
2. State that when the LLM Wiki MCP server is available, wiki/raw reads and
   searches should go through framework-owned MCP tools/resources before
   shell-family reads.
3. Add a compact operation-to-tool map:
   - `status`: check project/install/search readiness.
   - `read`: read full `wiki/` or `raw/` files.
   - `search`: run per-project retrieval after reading `wiki/index.md`.
   - `search-all`: run cross-project retrieval only when cross-project context
     is needed.
   - `register`: register the current project when needed.
   - `index`: refresh search after ingest/lint mutations.
4. Explain fallback behavior: if MCP is unavailable, report that and use
   index-based navigation rather than silently treating missing search as a
   result.
5. Preserve the existing instruction that `wiki/index.md` is the entry point.

Verification:

- Snapshot tests assert generated `AGENTS.md` contains the MCP-first block.
- The block includes the active instance's tool names.
- The block does not tell agents to use `headroom_read`.

## Phase 4 - Strengthen Operation Workflows

Update the core operation sections so agents know when to call MCP and when to
edit files directly.

### Query

Required flow:

1. Check status/readiness.
2. Read `wiki/index.md` via MCP read or MCP resource.
3. Run MCP search for every project question when the project is registered.
4. Inspect mode, readiness, fallback, zero-result, and result metadata.
5. Read returned wiki pages via MCP read.
6. Cite wiki paths directly and flag gaps or contradictions.

### Ingest

Required flow:

1. Read `wiki/index.md`.
2. Read curated `raw/` source material through MCP read when possible.
3. Write or update wiki pages using normal file edits.
4. Update `wiki/index.md` and append `wiki/log.md`.
5. Refresh the index with MCP index or the equivalent CLI command.
6. Report search readiness after indexing.

### Lint

Required flow:

1. Use MCP status/read/search to orient and identify candidate pages.
2. Read affected wiki pages through MCP read.
3. Patch wiki files directly.
4. Update `wiki/index.md` and append `wiki/log.md`.
5. Re-index when wiki content changed.

### Research

Required flow:

1. State that research is source acquisition and preparation, not a current
   deterministic MCP tool.
2. Collect research bundles under `raw/research/`.
3. Record provenance, source URLs/files, and a summary manifest.
4. Ingest the curated bundle into `wiki/`.
5. Consider a future `llm_wiki_research_prepare` or
   `llm_wiki_research_ingest` MCP tool only after the workflow is stable.

Verification:

- Generated `AGENTS.md` and `project_guidelines.md` agree on these flows.
- MCP prompts, if exposed by a host, point back to the same flows.
- Operation spec resources remain consistent with the generated guidance.

## Phase 5 - MCP Server Guidance Alignment

Keep the server's model-visible instructions, resources, and prompts aligned
with `AGENTS.md`.

Tasks:

1. Update MCP `initialize` instructions to name the same read/search/status
   sequence as `AGENTS.md`.
2. Ensure MCP prompts for query, ingest, lint, research, and init remain
   advisory and route back to project guidance plus deterministic tools.
3. Ensure MCP resources expose `AGENTS.md`, `project_guidelines.md`,
   `wiki/index.md`, `wiki/log.md`, and operation specs when present.
4. Add tests for the generated instructions and prompt text using
   instance-aware tool names.

Verification:

- `cargo test --test mcp` asserts initialize instructions contain the active
  instance's read/search/status names.
- `resources/list` still exposes canonical guidance resources.
- Prompt tests do not assume host prompt support.

## Phase 6 - Legacy Skill Interference Handling

Make the product honest about legacy skills shadowing MCP-first routing.

Tasks:

1. Add status/doctor guidance that legacy generated skill directories may
   intercept ordinary-language wiki requests before MCP.
2. Point users toward the existing cleanup/reinstall path instead of silently
   tolerating shadowing.
3. Keep user-authored legacy skill directories preserved unless the user
   explicitly asks to remove them.
4. Update docs so a clean no-skill profile is the expected parity test posture.

Verification:

- Existing legacy-skill warning tests remain passing.
- New status/doctor output, if added, is covered by focused tests.
- Host parity eval distinguishes skills-present behavior from no-skill MCP
  behavior.

## Phase 7 - Fresh Project Manual Proof

Use the current `llm-wiki-test` path to prove the improved guidance in a real
fresh project.

Procedure:

1. Build the test instance.
2. Install `llm-wiki-test`.
3. Initialize a fresh project.
4. Verify generated `AGENTS.md` includes `_test` MCP names.
5. Register and index the project.
6. Run ordinary-language query/lint/ingest prompts in a clean no-skill host
   profile.
7. Record which tools were called and whether search/read flowed through MCP.

Verification:

- Fresh project `AGENTS.md` names `llm_wiki_read_test` and
  `llm_wiki_search_test`.
- `llm-wiki-test doctor` reports registered/indexed status after setup.
- The host transcript shows MCP read/search calls for query.

## Phase 8 - Cross-Harness Parity Extension

Extend the existing rejected parity eval with the workflow breadth that remains
open.

Required scenarios on both Claude Code and Codex, from clean no-skill profiles:

1. Query with citations.
2. Search readiness/fallback when an index is missing or stale.
3. Unregistered project failure behavior.
4. Lint mutation that produces `wiki/index.md` and `wiki/log.md` diffs.
5. Ingest mutation from a small curated `raw/` source with index/log
   bookkeeping.
6. Research handoff language showing source collection into `raw/research/`
   rather than pretending an MCP research tool exists.

Verification:

- `wiki/evals/mcp-first-host-parity.eval.md` records host versions, prompts,
  tool-call transcripts, diffs, and verdict.
- Phase 5 of `wiki/plans/mcp-first-agent-surface.plan.md` can move toward
  acceptance only if these ordinary-language cases pass.

## Acceptance Criteria

Gate (must pass before any guidance-push criterion below is evaluated):

1. `llm_wiki_read` (production and `_test`) returns real Markdown. (Superseded
   framing: this was originally gated on retired compression-profile behavior.
   Per the "Correction: Phase 0 Diagnoses Superseded" note above and the
   2026-07-07 no-legacy reduction, the proxy, installed profile, and exclude set
   were all retired; the CLI read is authoritative and `llm_wiki_read` returns
   the real body through MCP routing with `HEADROOM_MCP_READ=off`. No profile or
   exclude-set assertion is required.)
2. Search results populate `class`/`status` for the canonical bullet format and
   the loosened real-world variants, and class/status filters select them.
3. `mode:auto`, `mode:semantic`, and `mode:hybrid` on a freshly indexed project
   return results with `thresholds_source: default`; lexical fallback is reserved
   for genuine readiness failures such as missing indexes or unavailable search
   artifacts.
4. `rerank` reports `rerank_applied` (true/false) plus the model name or reason.
5. `wiki/checklists/mcp-field-test.checklist.md` exists, and at least one accepted
   field-test run per posture (new / existing / migration) shows the four fixes
   holding.

Guidance push:

6. Generated `AGENTS.md` has a top-level MCP-first routing section.
7. Generated `AGENTS.md` uses instance-aware MCP names in production and test
   builds.
8. Query guidance requires MCP search after index orientation and requires
   explicit readiness/fallback handling.
9. Ingest and lint guidance preserve direct file edits for mutations but require
   MCP read/search/status for orientation where available.
10. Research guidance is honest: source acquisition into `raw/research/`, then
    ingest; no implied deterministic research MCP tool.
11. MCP initialize instructions, resources, and optional prompts align with
    generated project guidance.
12. Status/doctor/docs make legacy skill shadowing visible.
13. Snapshot and MCP tests cover the guidance and instance-name behavior.
14. A fresh `llm-wiki-test` project proof records the expected generated guidance
    and MCP tool availability.
15. The cross-harness parity eval is extended with mutation, fallback, failure,
    and research-handoff coverage.

## Closure

This plan can close when the Phase 0 bug-fix gate has landed (read returns real
content, class/status populate, `auto` degrades with explicit readiness, rerank
reports its status), Phase 1 has at least one accepted field-test run per posture,
the guidance hardening is implemented, tests prove the rendered output and MCP
instruction surface, and the host parity eval has at least one accepted clean
no-skill run showing ordinary-language query/read/search behavior through MCP with
the improved `AGENTS.md`.

Full closure of the broader MCP-first no-legacy cutover still belongs to
`wiki/plans/mcp-first-agent-surface.plan.md`.
