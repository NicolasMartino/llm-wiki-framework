# Plan: macOS Installed Binary Codesign Repair

- Document Class: Plan
- Status: Completed (master)
- Date: 2026-06-25
- Category: macOS install, test instance, binary materialization, release readiness
- Scope: Repair the macOS launch hang where newly installed `llm-wiki-test` and the matching local build binary enter kernel wait before CLI argument handling, by making macOS binary signing an explicit, tested part of managed install and release/test-instance proof.
- Sources: Live diagnostic observation on 2026-06-23; `src/install.rs`; `src/paths.rs`; `src/manifest/schema.rs`; `tests/post_install.rs`; `tests/mcp_install.rs`; `wiki/plans/test-instance-namespaced-binary.plan.md`; `wiki/plans/headroom-mcp-merge-readiness-repair.plan.md`; `wiki/roadmaps/cross-platform-release-e2e.roadmap.md`.

## Where This Stands (2026-10-06)

Completed against its own "Closure Criteria" below: the repair, its focused
tests and the live test-instance proof are recorded in "2026-06-25
Implementation Evidence", `wiki/log.md` records it, and the staged, signed
install is on master (`src/install.rs`, `manifest.binary.source_hash` in
`src/manifest/schema.rs`).

Still pending, outside this plan: the macOS release archive half of Phase 5's
gate. That proof was never run here; it is
`wiki/roadmaps/cross-platform-release-e2e.roadmap.md`, "R4 - Native macOS
Release Archive Proof" (Draft).

## Objective

Make macOS managed binaries reliably executable after install. `llm-wiki-test --version`, `--help`, `path`, and `mcp serve` must start normally from the managed home after local test-instance install and from the macOS release archive flow.

The repair must preserve install ownership semantics: manifests, partial-install recovery, uninstall collision detection, and reinstall idempotence must describe the final installed binary, not the unsigned or linker-signed source executable when those bytes differ.

## Evidence

Live diagnosis showed:

1. `/Users/nicolasmartino/.llm_wiki-test/bin/llm-wiki-test --version`, `--help`, and `path` produced no output within 10 seconds.
2. The hung processes entered macOS `U` state and did not exit from `SIGTERM` or `SIGKILL`, which indicates an uninterruptible kernel wait before normal Rust CLI handling.
3. The managed binary and `target/debug/llm-wiki` had the same SHA-256, so install copied the expected build.
4. `codesign --verify --strict` reported the binary valid, but `spctl --assess --type execute` returned `internal error in Code Signing subsystem`.
5. A disposable copy of the same binary under `/private/tmp`, after `xattr -c` and `codesign --force --sign -`, ran immediately and printed `llm-wiki-test 0.2.2`.
6. The repro followed installing a new test-instance binary on top of an existing global test-instance install. The install path currently writes with `fs::copy(current_exe, managed_binary)`, so replacement is not staged through a separate inode plus atomic rename. A previously spawned `llm-wiki-test mcp serve` can keep the old executable image alive while the installer overwrites the managed path.

Conclusion: this is a macOS execution/signing/materialization defect, not an MCP-only profile defect and not a CLI argument parsing defect. The most likely trigger is in-place replacement of a Mach-O binary that may still be referenced by an existing MCP server process, leaving code-signing/runtime assessment in an inconsistent state for new launches.

## In Scope

1. macOS-specific managed binary signing during `install`.
2. Manifest hash and ownership semantics for signed installed binaries.
3. Atomic managed-binary replacement so reinstall over an existing/running MCP server does not overwrite the executable inode in place.
4. Partial-install and reinstall preflight behavior when signing changes bytes.
5. Test-instance install proof for `llm-wiki-test`.
6. macOS release/local-build smoke proof that the installed binary can execute without relying on `PATH`.
7. Focused operator guidance for clearing or avoiding wedged diagnostic processes while testing the repair.

## Out Of Scope

1. Apple Developer ID signing, notarization, or Gatekeeper distribution policy beyond ad-hoc local execution.
2. Changing Headroom behavior beyond the retained safety rule.
3. Linux or Windows binary signing.
4. Treating `spctl --assess` as the closure gate. The observed signed temp copy still returned the same `spctl` internal error while executing successfully, so actual launch proof is the gate.

## Design Constraints

1. Do not sign the currently running executable in place from inside `install`.
2. Do not overwrite the existing managed binary in place. Copy to a temporary file in the same directory, set permissions, sign the temporary file on macOS, compute final metadata from that file, then atomically rename it over the managed binary.
3. Use `std::process::Command` with explicit argv for `codesign`; do not shell out through interpolated command strings.
4. Use a stable ad-hoc signing identity/identifier for managed binaries so repeated installs are predictable.
5. If signing changes bytes, compute manifest ownership from the final signed managed binary. Do not leave `manifest.binary.hash` tied to the pre-sign source bytes.
6. Reinstall preflight must compare against the expected installed representation, not blindly against the raw current executable hash on macOS.
7. A failed `codesign` step must fail install with an actionable error and leave no final managed binary that the manifest claims as owned.
8. Tests that launch a candidate binary must be bounded so a recurrence cannot wedge the test suite indefinitely.

## Phase 0 - Test Hygiene And Local Recovery

1. Stop launching the broken installed binary repeatedly during diagnosis.
2. Record that existing `U`-state processes may require OS/session cleanup rather than normal signal handling.
3. For immediate local unblocking only, explicitly ad-hoc sign the currently installed test binary or replace it with a signed copy, then verify `--version` before continuing manual MCP tests.
4. Keep the durable fix in the installer/build path, not only as a local manual command.

Gate: local manual recovery is documented separately from the product fix, so future evidence does not confuse a one-off signed copy with an installer guarantee.

## Phase 1 - Make Binary Replacement Atomic

1. Replace `fs::copy(current_exe, managed_binary)` with a staged install path.
2. Create the staged file in the same directory as the managed binary so final replacement can use same-filesystem atomic rename.
3. Copy current executable bytes to the staged file and set target permissions there.
4. On macOS, sign the staged file before it becomes the managed binary.
5. Atomically rename the staged file over the managed binary only after copy, permission, and signing steps have succeeded.
6. Ensure partial-install cleanup removes any stale staged file without deleting an unrelated user file.

Gate: reinstalling while an old `llm-wiki-test mcp serve` process is still alive replaces the managed path without modifying the old executable inode in place.

## Phase 2 - Add macOS Signing Helper

1. Add a small macOS-only helper in the install/materialization layer, for example `sign_macos_binary(path, identifier, context)`.
2. Invoke `codesign --force --sign - --identifier <stable-id> <staged-binary>` after copying the executable and before final replacement/ownership metadata is written.
3. Keep non-macOS behavior byte-for-byte equivalent where practical.
4. Surface diagnostics under `--verbose`: signing skipped on non-macOS, signing command path, target path, and signing failure stderr.

Gate: a managed macOS binary produced by `install` has an explicit ad-hoc signature and can execute `--version`.

## Phase 3 - Reconcile Manifest And Preflight Hashes

1. Audit every place that compares `current_exe_hash`, `manifest.binary.hash`, and the managed binary hash.
2. Decide whether the manifest needs a source hash plus installed hash, or whether `manifest.binary.hash` remains the installed-file hash and source comparison uses a separate in-memory expected installed hash.
3. Update partial-install recovery so a partially signed or unsigned managed binary is classified correctly.
4. Ensure `--force` replacement behavior still handles user-modified managed binaries conservatively.
5. Preserve uninstall behavior: uninstall may remove only files proven owned by the manifest.

Gate: a second `install` after a successful signed macOS install is idempotent and does not misclassify the signed binary as a user collision.

## Phase 4 - Regression Tests

1. Extend `tests/post_install.rs` so the managed binary launch proof is bounded and asserts `--version` exits successfully after install.
2. Add a macOS-specific assertion that the manifest hash matches the final installed binary bytes after signing.
3. Add or extend a test-instance install test so the `llm-wiki-test` managed binary can execute independently of `PATH`.
4. Add a reinstall-over-existing test that proves binary replacement uses a staged file plus atomic rename instead of writing through the managed path.
5. Add a failure-path test using a fake `codesign` on `PATH` when practical, proving install fails clearly and leaves partial state recoverable.
6. Keep existing Linux/Windows tests passing with no `codesign` dependency.

Gate: focused tests fail before the fix on macOS or under the fake failure path, and pass after the fix.

## Phase 5 - Build And Release Proof

1. Review local test-instance build/install recipes so the source executable used to run `install` can start on macOS during dogfood.
2. If needed, add a macOS post-build signing step to the relevant local test-instance or release-E2E recipe before invoking the binary.
3. Verify the macOS release archive flow produces a binary that can run `--version` and can install a managed binary that also runs `--version`.
4. Keep the managed-home MCP config pointing to the signed installed binary.

Gate: macOS local dogfood and release archive proof both show a runnable installed binary.

## Verification

Required focused verification:

1. `rtk cargo fmt`
2. `rtk cargo test --test post_install`
3. `rtk cargo test --test install`
4. `rtk cargo test --test mcp_install`
5. Test-instance macOS live proof:
   - build/install the `test` instance
   - run `~/.llm_wiki-test/bin/llm-wiki-test --version`
   - run `~/.llm_wiki-test/bin/llm-wiki-test --help`
   - run an MCP initialize/tools-list smoke against `~/.llm_wiki-test/bin/llm-wiki-test mcp serve`

Recommended broader verification before merge:

1. `rtk cargo test`
2. macOS release archive smoke from `wiki/roadmaps/cross-platform-release-e2e.roadmap.md`

## 2026-06-25 Implementation Evidence

Implemented the local managed-install repair:

1. `install` now copies the current executable to a temporary file in the
   managed binary directory, applies permissions there, ad-hoc signs the staged
   file on macOS with `/usr/bin/codesign --force --sign - --identifier
   dev.llm-wiki.<binary>`, then atomically persists the staged file over the
   managed binary path.
2. `manifest.binary.hash` now records the final installed bytes, and
   `manifest.binary.source_hash` records the source executable bytes used for
   reinstall/preflight comparisons. This keeps ownership and uninstall checks
   tied to installed bytes while allowing macOS signing to change those bytes.
3. Partial-install recovery, preflight replacement checks, and backup
   snapshots account for signed installed bytes versus source executable bytes.
4. Installed-binary launch tests are bounded so a recurrence fails the test
   instead of wedging the suite. The tests assert the manifest hash matches the
   installed binary and that the managed binary can launch without `PATH`.
5. Codex MCP config merge/uninstall now preserves existing TOML text around the
   managed server block, so the test-instance live proof can verify production
   Codex config is restored byte-for-byte after `llm-wiki-test` uninstall.
6. `tools/test-instance-live-session-proof.sh` now proves the installed
   `llm-wiki-test` launches `--version`, `--help`, `status`, and
   `mcp serve`/`tools/list` from the managed home.

Verification passed:

1. `rtk cargo test --test post_install`
2. `rtk cargo test --test install`
3. `rtk cargo test --test mcp_install`
4. `rtk cargo test --test identity_lint`
5. `rtk cargo check --tests`
6. `rtk cargo test --test mcp`
7. `rtk cargo test --test status_doctor`
8. `rtk cargo test --test search_commands`
9. `rtk cargo insta test --test init --accept`
10. `rtk cargo clippy`
11. `rtk cargo test` (314 passed, 2 ignored)
12. `rtk just test-instance-live-session-proof`

Remaining evidence: native macOS release-archive proof remains governed by
`wiki/roadmaps/cross-platform-release-e2e.roadmap.md`.

## Closure Criteria

This plan closes when:

1. macOS `install` explicitly produces a runnable signed managed binary.
2. Manifest and reinstall semantics account for final installed bytes.
3. The test instance no longer hangs on `--version`, `--help`, or MCP startup after global install.
4. Focused tests and one live macOS dogfood pass.
5. `wiki/index.md` and `wiki/log.md` record the completed repair and evidence.
