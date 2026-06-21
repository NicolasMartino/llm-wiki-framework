# Plan: Headroom Runtime Companion

- Document Class: Plan
- Status: Draft
- Date: 2026-06-21
- Category: Agent runtime, context compression, provenance safety
- Scope: Execute the ten Acceptance Criteria in
  `wiki/proposals/headroom-runtime-companion.proposal.md` by shipping an
  opt-in Headroom profile, documenting Modes A through D, adding a managed
  `install --with-headroom` materialization path, adding a minimal doctor
  advisory for documented Mode C usage, and pinning the Headroom and Codex
  tool-name surfaces with fixture tests. This plan must not add a runtime or
  build dependency on Headroom or either Headroom SDK.
- Sources:
  - `wiki/proposals/headroom-runtime-companion.proposal.md`
  - `raw/research/2026-06-11-headroom-llm-wiki-comparison/`
  - `raw/research/2026-06-20-codex-tool-surface/`
- Related:
  - `wiki/decisions/llm-wiki-binary-distribution.decision.md`
  - `wiki/decisions/binary-path-bootstrap.decision.md`
  - `wiki/decisions/agent-owns-wiki.decision.md`
  - `wiki/decisions/three-layer-architecture.decision.md`
  - `wiki/specs/wiki-query-skill.spec.md`
  - `wiki/checklists/observability-contract.checklist.md`

## Deliverable

A complete, opt-in Headroom Runtime Companion for `llm-wiki` projects.

Users can run `llm-wiki install --with-headroom`, source the materialized
profile from the `llm-wiki` managed home, and then start `headroom proxy`
themselves. For Claude Code Mode C, the profile preserves the proposal's
wiki/raw safety invariant by excluding read-like tools from Headroom's
compression path. For OpenAI Codex CLI, the profile defensively includes the
captured Codex function-tool names, but the documented recommendation remains
Mode D or Mode B because Codex performs file reads and patches through
shell-family or patch tools.

The companion is not a Headroom launcher. Default project bootstrap does not
start Headroom, no Headroom source is vendored, no Headroom SDK is adopted, and
the framework continues to install, run, doctor, and uninstall with no
Headroom binary present.

## Corrected Design Constraints

- Codex Mode C is not promised as equivalent to Claude Mode C. The plan only
  promises defensive profile coverage plus fixture tests for the Codex names
  captured on 2026-06-20. Installed Codex tool-name verification is a follow-up
  item, matching Acceptance Criterion 10.
- The Headroom pin is a package/source-surface pin, not a claim that the raw
  research bundle was a release tarball. The captured package metadata names
  `headroom-ai` version `0.24.0`; the research summary says the source archive
  reflected GitHub `main` at review time. Implementation must make that
  distinction explicit in diagnostics and tests.
- All paths are resolved through `Paths::managed_home()` or existing path
  helpers. User-facing examples may show Unix-style paths, but code and tests
  must not hardcode `~/.llm_wiki/...`.
- `doctor` is advisory only. It must not validate `HEADROOM_*`, must not fail
  when Headroom is missing, and must preserve stable parseable output.
- The materialized profile must be tracked as managed state without abusing the
  existing skill-entry model. If the current manifest schema lacks a suitable
  general asset slot, Phase 4 must add one deliberately and test backward
  compatibility.

## Touchpoints

Primary implementation modules:

- `src/cli.rs`: declares `install --with-headroom` and `uninstall --headroom`;
  defines flag conflicts and help text.
- `src/install.rs`: materializes the embedded profile, performs the
  Headroom package/source-surface check, writes managed-state metadata, and
  preserves baseline install behavior without Headroom.
- `src/uninstall.rs`: removes the materialized Headroom subtree during default
  uninstall and supports `uninstall --headroom` as a profile-only cleanup path.
- `src/doctor.rs`: emits the Mode C base-URL advisory while preserving exit
  codes and parseable output contracts.
- `src/embed.rs`: adds a non-skill embedded asset registry or equivalent helper
  for `assets/headroom/llm-wiki.profile.env`.
- `src/manifest/`: adds or reuses a manifest field for non-skill managed
  assets, with explicit schema and compatibility tests.

Supporting files:

- `assets/headroom/llm-wiki.profile.env`: the bundled profile.
- `AGENTS.MD` and `templates/base/project_guidelines.md`: read-only access
  contract, Codex-specific formulation, and `headroom_read` ban.
- `wiki/references/headroom-context-compression.reference.md`: four-mode
  operational reference.
- `tests/install.rs`: install, uninstall, manifest, no-Headroom regression, and
  release/source-surface refusal cases.
- `tests/status_doctor.rs`: human and parseable doctor advisory behavior.
- `tests/fixtures/`: fixture tests for Headroom proxy CLI/env surface,
  `DEFAULT_EXCLUDE_TOOLS`, `DEFAULT_TOOL_PROFILES` keys, and Codex function
  tool names.

## In Scope

- Create `wiki/references/headroom-context-compression.reference.md` and link
  it from `wiki/index.md`.
- Update `AGENTS.MD` and `templates/base/project_guidelines.md` with:
  - read-only access contract for `wiki/` and `raw/`;
  - Claude Code formulation using read-tool exclusions;
  - Codex formulation explaining shell-family and patch-tool carve-outs;
  - normative ban on enabling or calling `headroom_read` against `wiki/` or
    `raw/`.
- Add `assets/headroom/llm-wiki.profile.env` with the exact profile content
  documented in the proposal's Part 2.
- Embed the profile through a non-skill asset path.
- Add `install --with-headroom`, which:
  - writes the profile under `Paths::managed_home()/headroom/`;
  - records the file hash as managed state;
  - is idempotent on rerun;
  - refuses when the installed Headroom package/source surface cannot be
    verified against the captured pin;
  - preserves ordinary install when Headroom is absent and `--with-headroom`
    is not requested.
- Add `uninstall --headroom`, which removes only the managed Headroom subtree
  and updates managed-state metadata consistently.
- Add the minimal `doctor` advisory for
  `ANTHROPIC_BASE_URL=http://127.0.0.1:8787`.
- Add fixture tests that pin:
  - Headroom proxy CLI flags and matching `HEADROOM_*` env vars against
    `sources/14-headroom-source-proxy-server.py`
    (`sha256:bdd7b060f59f19dbd698890bc6fe0321de326c63f8b48e9c53e4bab4188e23d6`);
  - `DEFAULT_EXCLUDE_TOOLS` membership and `DEFAULT_TOOL_PROFILES` keys against
    `sources/17-headroom-config-excerpt.py`
    (`sha256:83354e33444d929df0d1570546829390745d8c58c7fe64ad4d88ecb456cd1853`);
  - Codex shell/request-permission tool names against
    `sources/01-codex-handlers-shell-spec.rs`
    (`sha256:27010c753a36ff3f9a9f5d4bedea92c973853ffd10f0ea809225965719a4361a`);
  - Codex patch tool name against
    `sources/02-codex-handlers-apply-patch-spec.rs`
    (`sha256:7859714042f7b0a0e8ec2fc5c53bed2c836bfd25e0452afd23760028a3e30e2a`).
- Add a gated containerized Headroom proxy E2E that proves a real
  `headroom-ai==0.24.0` install can run with the materialized profile.
- Run a dogfood evidence pass after implementation and after the containerized
  proxy E2E passes.
- Promote a decision only after implementation and dogfood bookkeeping are
  complete.

## Out Of Scope

- Vendoring Headroom source.
- Adopting Headroom TypeScript or Python SDKs.
- Starting Headroom during `llm-wiki init`, `install`, or default bootstrap.
- Migrating `llm-wiki search` from shell invocation to first-class MCP tools.
  Reserved names `llm_wiki_search` and `llm_wiki_search_all` remain forward
  compatibility entries only.
- Full install-time verification of the installed Codex CLI function-tool
  surface. Acceptance Criterion 10 requires fixture coverage for promotion;
  installed Codex verification is a follow-up plan item.
- Persistent-memory features from Headroom's SDKs.
- `headroom learn --apply` mutation of `AGENTS.MD` or framework-owned files.
- Capturing `headroom_stats` into `wiki/log.md`; measurements live in `raw/`
  and are promoted to `wiki/references/` only when the dogfood evidence is
  good enough.
- Transcript-scanning lint for read-vs-shell invariants.

## Phases

### Phase 1 - Documentation Foundation

Land the wiki-side contract before implementation depends on it.

1. Author `wiki/references/headroom-context-compression.reference.md`.
   Describe Modes A through D with citations into both research bundles.
   State that Mode D is the default recommendation for `wiki-ingest` sessions
   and for Codex unless a future Codex-specific verification plan lands.
2. Update `AGENTS.MD` with the read-only access contract for `wiki/` and
   `raw/`, the Claude formulation, the Codex formulation, and the
   `headroom_read` ban.
3. Mirror the same rules into `templates/base/project_guidelines.md`.
4. Update `wiki/index.md` with the new reference page.
5. Append a `lint` or `update` entry to `wiki/log.md`.

Verification:

- `wiki/index.md` links the reference page.
- `AGENTS.MD` and `templates/base/project_guidelines.md` contain parallel
  contract sections.
- The Codex section does not promise that proxy Mode C protects arbitrary shell
  reads.

Closes: AC 1, AC 2, AC 7.

### Phase 2 - Profile Asset And Embedding

Create the profile as a framework-owned asset without modeling it as a skill.

1. Create `assets/headroom/llm-wiki.profile.env` with the exact content from
   the proposal's Part 2.
2. Add a non-skill embedded asset registry or helper in `src/embed.rs`.
3. Add a unit test that round-trips the embedded profile against the on-disk
   asset.
4. Add a test that the profile includes the Claude Code default names,
   the captured Codex names, and the reserved future `llm_wiki_search` /
   `llm_wiki_search_all` names.

Verification:

- Build succeeds.
- The profile is retrievable through the embedded asset path.
- Round-trip and profile-content tests pass.

Closes: AC 3 asset existence.

### Phase 3 - Upstream Pin Fixture Tests

Make fixture drift visible before implementation relies on the profile.

1. Add fixture test 1 for `sources/14-headroom-source-proxy-server.py`.
   Assert the CLI/env surface used by the profile still exists in the captured
   source: `--no-optimize` / `HEADROOM_OPTIMIZE`, `--exclude-tools` /
   `HEADROOM_EXCLUDE_TOOLS`, `--tool-profile` / `HEADROOM_TOOL_PROFILES`,
   `--compress-user-messages` / `HEADROOM_COMPRESS_USER_MESSAGES`,
   `--min-tokens` / `HEADROOM_MIN_TOKENS`, `--max-items` /
   `HEADROOM_MAX_ITEMS`, `HEADROOM_CCR_TTL_SECONDS`, `--disable-kompress` /
   `HEADROOM_DISABLE_KOMPRESS`, and `HEADROOM_MCP_READ`.
2. Add fixture test 2 for `sources/17-headroom-config-excerpt.py`.
   Assert `DEFAULT_EXCLUDE_TOOLS` membership and `DEFAULT_TOOL_PROFILES` keys
   only. Do not pin profile-preset values that were not captured verbatim.
3. Add fixture test 3 for the Codex captures:
   `shell_command`, `exec_command`, `write_stdin`, and `request_permissions`
   in `sources/01-codex-handlers-shell-spec.rs`, and `apply_patch` in
   `sources/02-codex-handlers-apply-patch-spec.rs`.
4. Add a fixture hash check so a source refresh is intentional.

Verification:

- Tests fail on a profile env-var typo.
- Tests fail on fixture edits that remove a pinned surface.
- Tests fail if fixture bytes change without an intentional hash update.

Closes: AC 4, AC 5, AC 10 fixture portion.

### Phase 4 - Install, Manifest, And Uninstall

Implement materialization and cleanup as managed runtime state.

1. Add `--with-headroom` to `install`.
2. Resolve the materialization target through `Paths::managed_home()`:
   `Paths::managed_home()/headroom/llm-wiki.profile.env`.
3. Add or reuse an explicit manifest representation for non-skill managed
   assets. The chosen shape must not overload skill entries. It must record
   path, kind, SHA-256, and owning `llm-wiki` version.
4. Preserve compatibility with existing schema-version 2 manifests. If a schema
   bump is required, document migration and unsupported-version behavior before
   implementation.
5. Implement the Headroom package/source-surface check:
   - resolve the candidate through `HEADROOM_BIN` or `PATH`;
   - verify package identity is `headroom-ai`;
   - verify version metadata is `0.24.0` when metadata is available;
   - locate the installed package source or otherwise refuse with a diagnostic
     that explains the source could not be inspected;
   - compare the installed `DEFAULT_EXCLUDE_TOOLS` and
     `DEFAULT_TOOL_PROFILES` key surface against the captured config excerpt;
   - refuse `--with-headroom` on mismatch, naming the divergent surface and
     pointing at the proposal rationale.
6. Keep plain `llm-wiki install` and `llm-wiki install --disable-llm-search`
   green without Headroom present.
7. Update `uninstall` so default uninstall removes the Headroom subtree along
   with other managed state.
8. Add `uninstall --headroom` for profile-only cleanup. It must conflict with
   `--include-binary` and `--search-artifacts`; `--force` may apply only to
   absent or malformed Headroom state.

Verification:

- Redirected-home install with `--with-headroom` materializes the profile and
  records managed-state metadata.
- Rerun is idempotent.
- Mismatched or uninspectable Headroom refuses only `--with-headroom`.
- Plain install works when Headroom is absent.
- Default uninstall removes the Headroom subtree.
- `uninstall --headroom` removes only the Headroom subtree and preserves other
  managed state.
- CLI flag-conflict tests cover `--headroom` with `--include-binary` and
  `--search-artifacts`.

Closes: AC 3 materialization portion, AC 6, AC 8 install/uninstall portion.

### Phase 5 - Doctor Advisory

Add the documented Mode C pointer without turning doctor into a Headroom
validator.

1. Detect exactly `ANTHROPIC_BASE_URL=http://127.0.0.1:8787`.
2. For human output, emit one advisory line that says the Mode C profile must be
   sourced before starting `headroom proxy`.
3. Reference a stable target. If the project reference page is not guaranteed
   to exist in initialized user projects, prefer a command-neutral phrasing
   such as "see the Headroom context compression reference in this framework
   wiki" plus the managed profile path resolved from `Paths::managed_home()`.
4. Do not validate `HEADROOM_*`, do not require Headroom to be installed, and
   do not fail doctor.
5. Preserve parseable output. If doctor currently has JSON or other structured
   output, add an optional advisory field without changing existing field
   meanings; otherwise prove the human-only advisory does not affect parseable
   commands.

Verification:

- Env var unset: no Headroom advisory.
- Env var set to another URL: no Headroom advisory.
- Env var set to the Mode C URL: exactly one advisory in human output.
- Exit code is unchanged in all cases.
- Structured output tests prove stable schema or intentionally added advisory
  field.
- No-Headroom-present doctor still succeeds.

Closes: AC 9, AC 8 doctor portion.

### Phase 6 - Containerized Headroom Proxy E2E

Prove the companion against a real Headroom process in an isolated Linux
environment. This is a gated E2E lane, not part of default `cargo test`, because
it depends on Python packaging, container runtime availability, and optionally
network access when the container image is not prebuilt.

1. Add a dedicated recipe, tentatively `just headroom-proxy-e2e`, or an
   equivalent release-E2E runner entry.
2. Build or use a minimal Debian-based container with Python and
   `headroom-ai==0.24.0` installed. The final CI path should avoid live package
   downloads by using a pinned image, vendored wheel cache, or prebuilt artifact.
3. Copy the current `llm-wiki` binary into the container and run
   `llm-wiki install --with-headroom` under an isolated `HOME`.
4. Source the materialized profile from
   `Paths::managed_home()/headroom/llm-wiki.profile.env`.
5. Start `headroom proxy` on `127.0.0.1:8787` with the sourced profile.
6. Run a tiny scripted proxy request that exercises the Headroom server enough
   to produce `headroom_stats` and prove the proxy is reachable. This request
   must use synthetic content only; no project `wiki/` or `raw/` content is
   needed for the smoke.
7. Run `llm-wiki doctor` with `ANTHROPIC_BASE_URL=http://127.0.0.1:8787` and
   assert the advisory appears exactly once.
8. Assert the generated profile contains the expected exclude set inside the
   container and that the installed Headroom package/source-surface check passed.
9. Cleanly stop the proxy and save stdout, stderr, `headroom_stats`, package
   metadata, profile hash, and container image identifier under a temporary E2E
   output directory.

Verification:

- The E2E fails if `headroom-ai` cannot be inspected or does not match the
  pinned package/source surface.
- The E2E fails if `llm-wiki install --with-headroom` does not materialize the
  profile in managed home.
- The E2E fails if `headroom proxy` cannot start with the materialized profile.
- The E2E fails if `doctor` does not emit exactly one Mode C advisory.
- Default `cargo test --workspace` remains independent of Docker, Python
  packaging, and network access.

Closes: real Headroom proxy integration coverage for AC 3, AC 6, AC 8, and
AC 9. It does not replace the dogfood evidence pass because it is a smoke test,
not a representative ingest workload.

### Phase 7 - Dogfood Evidence Pass

Run after Phases 1 through 6 land. This phase may produce only raw evidence; a
wiki reference is promoted only when evidence warrants it.

1. Define the measurement fixture before running:
   - same research bundle or ingest fixture for both runs;
   - same agent instructions except for Mode C vs Mode D;
   - explicit cache reset or cache-retention policy;
   - token-count collection method;
   - transcript redaction policy;
   - provenance-breakage checks for `wiki/` and `raw/` reads.
2. Run Mode D baseline and Mode C profile run.
3. Capture `headroom_stats` JSON, before/after token counts, relevant stderr
   or stdout advisory snippets, and the exact profile hash.
4. Write captures and `research-summary.md` under
   `raw/research/2026-06-XX-headroom-dogfood/` with the date set at run time.
5. Promote `wiki/references/headroom-dogfood-evidence.reference.md` only if
   measurements show meaningful savings and no observable provenance breakage.
   "Meaningful" must be defined in the research summary before interpreting
   results. If the result is flat or negative, leave the finding in `raw/` and
   do not promote a positive reference page.

Verification:

- Raw bundle exists with reproducible setup notes.
- Research summary records the threshold, method, measurements, and conclusion.
- Promoted reference page, if any, cites the raw bundle and is linked from
  `wiki/index.md`.

Closes: dogfood requirement from
`raw/research/2026-06-11-headroom-llm-wiki-comparison/research-summary.md`.

### Phase 8 - Decision Promotion And Bookkeeping

Close the wiki state machine without re-accepting already accepted documents.

1. Author `wiki/decisions/headroom-runtime-companion.decision.md` after
   implementation and dogfood bookkeeping are complete.
2. Record the decision as the durable accepted design, citing the proposal,
   reference page, tests, and dogfood bundle.
3. Do not mark the proposal Accepted again; it is already `Status: Accepted`
   with `Promoted: 2026-06-20`.
4. Update this plan's status to Completed only after all phase verification
   gates pass.
5. Update `wiki/index.md` and append to `wiki/log.md`.

Verification:

- Decision page exists and is indexed.
- Proposal status is not churned incorrectly.
- Plan status changes to Completed only after implementation gates pass.
- Log entry names pages changed.

Closes: remaining promotion bookkeeping. AC 8 is reasserted through the
no-Headroom regression tests from Phases 4 and 5 plus the real-proxy smoke from
Phase 6; all other ACs are closed by the phase that owns them.

## Risks And Mitigations

- Headroom upstream changed after the raw capture. Mitigation: fixture hashes
  make stale captures explicit; `--with-headroom` refuses uninspectable or
  mismatched installed packages.
- The source capture was from GitHub `main`, not a release tarball. Mitigation:
  diagnostics and tests call this a package/source-surface pin for
  `headroom-ai` `0.24.0`, not a stronger artifact-authenticity claim.
- Manifest tracking grows beyond the current schema shape. Mitigation: Phase 4
  requires an explicit non-skill managed-asset representation and compatibility
  tests before materialization is considered complete.
- Codex tool names change. Mitigation: fixture tests catch drift when raw
  captures are refreshed; install-time Codex verification remains a documented
  follow-up, so the plan does not promise more than it tests.
- Dogfood shows little or no savings. Mitigation: negative results are valid
  raw evidence and do not block the safety decision; they only block promotion
  of a positive dogfood reference page.
- Containerized Headroom E2E is unavailable on a contributor machine.
  Mitigation: keep it as a gated release/E2E recipe with captured artifacts,
  not a default unit or integration test. CI can run it where Docker and the
  pinned package image are available.
- Projects enable `HEADROOM_MCP_READ=on`. Mitigation: documented normative ban
  in `AGENTS.MD`, project guidelines, and the reference page. No code
  enforcement is added in this plan.

## Verification Gates

Every implementation PR or patch series for this plan must run:

- `cargo test --workspace`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo insta test --workspace --check` when snapshot tests are touched
- `just audit-legacy` when generated skill/runtime surfaces are touched
- `just headroom-proxy-e2e` or the finalized equivalent before promotion, on a
  host with the required container runtime and pinned Headroom package image
- `git diff --check`
- Wiki lint pass covering `wiki/index.md`, this plan, the proposal, and any new
  reference or decision pages
