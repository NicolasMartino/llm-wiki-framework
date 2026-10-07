# Eval: Test Instance Namespaced Binary Implementation Review

Document Class: Eval
Status: Candidate
Date: 2026-06-22
Category: Implementation review
Scope: Line-level review of the `wiki/plans/test-instance-namespaced-binary.plan.md` implementation across the full index diff from `03083c3` through the staged worktree.
Sources: `wiki/plans/test-instance-namespaced-binary.plan.md`; `wiki/decisions/test-instance-namespaced-binary.decision.md`; local diff `git diff 03083c3`; rendered test-instance skill output from `LLM_WIKI_INSTANCE=test target/debug/llm-wiki build --target both`
Related: `wiki/plans/test-instance-namespaced-binary.plan.md`; `wiki/decisions/test-instance-namespaced-binary.decision.md`

## Review Surface

Reviewed the implementation diff from `03083c3` through the current staged index, covering the committed implementation commits `c2b6b04` and `806c68e` plus staged follow-up changes.

Files reviewed: `build.rs`, `justfile`, `src/build.rs`, `src/cli.rs`, `src/doctor.rs`, `src/embed.rs`, `src/install.rs`, `src/instance.rs`, `src/main.rs`, `src/manifest/schema.rs`, `src/paths.rs`, `src/skill_render.rs`, `src/status.rs`, `src/uninstall.rs`, `tests/identity_lint.rs`, `tests/instance_snapshot.rs`, the new snapshot, the then-current test-instance smoke script, and the related wiki updates.

## Findings

### High: rendered test skills still route agents to unsuffixed skill names

`src/skill_render.rs:18-20` only rewrites `frontmatter.name` and the `{llm_wiki_binary}` marker. It does not rewrite cross-skill references inside the skill bodies. The source dispatcher still says to route `init` to `wiki-init`, `query` to `wiki-query`, etc. (`assets/skills/wiki/SKILL.md:21-27`).

I confirmed this in rendered test-instance output: directories and frontmatter become `wiki-test`, `wiki-init-test`, etc., and binary command examples become `llm-wiki-test`, but dispatcher routes and invocation examples still point to unsuffixed `wiki-*` names. That can cause a live test-instance session to fall back to production skills or tell the agent to invoke production skill names, defeating the purpose of coexisting test and production installs.

Expected fix: teach skill rendering about the active skill namespace, not only the active binary. At minimum, render dispatcher route targets, dispatcher aliases, direct skill invocations, and "use wiki-research first" style cross-skill references with `instance::skill_name(...)`. Add a test-instance build snapshot or targeted assertions that fail on unsuffixed `wiki`, `wiki-init`, `wiki-ingest`, `wiki-lint`, `wiki-query`, or `wiki-research` references in rendered test skills except where intentionally describing production.

### Medium: the Cargo artifact name remains `target/debug/llm-wiki`, despite plan language saying the test build yields `llm-wiki-test`

`Cargo.toml:8-9` still declares the single binary target as `llm-wiki`. A `LLM_WIKI_INSTANCE=test cargo build --bin llm-wiki` build therefore emits only `target/debug/llm-wiki`; it does not create `target/debug/llm-wiki-test`. The then-current smoke script reflected this by building with `LLM_WIKI_INSTANCE=test` and then invoking `target/debug/llm-wiki` for install/status/uninstall.

The runtime identity is correct after that build (`Usage: llm-wiki-test`), and install copies the managed binary under the test instance name. The failure mode is operational: after a manual test build, the path that looks like the production debug binary is actually a test-identity binary until rebuilt. The plan still says "`LLM_WIKI_INSTANCE=test cargo build` yields `llm-wiki-test`" (`wiki/plans/test-instance-namespaced-binary.plan.md:73`), which is not true for Cargo's target artifact.

Expected fix: either produce an explicit developer artifact named `target/debug/llm-wiki-test` after the test build, or update the plan/smoke docs to call out the "carrier artifact" behavior explicitly and require running the managed `~/.llm_wiki-test/bin/llm-wiki-test` binary for live proof. The current smoke cleanup rebuilds the default binary, but it only protects that script's happy and trapped exit paths.

### Medium: the legacy test-instance smoke does not prove the managed test binary or rendered test skills are safe

The smoke script verifies the then-current profile/setup surface, `_test` tool names, manifest `installed_by`, status output, and manifest removal. It does not run the installed managed binary at `$HOME/.llm_wiki-test/bin/llm-wiki-test`, assert `manifest.binary.path`, or inspect generated skill bodies for the suffixed managed binary path and suffixed skill references.

Because status and uninstall are invoked through `target/debug/llm-wiki`, the smoke can pass even if the managed binary path or skill payloads are wrong, as long as the carrier debug binary can read the test manifest. This missed the rendered-skill namespace bug above.

Expected fix: after install, run `$SMOKE_HOME/.llm_wiki-test/bin/llm-wiki-test status`; assert the manifest binary path ends with `.llm_wiki-test/bin/llm-wiki-test`; grep installed/rendered skills for the managed test binary path; and reject unsuffixed skill references in the test-instance skill payloads.

### Medium: the identity lint is narrower than the invariant it documents

`tests/identity_lint.rs:43` scans only `src/`, and `tests/identity_lint.rs:24-32` forbids binary/home/tool identities but not skill identities such as `wiki-test` or `wiki-init-test`. That leaves meaningful minting/drift sites outside the lint's coverage:

- `build.rs:39` hard-codes `llm-wiki-test` to emit `LLM_WIKI_COMPILED_BINARY_STEM`.
- `src/instance.rs:95` hard-codes `llm-wiki-test search` during the then-existing profile rewriting instead of using the binary-stem helper.
- Rendered skill namespace references are not part of the forbidden identity set.

Some duplication in `build.rs` is structurally necessary because the build script runs before the crate. Still, the lint currently says suffixed identities are minted only by the derivation API, while the implementation has at least one required minting site outside `src/instance.rs` and several untracked rendered-output identities.

Expected fix: either narrow the invariant text to "runtime source under `src/`" and add explicit tests for build-script/profile/rendered-skill consistency, or broaden the lint/test suite to cover `build.rs` plus generated test output. Also consider failing on unreadable source files instead of `unwrap_or_default()` at `tests/identity_lint.rs:52`, and allowlisting the exact `src/instance.rs` path rather than any file named `instance.rs`.

## Checks That Passed

- `cargo test --test identity_lint --test instance_snapshot` passed: 3 tests across 2 suites.
- `LLM_WIKI_INSTANCE=staging cargo build --bin llm-wiki` failed in `build.rs` with the unsupported-instance message.
- `LLM_WIKI_INSTANCE=test cargo build --profile dist --bin llm-wiki` failed in `build.rs` with `PROFILE="release"`, confirming the `dist` profile trips the release guard.
- `LLM_WIKI_INSTANCE=test just release-guard` failed before running a release command.
- `LLM_WIKI_INSTANCE=test cargo build --bin llm-wiki` succeeded; `target/debug/llm-wiki --help` displayed `Usage: llm-wiki-test`.
- Rebuilt the default debug binary afterward with `cargo build --bin llm-wiki`.

## Not Run

- Full `just verify`.
- The then-current test-instance smoke, because it installed an external compression package into a venv and was not needed to prove the review findings.

## Resolution (2026-06-22)

All four findings were addressed in follow-up work; verified by `cargo test`
(workspace), the production output-equality snapshot (`tests/instance_snapshot.rs`,
unchanged → production rendering is byte-for-byte identical), and the later
test-instance live-session proof.

- **High (rendered skill namespace):** `src/skill_render.rs` now rewrites
  cross-skill references in all body fields via a delimiter+boundary scanner
  (`` `name` ``, `<name>`, `$name`), and `apply_skill_namespace_to_text` applies
  the same rewrite to the Codex runtime-config YAML (`build.rs`, `install.rs`).
  The Codex projector's generated *dispatcher aliases* were a second leak: it
  hard-coded `$wiki` and detected the dispatcher by the literal name `"wiki"`
  (which the rename broke, so `wiki-test` wrongly emitted aliases). Fixed in
  `crates/llm-wiki-schema/src/projector/codex.rs` — dispatcher detection now keys
  on `invocation_style == Dispatch`, and the alias token comes from a new
  `CodexProjector::with_dispatcher_name` (default `wiki`, set to
  `instance::skill_name("wiki")` at both call sites). Rendered test payloads now
  pass a boundary-aware leak scan across all three reference forms.
- **Medium (Cargo artifact):** plan Deliverable now documents the carrier-artifact
  behavior explicitly and points live proof at the managed
  `~/.llm_wiki-test/bin/llm-wiki-test`.
- **Medium (smoke proof):** the smoke now runs the *managed* test binary
  (`status`), asserts the manifest binary path, greps rendered skills for the
  managed binary path and the suffixed dispatcher route, and rejects any
  unsuffixed cross-skill reference.
- **Medium (identity lint):** allowlist matches the exact `src/instance.rs` path
  (not any `instance.rs`); unreadable source files now fail the test instead of
  `unwrap_or_default()`; the scope note narrows the invariant to runtime source
  under `src/` and explains why `build.rs` and rendered output are covered
  elsewhere. `src/instance.rs` no longer hard-codes the suffixed retired-profile
  path or binary stem — it derives them from the existing helpers.

## Overall Assessment

Follow-up verification on 2026-06-22 found the review findings addressed. The rendered test skills now namespace dispatcher routes, direct invocations, Codex dispatcher aliases, and cross-skill references; the smoke now runs the managed `~/.llm_wiki-test/bin/llm-wiki-test` binary, asserts the managed binary path, and rejects unsuffixed rendered skill references. The Cargo debug artifact remains the expected carrier path (`target/debug/llm-wiki`), while the deliverable test binary is the managed install artifact. Identity lint scope is now explicit and backed by exact-path allowlisting, unreadable-file failure, the empty-namespace snapshot, and rendered-payload smoke assertions.
