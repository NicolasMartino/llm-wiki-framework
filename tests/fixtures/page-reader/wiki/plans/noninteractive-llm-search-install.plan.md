# Plan: Non-Interactive LLM Search Install

- Document Class: Plan
- Status: Completed
- Date: 2026-05-23
- Category: Install UX, search model materialization, automation
- Scope: Implement explicit non-interactive LLM-search installation for the
  balanced profile with named posture, model-download, and profile-license
  flags; preserve install-owned model materialization, fail closed before any
  managed-state mutation when runtime confirmations are missing, and prove the
  installed binary changed with a one-off version bump.
- Sources: wiki/proposals/noninteractive-llm-search-install.proposal.md, wiki/plans/idempotent-search-model-install.plan.md, wiki/references/llm-search-model-licensing.reference.md, wiki/checklists/observability-contract.checklist.md, wiki/plans/cli-observability.plan.md, src/cli.rs, src/install.rs, src/search_models.rs, tests/install.rs
- Related: wiki/decisions/semantic-hybrid-search-mode.decision.md, wiki/plans/semantic-hybrid-search.plan.md, wiki/proposals/search-model-selection.proposal.md, wiki/checklists/observability-contract.checklist.md

## Deliverable

Add a scripted install path:

```text
llm-wiki install --non-interactive --enable-llm-search --profile balanced --confirm-model-downloads --accept-profile-licenses
```

The command enables the same balanced semantic/hybrid search profile as the
interactive enabled install path. It writes the same global search config shape
as today: `[project_default]` and `[global_search]` both enabled from the
selected profile.

The lexical/no-LLM scripted path remains:

```text
llm-wiki install --non-interactive --disable-llm-search
```

The existing `llm-wiki install --disable-llm-search` remains valid for backward
compatibility, but docs should prefer the explicit `--non-interactive` spelling.

## Non-Negotiable Constraints

1. No broad `--yes` flag is added.
2. Model downloads remain install-owned. Search, search-all, index, index-all,
   doctor, eval, and wiki-query must not download model artifacts.
3. Missing runtime confirmations fail before any managed install/search state is
   mutated. That includes `accepted-licenses.toml`, `models/artifacts.toml`,
   `search.toml`, `external-dependencies.toml`, model files, indexes, managed
   binary bytes, installed skills, partial markers, backups, and manifest state.
4. `--confirm-model-downloads` confirms search model downloads or replacement
   downloads only. It does not accept licenses and does not auto-answer other
   prompts.
5. `--accept-profile-licenses` accepts only licenses/terms for the selected
   profile's required models, not the whole catalog.
6. `--force` remains the only way to replace a hash-mismatched managed model
   artifact.
7. `--configure-search` is redundant beside explicit non-interactive posture
   flags. It does not change the selected posture or make prompts legal.
8. The implementation must satisfy the CLI observability checklist.

## In Scope

- Add install CLI flags and validation for:
  - `--non-interactive`
  - `--enable-llm-search`
  - `--profile balanced`
  - `--confirm-model-downloads`
  - `--accept-profile-licenses`
- Preserve the existing interactive prompt flow for plain terminal installs.
- Preserve `--disable-llm-search` as a backward-compatible non-interactive
  lexical-only path.
- Add an early no-write preflight for non-interactive enabled LLM search.
- Reuse the existing idempotent install classifier and plan logic for verified,
  missing, and hash-mismatched artifacts.
- Split download confirmation and license acknowledgement into two logical
  gates while allowing interactive UI wording to remain combined when both
  gates are required.
- Add verbose diagnostics for new success and refusal paths.
- Document the recommended command in `README.md`.
- Bump the package version once for implementation proof and regenerate any
  affected snapshots.

## Out Of Scope

- Additional model/profile bundles beyond `balanced`.
- Per-project search model selection or pending profile promotion.
- Runtime command downloads.
- Changing model hashes, repository revisions, or license metadata.
- Automatic version bumping, release automation, or a new update mechanism.
- Fixing any separate direct-run qmd-rs issue.
- Changing cleanup semantics for `uninstall --search-artifacts`.

## Implementation Touchpoints

- `src/cli.rs`: install args and clap-level validation.
- `src/install.rs`: early non-interactive search preflight, posture resolution,
  install ordering, confirmation validation, diagnostics, and search config
  writes.
- `src/search_models.rs`: likely unchanged catalog; existing
  `AcceptedLicenses`, `classify_model_artifact`, `models_for_profile`, and
  profile helpers remain the source of truth.
- `src/search_profile.rs`: existing `SearchConfig::enabled` and
  `SearchConfig::disabled` write the required config shape.
- `tests/install.rs`: integration tests for CLI wiring, no-write refusal, and
  verbose diagnostics.
- `src/install.rs` unit tests: pure runtime-state matrix tests around the
  enabled-search plan and confirmation validator.
- `README.md`: scripted install command documentation.
- `Cargo.toml`, `Cargo.lock`, init snapshots: one-off version bump and generated
  version-bearing fixture updates.

## Phase 1 - CLI Surface And Static Validation

Add install flags:

```text
--non-interactive
--enable-llm-search
--profile balanced
--confirm-model-downloads
--accept-profile-licenses
```

Use clap for static shape checks where it fits:

- `--enable-llm-search` conflicts with `--disable-llm-search`.
- `--enable-llm-search` requires `--non-interactive`.
- `--profile` requires `--enable-llm-search`.
- The only accepted initial profile value is `balanced`.
- `--confirm-model-downloads` requires `--enable-llm-search`.
- `--accept-profile-licenses` requires `--enable-llm-search`.

Implement any one-of posture validation that clap cannot express cleanly in a
small install argument validator:

- `--non-interactive` requires either `--enable-llm-search` or
  `--disable-llm-search`.
- Plain non-interactive install without posture fails with guidance naming both
  supported scripted forms.
- Interactive install without `--non-interactive` keeps the current prompt-first
  behavior.

Diagnostics:

- `non-interactive: true/false`
- `enable llm search: true/false`
- `disable llm search: true/false`
- `search profile argument: balanced/default`
- `confirm model downloads: true/false`
- `accept profile licenses: true/false`

Tests:

- `--enable-llm-search` with `--disable-llm-search` fails.
- `--enable-llm-search` without `--non-interactive` fails.
- `--non-interactive` without explicit posture fails and writes no install
  metadata.
- `--profile balanced` without `--enable-llm-search` fails.
- `--confirm-model-downloads` without `--enable-llm-search` fails.
- `--accept-profile-licenses` without `--enable-llm-search` fails.

## Phase 2 - Early No-Write Search Preflight

Current install writes partial markers, backups, managed binary bytes, installed
skills, and manifest state before `configure_search`. That order is wrong for
non-interactive enabled-search refusal: missing runtime confirmations must leave
no managed state behind.

Add an early preflight before partial marker recovery or any filesystem writes
when `--non-interactive --enable-llm-search` is present.

The preflight may read:

- global paths from `Paths::from_env`
- `accepted-licenses.toml`
- managed model files for SHA-256 classification
- current search config for diagnostics

The preflight must not write:

- partial marker
- backups
- managed binary
- installed skills
- manifest
- accepted licenses
- model artifacts
- search config
- external dependencies
- model files
- index files

The preflight should produce a reusable enabled-search decision:

```text
EnabledSearchPreflight {
  profile,
  models,
  install_plan,
  license_confirmation_required,
  model_download_confirmation_required,
}
```

The implementation may re-run cheap reads later if necessary, but the first
runtime-confirmation refusal must happen before install mutation begins.

Tests:

- Fresh temp `HOME`, command includes `--accept-profile-licenses` but omits
  `--confirm-model-downloads`: failure, no `.llm_wiki/manifest.json`, no
  `.llm_wiki/install.partial.json`, no `.llm_wiki/accepted-licenses.toml`, no
  installed skill trees.
- Fresh temp `HOME`, command includes `--confirm-model-downloads` but omits
  `--accept-profile-licenses`: same no-write assertion.
- Hash-mismatch preflight without `--force`: failure before install mutation.

## Phase 3 - Runtime Confirmation Validator

Extract a pure validator close to `plan_enabled_search_install`:

```text
validate_noninteractive_enabled_search(
  args,
  enabled_search_plan,
) -> Result<()>
```

Runtime rules:

- If any action is `Download` or `Replace`, require `--confirm-model-downloads`.
- If licenses are missing or stale for any required model, require
  `--accept-profile-licenses`.
- If any model is hash-mismatched, `plan_enabled_search_install` keeps requiring
  `--force` before producing a replace action.
- Extra `--confirm-model-downloads` and `--accept-profile-licenses` flags are
  accepted as no-op scripted intent when not needed.

The existing interactive code can continue to show one combined confirmation
when both gates are required, but the internal state should treat license
acceptance and download confirmation separately.

Pure unit tests:

- all artifacts verified and licenses current: no confirmations required
- all artifacts verified and licenses stale/missing: profile-license flag
  required
- missing artifact and licenses current: download flag required
- missing artifact and licenses stale/missing: both flags required
- hash mismatch without `--force`: plan refuses before validator
- hash mismatch with `--force`: download flag required, and profile-license flag
  required only when license state is stale/missing
- extra consent flags when everything is current: success

## Phase 4 - Install Flow Integration

Thread the preflight decision into `configure_search`:

1. `--disable-llm-search` keeps using `configure_disabled_search`.
2. `--non-interactive --enable-llm-search` skips `prompt_search_posture` and
   enters enabled search configuration with the preflighted profile and plan.
3. `--configure-search` beside an explicit non-interactive posture is accepted
   as redundant and diagnosed in verbose output.
4. Interactive install continues to call `prompt_search_posture`.

Update `configure_enabled_search` so it can take either:

- an existing preflighted `EnabledSearchPreflight`, or
- no preflight, in which case it computes the current plan for the interactive
  path as today.

Preserve current config semantics:

- `SearchConfig::enabled` writes both `[project_default]` and `[global_search]`.
- `SearchConfig::disabled` writes both sections disabled and invalidates active
  accepted-license readiness as today.
- Missing or stale `models/artifacts.toml` is repaired from verified files.
- Verified artifacts are reused.
- Missing artifacts are downloaded only after required confirmations.
- Hash mismatches are never silently used.

## Phase 5 - Observability And User Output

Apply the observability checklist directly.

Verbose diagnostics must explain:

- command: install
- non-interactive mode
- selected posture: enabled, disabled, or interactive prompt
- selected profile and whether it was explicit
- model count
- artifact classification counts: verified, missing, hash-mismatched, replace
  allowed by force
- license classification counts: current, missing, stale
- skipped prompts because `--non-interactive` was supplied
- missing runtime confirmation when refusal occurs
- redundant `--configure-search` beside explicit non-interactive posture
- no-write preflight refusal before install mutation

Normal stdout can keep existing install messages. Verbose diagnostics must use
`CliContext` / tracing stderr and must not change exit behavior.

Tests:

- verbose enabled non-interactive refusal reports selected posture, profile,
  artifact/license state, and the missing confirmation.
- verbose `--non-interactive --disable-llm-search` still reports disabled search
  configuration.
- existing non-verbose output assertions remain stable.

## Phase 6 - Tests And Fixtures

Use pure unit tests for the full runtime matrix and integration tests for CLI
wiring and no-write behavior.

Required integration tests in `tests/install.rs`:

- static validation failures listed in Phase 1
- no-write missing `--confirm-model-downloads`
- no-write missing `--accept-profile-licenses`
- redundant `--configure-search` with explicit non-interactive posture
- disabled non-interactive path writes the same disabled search profile
- verbose refusal diagnostics

Required unit tests in `src/install.rs`:

- all-current reuse with no confirmations
- all-current reuse with extra confirmations
- verified artifacts with stale licenses
- missing artifacts with current licenses
- missing artifacts with stale/missing licenses
- hash mismatch without `--force`
- hash mismatch with `--force`

If the existing helpers make true integration success for enabled LLM search
impractical without real model artifacts, keep success coverage pure and avoid
networking in tests. Integration tests must not download model bytes.

## Phase 7 - Documentation And Version Proof

Update `README.md` install documentation:

```text
llm-wiki install
llm-wiki install --non-interactive --disable-llm-search
llm-wiki install --non-interactive --enable-llm-search --profile balanced --confirm-model-downloads --accept-profile-licenses
```

Explain that the full enabled command is recommended for portable automation
and that the shorter minimum flag sets are state-dependent.

Bump the package version once, likely from `0.2.0` to `0.2.1`, to prove the
managed install copied the current binary. Update:

- `Cargo.toml`
- `Cargo.lock`
- version-bearing snapshots
- any README/example version references if present

Run the install proof:

```text
cargo run -- install --non-interactive --enable-llm-search --profile balanced --confirm-model-downloads --accept-profile-licenses --skip-path-guidance
~/.llm_wiki/bin/llm-wiki --version
```

The version bump is one-time evidence, not a release or update mechanism.

## Verification

Focused verification:

```text
cargo fmt --check
cargo test --bin llm-wiki install::tests
cargo test --test install
cargo test --test status_doctor
cargo test --test search_commands
cargo test --test init
cargo insta test --test init --accept
git diff --check
```

Run `cargo test --workspace` if the focused tests pass and the change touches
shared install or search model helpers more broadly than expected.

Manual proof:

1. Confirm old managed binary version.
2. Bump package version once.
3. Run the full non-interactive enabled install command.
4. Confirm `~/.llm_wiki/bin/llm-wiki --version` reports the bumped version.
5. Rerun install and verify it reuses verified artifacts without redownloading.

## Implementation Evidence

Completed on 2026-05-23.

Implemented:

- Added `--non-interactive`, `--enable-llm-search`, `--profile balanced`,
  `--confirm-model-downloads`, and `--accept-profile-licenses` to
  `llm-wiki install`.
- Added early no-write runtime preflight for
  `--non-interactive --enable-llm-search`.
- Split model download confirmation from selected-profile license acceptance.
- Preserved `--disable-llm-search` behavior and accepted `--configure-search`
  as redundant beside explicit non-interactive posture.
- Added verbose diagnostics for posture, profile, artifact classification,
  license classification, skipped prompts, redundant configuration, and missing
  runtime confirmation refusals.
- Updated README automation docs.
- Bumped the workspace package version from `0.2.0` to `0.2.1` for one-off
  managed-binary proof.

Verification completed:

```text
cargo fmt --check
cargo test --bin llm-wiki install::tests
cargo test --test install
cargo test --test status_doctor
cargo test --test search_commands
cargo insta test --test init --accept
cargo test --test init
git diff --check
```

Manual version proof:

```text
/Users/nicolasmartino/.llm_wiki/bin/llm-wiki --version
# llm-wiki 0.2.0

cargo run -- install --non-interactive --enable-llm-search --profile balanced --confirm-model-downloads --accept-profile-licenses --skip-path-guidance
# All required model artifacts are already installed and verified; reusing managed copies.

/Users/nicolasmartino/.llm_wiki/bin/llm-wiki --version
# llm-wiki 0.2.1
```

## Acceptance Criteria

- Plain redirected-stdin `llm-wiki install` still fails closed with explicit
  search posture guidance.
- `--non-interactive --disable-llm-search` succeeds and writes the same disabled
  search profile as the existing disabled path.
- `--non-interactive --enable-llm-search --profile balanced` succeeds only when
  no runtime confirmations are required.
- The full recommended enabled command is accepted in every runtime state where
  `--force` is not required.
- Missing runtime confirmations fail before any managed install/search state is
  written.
- Enabled non-interactive install writes the same `[project_default]` and
  `[global_search]` config as interactive enabled install.
- Interactive install behavior remains prompt-first.
- No runtime search/index/eval command downloads model artifacts.
- Verbose diagnostics explain the new posture, profile, artifact, license, and
  refusal decisions without changing stdout or exit behavior.
- The managed binary version proof succeeds after the one-off package bump.
