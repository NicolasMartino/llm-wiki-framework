# Plan: Headroom/MCP Merge-Readiness Repairs

- Document Class: Plan
- Status: Blocked
- Note: The Headroom proxy carve-out portions of this plan are superseded by wiki/decisions/headroom-single-posture-mcp-first.decision.md (2026-06-30) and removed from the codebase in the no-legacy reduction; the proxy/router-bypass tasks and items have been stripped from this plan accordingly. The MCP-cutover, test-instance, and migration repairs here are unaffected and still stand.
- Date: 2026-06-23
- Updated: 2026-10-06
- Category: MCP cutover, test-instance safety, upgrade migration, release readiness
- Scope: Execute the repairs from `wiki/review/headroom-mcp-branch-merge-readiness.eval.md` before merging `impl/headroom-runtime-companion` as the no-legacy MCP cutover branch.
- Sources: `wiki/review/headroom-mcp-branch-merge-readiness.eval.md`; `wiki/plans/mcp-first-agent-surface.plan.md`; `wiki/evals/mcp-first-host-parity.eval.md`; `wiki/plans/harness-independent-wiki-read-tool.plan.md`; `wiki/decisions/test-instance-namespaced-binary.decision.md`; `wiki/evals/test-instance-live-session-proof.eval.md`; `wiki/roadmaps/cross-platform-release-e2e.roadmap.md`; `README.md`; `src/mcp/mod.rs`; `src/mcp_config.rs`; `src/install.rs`; `src/uninstall.rs`; `src/paths.rs`; `src/instance.rs`; `src/manifest/schema.rs`; `tests/mcp.rs`; `tests/mcp_install.rs`; `tests/install.rs`; `tests/post_install.rs`; `tests/status_doctor.rs`; `tests/identity_lint.rs`.

## Where This Stands (2026-10-06)

Done, with the evidence in "2026-06-23 Implementation Evidence" below, and on
master:

- Phase 0: the live test-instance run waited for Phase 1, and
  `just test-instance-live-session-proof` passed once Phase 1 landed.
- The code repairs of Phases 1 to 5 and 7.
- Phase 8's 2026-06-23 bookkeeping: the review, eval, plan, index and log pages
  record that evidence.

Still pending, and the reason this plan is Blocked rather than Completed:

- Phase 6, the clean no-skill host parity eval, and with it items 7 and 8 of the
  "Final Merge Gate". `wiki/evals/mcp-first-host-parity.eval.md` is still
  `Rejected`: query parity is proven on both harnesses, while the ingest or lint
  mutation, search fallback/readiness and failure workflows are unrecorded (its
  "Required Next Action", item 3).
- The rest of Phase 8, which needs Phase 6's evidence: moving that eval to
  `Accepted`, and closing `wiki/plans/harness-independent-wiki-read-tool.plan.md`
  once its read-tool evidence is recorded.

It waits on Phase 8, "Cross-Harness Parity Extension", of
`wiki/plans/mcp-first-agent-guidance.plan.md` (Active), which reruns that eval on
both harnesses from clean no-skill profiles with exactly these workflows. No
roadmap entry names that rerun yet. This plan completes when the eval moves to
`Accepted`, or when its remaining blocker is external and non-code.

Not pending here: the native Linux, macOS and Windows release proofs, deferred
to `wiki/roadmaps/cross-platform-release-e2e.roadmap.md` as item 9 of the
"Final Merge Gate" allows.

## Objective

Make the MCP branch safe to merge as the no-legacy MCP cutover by fixing blockers that would invalidate live testing or leave the old generated-skill path in place.

This plan is not a cross-platform release plan. Native Linux amd64, native macOS archive, and Windows host release-E2E proofs remain tracked by `wiki/roadmaps/cross-platform-release-e2e.roadmap.md`. They are not merge blockers for this branch unless the merge owner explicitly upgrades them into the final gate.

## Blocking Issues

| Issue | Evidence | Required outcome |
| --- | --- | --- |
| Test-instance MCP identity is not namespaced | `src/mcp_config.rs` hard-codes `llm-wiki`; `src/mcp/mod.rs` advertises production tool names | `llm-wiki-test` uses isolated MCP server and tool names, and uninstall preserves production MCP config |
| Production output equality can regress during namespacing refactor | `wiki/decisions/test-instance-namespaced-binary.decision.md` requires production outputs unchanged | production MCP config, tool names, and rendered guidance stay byte-identical after refactor |
| In-place upgrades can orphan old generated skills | install now produces no generated skills while old manifests may still own them | unchanged old generated skills are removed; user-edited or unmanaged orphans are preserved with explicit warnings |
| Legacy manifest compatibility is unproven | `src/manifest/schema.rs` rejects unsupported schema versions | migration test uses a real old-format generated-skill manifest, not only the current schema |
| Repo-local skills can shadow MCP dogfooding | `.claude/skills` / `.codex/skills` exist in repo | parity evals run with repo/user skills absent or moved aside by a restore-on-failure helper |
| Phase 5 host parity eval is rejected | `wiki/evals/mcp-first-host-parity.eval.md` | rerun clean no-skill host eval with query, read, search, and mutation evidence |
| MCP mutation parity is not an MCP-tool operation | `src/mcp/mod.rs` has read/search/index/register/status but no ingest/write/lint tool | eval explicitly proves host-native edits plus AGENTS/project guidance, not MCP mutation tools |
| README install docs are stale/incomplete | `README.md:25` generated-skill claim | docs state MCP config behavior, no daemon, no generated skills, and upgrade migration behavior |
| Current checkout search is unregistered | review finding | full-instance test covers both registered and unregistered search behavior |
| `identity_lint` has an omission blind spot | review finding | tests cover omitted MCP namespacing, not only hard-coded `_test` literals |
| `tests/post_install.rs` unused `Path` warning | `cargo check --tests` | warning removed |

## Phase 0 - Freeze Unsafe Live Testing

Purpose: prevent the live full-instance MCP dogfood from corrupting production MCP setup before isolation is fixed.

Steps:

1. Mark the `llm-wiki-test` live dogfood as blocked until Phase 1 passes.
2. Do not run the test-instance install/uninstall cycle against the real home MCP config until test-instance MCP server preservation is covered by tests.
3. Record in the repair review that Phase 1 is the first executable blocker before any live full-instance testing.

Verification:

The plan and review both state that the live test is blocked on Phase 1.

## Phase 1 - Namespace MCP Identity For Test Instance

Purpose: make the `test` build obey the accepted instance isolation contract for MCP server and tool identities while keeping production output byte-identical.

Implementation steps:

1. Add instance-derived MCP helpers in `src/instance.rs`.
   - `mcp_server_name()` returns `llm-wiki` for production and `llm-wiki-test` for test.
   - `mcp_tool_name(base)` returns production names unchanged and appends `_test` for test-instance tool names.
   - If specific helpers are clearer, add `mcp_read_tool_name()`, `mcp_search_tool_name()`, and related wrappers.
2. Refactor `src/mcp_config.rs`.
   - Replace hard-coded `SERVER_NAME: "llm-wiki"` with the active instance server name.
   - Merge Codex config under `mcp_servers.llm-wiki-test` for test instance.
   - Remove only the active instance's server entry during uninstall.
   - Preserve production `mcp_servers.llm-wiki` during `llm-wiki-test uninstall`.
   - Do not assert exact TOML quote style in tests; `llm-wiki-test` is a valid bare key, and serializer cosmetics are not the contract.
3. Refactor `src/mcp/mod.rs`.
   - Use instance-derived tool names in `tools/list`.
   - Dispatch active-instance tool names in `tools/call`.
   - Reject production tool aliases under the test instance unless a decision explicitly accepts aliases.
   - Render `initialize.instructions` and prompt text with active tool names where those names are visible to the user.
   - Keep MCP prompt and resource identifiers stable unless a collision is actually demonstrated: the host already disambiguates prompts/resources by MCP server name, so server-name namespacing prevents prompt/resource collisions.
4. Extend identity lint coverage.
   - Keep the hard-coded `_test` literal guard.
   - Add a test that fails when active test-instance MCP surfaces still emit production server/tool names.
   - Note explicitly that `identity_lint` passing is not sufficient evidence for this bug class because omission bugs never introduce `_test` literals.
5. Add a production output-equality gate.
   - Capture production MCP config rendering, MCP `tools/list` names, and startup guidance before and after the refactor.
   - Production must continue emitting exactly `llm-wiki` and `llm_wiki_*`.
   - Use the existing snapshot/golden-test pattern if present, or add an equivalent focused fixture if no current snapshot file covers this surface.

Tests:

1. Extend `tests/mcp_install.rs`.
   - Production install writes only the production MCP server entry.
   - With `LLM_WIKI_INSTANCE=test`, install adds only the test MCP server entry without changing the production server entry.
   - Test uninstall removes only the test server entry.
   - Production rendering is byte-identical after the refactor.
2. Extend `tests/mcp.rs`.
   - Production `tools/list` keeps production names.
   - Test-instance `tools/list` advertises suffixed names.
   - Test-instance `tools/call` accepts suffixed names.
   - Test-instance `tools/call llm_wiki_read` returns an unknown-tool error if aliases are unsupported.
3. Extend `tests/status_doctor.rs` if status/doctor output includes MCP startup names.
4. Extend `tools/test-instance-live-session-proof.sh`.
   - Snapshot production Codex MCP server entry before test install.
   - Confirm test install adds only test MCP state.
   - Confirm test uninstall restores MCP config to the pre-snapshot production state.

Verification commands:

```bash
rtk cargo test --test mcp
rtk cargo test --test mcp_install
rtk cargo test --test status_doctor
rtk cargo test --test identity_lint
rtk just test-instance-live-session-proof
```

Closure condition:

A `LLM_WIKI_INSTANCE=test` run can install and uninstall without changing production MCP config, test MCP tool names are namespaced, and production MCP output remains byte-identical unless a new accepted decision explicitly supersedes the test-instance contract.

## Phase 2 - Add Safe In-Place Upgrade Cleanup For Old Generated Skills

Purpose: prevent stale generated skills from surviving a new MCP-first install and continuing to route requests before MCP.

Implementation steps:

1. Add a failing upgrade test first.
   - Build the fixture in the actual old generated-skill manifest format/version used before the MCP cutover.
   - Include generated `.claude/skills/wiki-*` and `.codex/skills/wiki-*` entries with their old hashes.
   - Run the current install path.
   - Assert unchanged old manifest-owned skill files are gone.
   - Assert user-edited old files are preserved and produce explicit manual cleanup guidance.
2. Add legacy manifest read support only as far as migration requires.
   - If `Manifest::read` cannot deserialize the old manifest, add a scoped legacy parser or migration path before reconciliation.
   - The upgrade test must fail if the old manifest is skipped silently.
3. Add install-side reconciliation.
   - Before writing the new manifest, compute previous manifest skill entries not present in the new install file set.
   - Validate each current file hash against the old manifest.
   - Remove the file if it is manifest-owned and unchanged.
   - Remove now-empty managed skill directories.
   - If the hash changed, preserve the file and report explicit manual cleanup guidance.
4. Preserve user-authored and unmanaged files.
   - Never delete files that are not in the old manifest.
   - Never delete files whose current hash differs from the old manifest without an explicit force path and clear backup behavior.
   - If old skill directories exist but no readable manifest exists, warn only; do not remove them.
5. Ensure the new manifest does not lose cleanup accountability.
   - If a stale file is preserved because it was user-edited, record backup or diagnostic state so status/doctor can explain it.
   - Do not silently drop knowledge of an old generated skill that still exists and can route before MCP.
6. Add doctor/status warnings.
   - Warn when old generated-skill directories exist after install and are not manifest-owned.
   - State that such orphans can shadow MCP routing.

Verification commands:

```bash
rtk cargo test --test install
rtk cargo test --test post_install
rtk cargo test --test status_doctor
```

Closure condition:

In-place upgrade from a real old generated-skill manifest cannot leave unchanged generated skills active, and unmanaged or manifest-less orphans are warned about rather than removed.

## Phase 3 - Update MCP-First Install And Migration Documentation

Purpose: make public install guidance match runtime behavior.

README changes:

1. Replace the stale `README.md:25` claim that install "renders installed skills to call that managed path directly."
2. State that `llm-wiki install`:
   - installs the managed binary;
   - writes or merges Codex MCP config for the active instance;
   - uses host-managed stdio startup, not a background daemon;
   - no longer renders generated runtime skills.
3. Add upgrade migration notes.
   - Explain automatic cleanup of unchanged old generated skills after Phase 2.
   - Explain that user-edited generated skills are preserved with warnings.
   - Explain that manifest-less old skill directories are warned about, not deleted.
   - Provide manual cleanup guidance and the old-version uninstall option where appropriate.
4. Document operational MCP behavior.
   - `llm_wiki_read` can read scoped `wiki/` / `raw/` files through project discovery.
   - `llm_wiki_search` requires registered/indexed project state or returns readiness/failure metadata.
5. Update related docs if their install/runtime claims change.
   - `AGENTS.MD`
   - `templates/base/project_guidelines.md`
   - `wiki/references/headroom-context-compression.reference.md`
   - install/status/doctor output snapshots

Verification commands:

```bash
rtk cargo test --test install
rtk cargo test --test status_doctor
rtk cargo check --tests
```

Closure condition:

README and related docs no longer describe generated skills as the runtime install surface and accurately describe MCP-first upgrade behavior.

## Phase 4 - Define Repo-Local Skills Dogfood Posture

Purpose: prevent repo-local skills from masking MCP routing during no-legacy evaluation.

Decision steps:

1. Choose a dogfood posture.
   - Transitional: keep `.claude/skills` and `.codex/skills` in the repo as scaffolding, but move them aside for MCP parity evals.
   - Cutover: remove or archive project-local skills before claiming no-legacy parity.
2. If transitional, create a repeatable eval helper.
   - Move `.claude/skills` and `.codex/skills` aside.
   - Run the host eval.
   - Restore directories even if the eval fails.
   - Verify no `wiki-query` / `wiki-ingest` skill invocation appears in transcripts.
3. If cutover, update relevant decisions and index entries before deletion.
4. Update `wiki/evals/mcp-first-host-parity.eval.md` with the selected posture.

Closure condition:

Phase 5 parity eval cannot accidentally pass through project-local skills.

## Phase 5 - Register/Index Search State And Failure Cases

Purpose: cover the operational distinction between MCP read and MCP search.

Steps:

1. Add a fixture project for MCP search parity.
2. Cover registered state.
   - `llm_wiki_register`
   - `llm_wiki_index`
   - `llm_wiki_search`
   - read returned wiki pages with `llm_wiki_read`
3. Cover unregistered state.
   - server launched in a project that is not registered.
   - `llm_wiki_read` still works where project discovery allows.
   - `llm_wiki_search` returns clear error or readiness/failure payload.
4. Cover unavailable search cache/readiness.
   - stale or missing index.
   - semantic/hybrid readiness failure.
   - fallback metadata where lexical fallback is permitted.
5. Record the expected MCP result shape.
   - malformed params remain JSON-RPC errors.
   - command/read failures return MCP tool results with `isError: true`.

Verification commands:

```bash
rtk cargo test --test mcp
rtk cargo test --test search_commands
```

Closure condition:

Search behavior is proven for both success and failure/readiness states agents will encounter after MCP adoption.

## Phase 6 - Run Clean No-Skill Full-Instance Parity Eval

Purpose: close the rejected Phase 5 host parity gate with real host-client evidence.

Prerequisites:

1. Phase 1 complete.
2. Phase 2 complete or migration procedure documented and tested.
3. Phase 4 dogfood posture selected and helper available if transitional.
4. Claude Code auth/API blocker resolved or explicitly recorded as an external blocker.

Eval matrix:

1. Codex clean profile.
   - isolated `CODEX_HOME`.
   - no user-level skills.
   - repo skills moved aside or removed according to Phase 4.
   - MCP config points at the managed binary.
2. Claude Code clean profile.
   - isolated config/home where practical.
   - no project/user skills.
   - staged `.mcp.json` project MCP config installed.
3. Test-instance live dogfood.
   - use `llm-wiki-test`.
   - verify production MCP config is unchanged after uninstall.

Ordinary-language workflows:

1. Query a documented project fact.
2. Read `wiki/index.md` through MCP.
3. Search a registered project and inspect readiness metadata.
4. Trigger an unregistered-search failure and capture the tool result.
5. Perform a safe mutation workflow in a temp project.
   - The current MCP surface has no `ingest`, `write`, or `lint` mutation tool.
   - This step proves host-native edit tools plus `AGENTS.md` / project guidance can perform the mutation without generated skills.
   - Use a small lint or ingest fixture.
   - Verify a correct typed wiki page when ingest is tested.
   - Verify correct `wiki/index.md` and `wiki/log.md` diffs.
   - Verify cross-links and citations/provenance are preserved.
   - Verify the transcript contains no `wiki-query`, `wiki-ingest`, `wiki-lint`, or generated/user/project skill invocation.
6. Verify no generated/user/project skill handled any request.

Evidence to record:

1. host versions.
2. MCP config used.
3. transcript/JSONL paths.
4. tool-call counts by workflow.
5. wiki diffs for the mutation workflow.
6. pass/fail summary by workflow.
7. residual limitations.

Closure condition:

`wiki/evals/mcp-first-host-parity.eval.md` can move from `Rejected` to `Accepted` only when clean no-skill query/read/search evidence and host-native mutation evidence pass. If it remains `Rejected`, the remaining blocker must be smaller, specific, and external/non-code.

## Phase 7 - Clean Warnings And Run Full Verification

Purpose: eliminate cheap warning noise and rerun the automated gates touched by the repairs.

Steps:

1. Remove unused `Path` import in `tests/post_install.rs`.
2. Run focused suites:

```bash
rtk cargo check --tests
rtk cargo test --test mcp
rtk cargo test --test mcp_install
rtk cargo test --test install
rtk cargo test --test post_install
rtk cargo test --test status_doctor
rtk cargo test --test identity_lint
```

3. Run broad suites:

```bash
rtk cargo test
rtk cargo clippy
```

4. Run live proof after Phase 1:

```bash
rtk just test-instance-live-session-proof
```

Closure condition:

Focused and broad checks pass, with any ignored tests or external blockers recorded.

## Phase 8 - Wiki Closure And Promotion

Purpose: keep durable evidence aligned with the final implementation state.

Steps:

1. Update `wiki/review/headroom-mcp-branch-merge-readiness.eval.md`.
   - Mark each finding fixed, deferred, or still blocking.
2. Update `wiki/evals/mcp-first-host-parity.eval.md`.
   - Move to `Accepted` only if Phase 6 passes.
   - Keep `Rejected` if host auth or parity remains blocked, but narrow the blocker.
3. Update `wiki/plans/mcp-first-agent-surface.plan.md`.
   - Record final evidence.
   - Close only after its acceptance criteria are met.
4. Update `wiki/plans/harness-independent-wiki-read-tool.plan.md`.
   - Mark completed only after Phase 6 records read-tool evidence.
   - If Phase 6 cannot produce that evidence, keep the plan active and list it in the final gate.
5. Update `wiki/decisions/test-instance-namespaced-binary.decision.md`.
   - Even if the contract does not change, flip the deferred MCP namespacing clause to implemented once Phase 1 lands.
   - Add a superseding decision only if implementation intentionally changes the accepted contract.
6. Update `wiki/references/headroom-context-compression.reference.md` if final MCP tool names or server names change.
7. Update `wiki/index.md`.
8. Append `wiki/log.md`.

Closure condition:

No wiki page claims Phase 5 is closed until the eval is accepted; no read-tool proof is marked complete without Phase 6 read-tool evidence; no plan remains active solely because bookkeeping was skipped; and the index accurately summarizes the branch's merge-readiness state.

## 2026-06-23 Implementation Evidence

Implemented repairs:

- Phase 1: MCP identity is instance-derived in `src/instance.rs`,
  `src/mcp_config.rs`, and `src/mcp/mod.rs`; production output remains
  unsuffixed and test-instance output is suffixed.
- Phase 2: install accepts legacy manifest schema `1`, removes unchanged
  manifest-owned old generated skills, preserves edited old generated skills
  with warnings, and reports manifest-less old skill directories through
  install/status/doctor.
- Phase 3: README install guidance now documents MCP-first install behavior,
  no daemon, no generated runtime skills, and old-skill migration behavior.
- Phase 4: transitional no-repo-skill posture is implemented by
  `tools/with-repo-skills-disabled.sh`.
- Phase 5: MCP search/read behavior remains covered by `tests/mcp.rs` and
  `tests/search_commands.rs`.
- Phase 7: the unused `Path` warning in `tests/post_install.rs` is removed.
- Phase 8: review/eval/plan/index/log pages were updated with this evidence.

Verification:

- `cargo check --tests`
- `cargo test --test mcp`
- `LLM_WIKI_INSTANCE=test cargo test --test mcp`
- `cargo test --test mcp_install`
- `LLM_WIKI_INSTANCE=test cargo test --test mcp_install`
- `cargo test --test install`
- `cargo test --test post_install`
- `cargo test --test status_doctor`
- `cargo test --test identity_lint`
- `cargo test --test search_commands`
- `tools/with-repo-skills-disabled.sh rtk cargo test --test mcp`
- `just test-instance-live-session-proof`
- `cargo test`
- `cargo clippy`

Remaining gates: `wiki/evals/mcp-first-host-parity.eval.md` still controls
clean cross-harness ordinary-language parity acceptance, and
`wiki/roadmaps/cross-platform-release-e2e.roadmap.md` still controls native
Linux/macOS/Windows release-E2E proof scope.

## 2026-08-01 Production MCP Field-Test Review

The latest Codex and Claude production MCP field tests against the external
`/Users/nicolasmartino/Documents/keto_diet` project used the installed
production instance (`llm-wiki 0.2.15`, unsuffixed `llm_wiki_*` tools,
managed home `/Users/nicolasmartino/.llm_wiki`). They reconfirm the MCP surface
repairs that this plan depends on:

- status reports project registration, index freshness, indexed-file count, and
  lexical/semantic/hybrid readiness.
- register and force-index are idempotent against an already registered project.
- `llm_wiki_read` returns real wiki/raw Markdown with hashes and rejects
  traversal/outside-tree paths distinctly from missing files.
- lexical, auto, semantic, hybrid, class/status filters, and rerank diagnostics
  all return the expected parseable payloads.
- default thresholds are sufficient after indexing; calibration remains optional
  tuning, not a readiness gate.
- `search-all` preserves results from ready projects, reports missing/stale
  projects in per-project readiness/warnings, and honors include/exclude filters.

The first submitted report marked Step 18 as a `BUG` because it expected
cross-project hits to be grouped per project. Current accepted behavior is
different but coherent: per-project readiness belongs in `projects[]`, while
top-level `results[]` is a globally ranked flat result list with
`project_id`/`project_name` attribution on each hit. The field-test checklist
and host evals now record that as the contract. This is not a remaining
merge-readiness blocker unless the product later chooses to add a grouped
result view.

The Claude report adds one minor hardening follow-up: unknown
`search-all` filter keys (`include_projects` / `exclude_projects`) are silently
ignored instead of rejected or reported. The documented `include` / `exclude`
keys work and the run passed, so this is not a merge blocker for the current
MCP surface; it should be tracked as API ergonomics because silent ignore can
mask caller typos.

This review reduces the core MCP-surface risk for merge, but it does not close
the final gate by itself. Clean no-skill cross-harness ordinary-language parity
still belongs to `wiki/evals/mcp-first-host-parity.eval.md`, and native
release-E2E proof scope still belongs to
`wiki/roadmaps/cross-platform-release-e2e.roadmap.md`.

## Final Merge Gate

The branch is merge-ready only when all of these are true:

1. Test-instance MCP identity is namespaced or a superseding decision explicitly changes the contract.
2. Production MCP output remains byte-identical for server name, tool names, config rendering, and visible startup guidance.
3. `llm-wiki-test` install/uninstall does not mutate production MCP config after uninstall.
4. Old generated skills cannot silently survive an in-place MCP-first upgrade.
5. Old manifest format migration is tested; manifest-less orphan skills are warned about rather than removed.
6. README and install guidance match runtime behavior.
7. Clean no-skill Phase 5 parity eval is accepted, or the remaining blocker is explicitly external and non-code.
8. Mutation parity is evaluated as host-native edits plus project guidance, not as an MCP mutation tool.
9. Cross-platform release-E2E native Linux/macOS/Windows proofs are either explicitly deferred to `wiki/roadmaps/cross-platform-release-e2e.roadmap.md` or promoted into this gate by the merge owner.
10. Focused and broad automated checks pass.
11. `wiki/index.md` and `wiki/log.md` record the final state.
