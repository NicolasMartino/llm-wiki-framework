# Eval: Headroom/MCP Branch Merge-Readiness Review

Document Class: Eval
Status: Updated
Date: 2026-06-23
Updated: 2026-08-01
Category: Implementation review, merge readiness
Scope: Branch-level review of `impl/headroom-runtime-companion` against `master`, covering MCP-first cutover, legacy skill-projection deletion, test-instance binary, GGUF runtime, release E2E, runtime/upgrade impact of adopting MCP, and migration handling.
Sources: `git diff master...HEAD`; `src/mcp/mod.rs`; `src/mcp_config.rs`; `src/wiki_read/mod.rs`; `src/install.rs`; `src/uninstall.rs`; `src/paths.rs`; `src/instance.rs`; `tests/mcp.rs`; `tests/mcp_install.rs`; `tests/install.rs`; `tests/identity_lint.rs`; `README.md`; `wiki/evals/mcp-first-host-parity.eval.md`; `wiki/plans/mcp-first-agent-surface.plan.md`; `wiki/plans/harness-independent-wiki-read-tool.plan.md`; `wiki/proposals/mcp-first-surface.proposal.md`; `wiki/decisions/test-instance-namespaced-binary.decision.md`; `wiki/evals/test-instance-live-session-proof.eval.md`.
Related: `wiki/plans/mcp-first-agent-surface.plan.md`; `wiki/evals/mcp-first-host-parity.eval.md`; `wiki/plans/test-instance-namespaced-binary.plan.md`; `wiki/evals/test-instance-live-session-proof.eval.md`; `wiki/references/headroom-context-compression.reference.md`.

## 2026-06-23 Repair Update

The code-level merge blockers identified by this review have been repaired on
the Headroom/MCP branch:

- Test-instance MCP server and tool identities are derived from
  `src/instance.rs`; production remains `llm-wiki` / `llm_wiki_*`, while the
  test instance advertises `llm-wiki-test` and suffixed MCP tool names.
- The `llm-wiki-test` install/uninstall live proof now installs only the test
  MCP server entry, uninstalls explicitly, and requires the real Codex config to
  match its pre-run copy.
- In-place MCP-first upgrade cleanup now removes unchanged manifest-owned old
  generated skills, preserves user-edited generated skills with manual cleanup
  warnings, accepts the legacy manifest schema, and warns on manifest-less old
  skill directories.
- README install guidance now documents MCP config behavior, host-managed stdio
  startup, no background daemon, no generated runtime skills, and old-skill
  migration warnings.
- Repo-local skill dogfooding uses the transitional posture: keep repo-local
  skills, but run no-skill parity checks through
  `tools/with-repo-skills-disabled.sh`.
- The warning in `tests/post_install.rs` is removed.

Verification completed:

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

Residual merge gates remain outside this code-repair slice: clean cross-harness
ordinary-language host parity still depends on
`wiki/evals/mcp-first-host-parity.eval.md`, and native Linux/macOS/Windows
release-E2E proofs remain governed by
`wiki/roadmaps/cross-platform-release-e2e.roadmap.md`.

## 2026-08-01 Production MCP Field-Test Review Update

The latest user-supplied Codex and Claude field tests ran against the
production unsuffixed MCP instance (`llm-wiki 0.2.15`, managed home
`/Users/nicolasmartino/.llm_wiki`) in the external
`/Users/nicolasmartino/Documents/keto_diet` workspace. Together they are clean
evidence for the pure MCP surface in an existing ready-index project posture:

- `llm_wiki_read` returned real wiki/raw Markdown with hashes and preserved
  scoped path safety.
- lexical, auto, semantic, and hybrid search returned populated result arrays.
- semantic/hybrid used `thresholds_source:"default"` without calibration.
- class/status filters and explicit rerank diagnostics behaved correctly.
- `search-all` reported per-project readiness/warnings, preserved ready-project
  hits, and honored include/exclude filters.

The only submitted blocking `BUG` row was Step 18's expectation that
`search-all` hits be grouped per project. That expectation is now corrected in
the field-test checklist: the accepted response shape is per-project readiness
in `projects[]` plus a top-level globally ranked flat `results[]` list carrying
`project_id`/`project_name` on each hit. The row is therefore a checklist/API
contract clarification, not a product blocker.

The Claude report adds one minor follow-up: `llm_wiki_search_all` silently
ignored obsolete `include_projects` / `exclude_projects` keys before the run
used the documented `include` / `exclude` keys successfully. This should be
tracked as API hardening or validation work, but it does not reopen the core
MCP read/search/status/register/index correctness gate.

Merge implication: the production MCP read/search/status/index/register surface
is no longer the risk this review is waiting on. The remaining gates are the
same as the repair update above: accepted clean no-skill cross-harness
ordinary-language parity or a narrowed external blocker, and an explicit
decision on native release-E2E proof scope.

## Summary

This branch is not a single MCP feature. It combines at least six workstreams:

1. MCP-first agent surface: Rust-native stdio server, deterministic tools, resources, advisory prompts, Codex MCP config, and staged Claude MCP config.
2. Runtime generated skill-projection removal from install/build/runtime paths.
3. Coexisting `llm-wiki-test` instance support.
4. GGUF CPU runtime portability and release-E2E lanes.
5. Release E2E Linux/Docker/native-lane work.
6. Wiki evidence/bookkeeping updates for the above.

The core MCP implementation and config lifecycle are broadly sound, but the branch is not merge-ready on its own stated acceptance criteria.

## Findings

> **Superseded snapshot.** These are the findings as originally recorded on
> 2026-06-23. The code-level blockers below were subsequently repaired — see the
> "2026-06-23 Repair Update" section above for what was fixed and the verification
> run. Read the items here as the original review, not as currently-open blockers;
> the only residual gates are the cross-harness host-parity and release-E2E items
> called out in that update.

### Blocker: MCP-first Phase 5 is still not accepted

`wiki/evals/mcp-first-host-parity.eval.md` remains `Status: Rejected`. The eval records clean no-skill query routing through MCP on both harnesses, but explicitly says Phase 5 still lacks ordinary-language proof for mutation workflows, `wiki/index.md` / `wiki/log.md` bookkeeping diffs, search fallback/readiness behavior, and failure behavior when a project is unregistered or search cache is unavailable.

`wiki/plans/mcp-first-agent-surface.plan.md` is still `Status: Active` and requires accepted cross-harness ordinary-language parity before closure. `wiki/plans/harness-independent-wiki-read-tool.plan.md` is also still `Status: Active`, even though the implementation evidence says the read-tool foundation landed. That is either stale bookkeeping or an unclosed sub-plan; either way it should be resolved before merge.

The proposed full-instance test is therefore not optional polish. It is the missing Phase 5 evidence.

Required proof before merge:

- clean no-skill profile for both harnesses
- repository skills moved aside or otherwise proven not to route
- ordinary-language query, read, search, mutation/lint or ingest bookkeeping, fallback/readiness, and unregistered-project failure cases
- transcript/evidence showing whether routing used MCP tools, project instructions, resources, prompts, native filesystem reads, or skills

### Blocker: test-instance MCP identity is not namespaced

The test-instance decision says MCP/tool names are part of the integration surface and must use `_test` names for the test instance, e.g. `llm_wiki_search_test` and `llm_wiki_search_all_test` (`wiki/decisions/test-instance-namespaced-binary.decision.md`).

The current MCP implementation does not apply that namespace:

- `src/mcp_config.rs:8` hard-codes `SERVER_NAME: "llm-wiki"`.
- `src/mcp_config.rs:16-27` renders Claude config with the `"llm-wiki"` server key.
- `src/mcp_config.rs:30-65` merges Codex config under `[mcp_servers."llm-wiki"]`.
- `src/paths.rs:66-67` writes Codex config to the shared `~/.codex/config.toml`, not an instance-specific config path.
- `src/mcp/mod.rs` advertises and dispatches hard-coded production tool names (`llm_wiki_read`, `llm_wiki_search`, `llm_wiki_search_all`, `llm_wiki_index`, `llm_wiki_register`, `llm_wiki_status`).

The decision's low-stakes caveat was that MCP/tool names were reserved but not yet built. That caveat has expired on this branch: the MCP tools and install configuration are now implemented, so the namespace derivation needed to be applied here.

Impact: this blocks the proposed live full-instance test, not only final merge. A `LLM_WIKI_INSTANCE=test` install can overwrite the production Codex MCP server entry with a command pointing at `~/.llm_wiki-test/bin/llm-wiki-test`, and test uninstall can remove the shared `"llm-wiki"` MCP entry. That violates the test instance's stated purpose: coexist with production and ensure uninstall touches only the test instance. The staged Claude config is in the test managed home, but its server key is still `"llm-wiki"`, so copying it into a project `.mcp.json` would collide with the production server name. The actual test MCP tools also remain production-named, while the test-instance derivation layer reserves `_test` tool names.

The accepted test-instance live-session proof does not appear to cover this MCP identity surface; `tools/test-instance-live-session-proof.sh` and `wiki/evals/test-instance-live-session-proof.eval.md` do not exercise MCP config names, Codex MCP config preservation, or test-instance MCP tool names.

Required fix:

- derive MCP server names through the instance identity layer, likely `llm-wiki` / `llm-wiki-test`
- derive MCP tool names or explicitly amend the test-instance decision if tool names must remain stable
- add `LLM_WIKI_INSTANCE=test` coverage for install/status/uninstall MCP config preservation
- add an MCP session test proving the test binary advertises the intended test-instance tool names, or document and decide why it deliberately does not

### High: upgrade path orphans previously generated skills

Runtime install no longer renders generated skills:

- `render_install_files` returns `Vec::new()` (`src/install.rs` near the current render-install-file function).
- `run_install` builds a new manifest from the current `skill_entries`, which are now empty (`src/install.rs:160-178`).

That proves new installs do not materialize skill trees. It does not migrate old installs that already have generated `.claude/skills/wiki-*` or `.codex/skills/wiki-*` files.

`install_files` writes the new file set; it does not reconcile files the previous manifest owned but the new manifest omits. `uninstall` removes manifest-owned skill entries, but only if the old manifest is still present and uninstall runs before a new install overwrites the manifest with an empty skills list.

Impact: users upgrading in place can keep stale generated skills on disk. Those stale skills can continue to win ordinary-language routing before MCP, undermining the no-legacy cutover.

Required fix or documented migration:

- install-side reconciliation that removes old manifest-owned skill files omitted by the new file set, or
- explicit upgrade procedure: run old `llm-wiki uninstall` before installing this branch, plus a verification command that no generated skills remain.

### Medium: committed repo-local skills still shadow MCP dogfooding

This checkout still contains project-local skills:

- `.claude/skills/wiki-{query,init,ingest,lint,research}/`
- `.codex/skills/wiki*/`

The host-parity eval already recorded that a skill-capable profile invoked `wiki-query` before MCP. That means this development repo is not representative of a no-legacy MCP install unless those project-local skills are moved aside during evals.

This may be acceptable as transitional dogfood scaffolding, but it must be an explicit decision. Otherwise ordinary-language testing in this repo can accidentally validate the old skill path while the branch claims MCP-first behavior.

### Medium: current checkout is not registered for search

`llm-wiki search --mode auto --format json "MCP merge readiness"` failed in this checkout with:

```text
Error: current project /Users/nicolasmartino/Documents/local_llm_wiki/llm_wiki_framework_headroom_impl is not registered; run `llm-wiki register /Users/nicolasmartino/Documents/local_llm_wiki/llm_wiki_framework_headroom_impl`
```

That is not a code bug by itself. It is an operational impact of adopting MCP/search from now on: `llm_wiki_read` can work by project discovery, but `llm_wiki_search` follows registered-project search semantics and needs a register/index step or clear fallback behavior. The full-instance test should include both registered and unregistered states.

### Medium: README has stale install text and lacks MCP-first install documentation

`README.md:25` still says `llm-wiki install` "renders installed skills to call that managed path directly." That is now factually false because runtime install no longer renders generated skills.

The README also does not document the new MCP realities:

- `llm-wiki install` writes/merges Codex MCP configuration.
- Claude Code config is staged and must be manually copied/registered per project.
- MCP startup is host-managed stdio; no daemon is installed.
- generated skills are no longer runtime install output, despite the current README wording.
- upgrade users may need stale generated skill cleanup until install-side migration exists.

This is documentation debt rather than an implementation blocker, but it directly affects safe adoption.

### Low: MCP forward-compat and project-discovery assumptions need documentation

- `src/mcp/mod.rs:17` pins protocol version `"2025-06-18"` with no negotiation. That is acceptable for the current implementation but should be tracked as a forward-compat assumption.
- `src/wiki_read/mod.rs:108-110` resolves the default project from `env::current_dir()`. Host-managed stdio therefore needs the server spawned inside or above a project tree unless callers pass an explicit project id where supported. The MCP config/docs should make that expectation clear.

### Low: `cargo check --tests` warning remains

`cargo check --tests` passes with one warning:

```text
warning: unused import: `Path`
 --> tests/post_install.rs:3:17
```

This is not a merge blocker, but it is cheap cleanup.

## Confirmed Good

- `cargo check --tests` passes: 0 errors, 1 warning.
- `cargo check` passes.
- `cargo test --test mcp` passes: 15 tests.
- `cargo test --test mcp_install` passes: 4 tests.
- `cargo test --test identity_lint` passes: 3 tests.
- `cargo test --test install` passes: 34 tests.
- Claude MCP config is a manifest-owned staged asset and is removed through the manifest asset loop on uninstall.
- Codex MCP config merge/removal preserves unrelated config entries and removes only the owned server entry in the covered production case.
- MCP tool handling distinguishes malformed params / unknown tools as JSON-RPC protocol errors from runtime command/read failures as MCP tool results with `isError: true`.
- Read-path tests cover parent escape, symlink escape, binary content omission, and oversized content omission.
- No generated skill projection source modules remain in the runtime install surface; identity lint guards against reintroducing the old projection names.
- `identity_lint` passing is not evidence that the test-instance MCP identity bug is safe. That lint catches hard-coded `_test` literals minted outside `src/instance.rs`; this MCP bug is an omission that emits production names and therefore structurally cannot be caught by that lint.

## Not Run

- Full `cargo test` in this review pass.
- Cross-harness clean no-skill full-instance Phase 5 eval.
- `LLM_WIKI_INSTANCE=test` MCP config/tool-name live proof.
- Native Linux/macOS/Windows release archive proof beyond the evidence already recorded in release-E2E wiki pages.

## Overall Assessment

Do not merge this branch as the final no-legacy MCP cutover yet.

Minimum merge-readiness work:

1. Fix or explicitly decide the test-instance MCP identity behavior before running the live full-instance test.
2. Add install-side stale generated-skill reconciliation or document a hard upgrade procedure.
3. Run and record the Phase 5 clean no-skill full-instance test across both harnesses.
4. Resolve the active/rejected wiki bookkeeping once evidence exists.
5. Update README/install docs for MCP-first operation and migration.

After those are handled, the core MCP implementation looks close: the deterministic tool path, config lifecycle, read scoping, and no-legacy runtime projection deletion all have useful automated coverage.
