# Plan: Idempotent Search Model Install

- Document Class: Plan
- Status: Active
- Date: 2026-05-14
- Category: Install UX, search model materialization, managed runtime state
- Scope: Make repeated `llm-wiki install` and `llm-wiki install --configure-search` reuse already verified search model artifacts instead of downloading unchanged model bytes again, while keeping search-type changes separate from artifact deletion.
- Sources: user discussion 2026-05-14, wiki/plans/semantic-hybrid-search.plan.md, wiki/proposals/search-model-selection.proposal.md, wiki/references/llm-search-model-licensing.reference.md, src/install.rs, src/search_models.rs
- Related: wiki/decisions/semantic-hybrid-search-mode.decision.md, wiki/plans/project-registry-search-artifacts.plan.md, wiki/references/llm-search-model-licensing.reference.md, wiki/proposals/search-model-selection.proposal.md, wiki/specs/documentation-model.spec.md

## Deliverable

`llm-wiki install` becomes idempotent for search model artifacts. When the
selected LLM search profile's required models already exist under
`~/.llm_wiki/models/`, match the embedded catalog hashes, and have current
accepted-license records, a repeated install must skip network downloads. It
may repair or rewrite control-plane records such as `search.toml`,
`accepted-licenses.toml`, and `models/artifacts.toml`, but it must not fetch
unchanged model bytes.

`llm-wiki install` and `llm-wiki install --configure-search` should replay the
current installed search values as defaults, matching the rerun-init UX:
current posture, current profile, and current model state are visible and
editable rather than treated as a fresh setup every time.

`llm-wiki install --configure-search` remains the interactive search posture
surface. It must allow switching from semantic/hybrid LLM search to lexical-only
search and back again. Switching to lexical-only writes a disabled search
profile immediately, so search and index commands stop using LLM model
artifacts. Managed model files and semantic indexes are large rebuildable
artifacts, but install/configure must not delete them. If disabling LLM search
leaves removable artifacts behind, install should report that fact and point to
an explicit cleanup command, not perform deletion itself.

Cleanup belongs to the uninstall/cleanup command surface. Full `llm-wiki
uninstall` should remove global framework-owned state, including model files,
search indexes, accepted-license records, search config, registry state,
installed skills, and optionally the managed binary when `--include-binary` is
present. A narrower cleanup flag or sibling command, such as `llm-wiki
uninstall --models` or `llm-wiki uninstall --search-artifacts`, should remove
only downloaded model files and search indexes without rewriting install
configuration. The targeted search-artifact uninstall must refuse when the
active global search config still enables LLM search unless the user passes
`--force`; the forced path is an explicit break-glass cleanup and can leave
semantic/hybrid readiness broken until search is reconfigured. The exact flag
name is a pre-implementation naming decision.

This is a new uninstall decision that supersedes the older D9 registry/search
artifact plan clause saying `llm-wiki uninstall` must not remove the registry,
search indexes, or model cache. The implementation must update that completed
plan or promote this behavior to a decision before code lands.

The visible behavior should distinguish these cases:

1. All required models are already installed and verified: skip download
   confirmation and report reuse.
2. Some models are missing: ask consent for and download only the missing
   artifacts, while reusing verified artifacts.
3. All required models are installed and verified, but accepted-license records
   are missing or stale: prompt only for license/terms acknowledgement and do
   not describe the action as a download.
4. A managed artifact exists but has the wrong hash: fail unless `--force`; with
   `--force`, replace only the mismatched artifact.

## Problem

The current installer has a reuse check inside `materialize_model`, but the
interactive install flow still frames every enabled search install as a download
operation. A user running install twice can be asked to download the same large
models twice, which violates the managed-artifact expectation and makes
semantic/hybrid setup feel unsafe.

The accepted model contract says model bytes are consented install-time
downloads, stored under `~/.llm_wiki/models/`, pinned to immutable revisions,
verified by SHA-256, and recorded in managed state. Repeated install should use
that state as the source of truth before attempting network work.

## In Scope

- Classify every model required by the selected profile as verified, missing,
  or hash-mismatched before prompting for downloads.
- Treat accepted-license records as model-specific readiness state: model id,
  license, and terms URL must match the current catalog entry.
- Reuse verified files even when `models/artifacts.toml` is missing or stale,
  then rewrite the artifacts record from verified local bytes.
- Download only missing artifacts.
- Preserve the existing fail-closed behavior for hash mismatches unless
  `--force` is present.
- Make `--force` replace only artifacts that are missing or invalid; it should
  still reuse valid artifacts.
- Add diagnostics and tests that prove the second install path does not invoke
  the downloader.
- Treat disabling LLM search as an active profile change: write disabled
  `search.toml`, skip semantic readiness, invalidate active license acceptance
  for disabled LLM search so re-enabling prompts again, and print explicit
  cleanup guidance when model/index artifacts remain.
- Add or update an explicit uninstall/cleanup command surface for removing
  global framework-owned model and search artifacts.
- Make full uninstall remove global framework-owned state as a new contract,
  superseding the older registry/search artifact plan.
- Update wiki docs after implementation if the final behavior differs from this
  plan.

## Out Of Scope

- Search command model selection or `--choose-model` pending-state promotion.
- New model profiles beyond the existing balanced profile.
- Changing the model catalog, hashes, licenses, or download URLs.
- Runtime search/index commands downloading models.
- Deleting model files or semantic indexes from install/configure merely
  because the active profile changes to lexical-only.
- Deleting project-local `.llm_wiki/` folders or project wiki content during
  global uninstall or model cleanup.
- Moving model artifacts out of `~/.llm_wiki/models/`.

## Implementation Touchpoints

- `src/install.rs`: `configure_enabled_search` owns the interactive profile
  flow, consent text, artifact loop, and search profile writes.
- `src/search_models.rs`: `materialize_model`, `sha256_file`,
  `ModelArtifacts`, and `AcceptedLicenses` own model verification and artifact
  records.
- `tests/install.rs`: redirected-`HOME` install tests cover installer behavior.
- `tests/search_commands.rs` and `tests/eval_commands.rs`: existing fake
  artifact helpers show the expected `models/artifacts.toml` and license record
  shape.
- `src/uninstall.rs`, `src/cli.rs`, `src/main.rs`, and `src/paths.rs`:
  uninstall command shape, force/include flags, dispatch, managed paths, and
  safety roots for targeted cleanup.
- `src/registry/mod.rs`: registry path and deletion semantics for full
  uninstall.
- `tests/install.rs`, `tests/post_install.rs`, `tests/status_doctor.rs`, and
  `tests/registry.rs`: install/uninstall/registry redirected-`HOME` behavior.

## Phase 1 - Classify Existing Model State

Add a pure model-state classification API near `materialize_model`. The API
must accept an injectable catalog/model list for tests so unit tests can use
tiny fixture files instead of the real multi-hundred-megabyte model hashes.

```text
Verified { record }
Missing { path }
HashMismatch { path, observed_sha256 }
```

The classifier should:

1. Compute the managed path from the catalog model id and file name.
2. If the file does not exist, return missing.
3. If the file exists, hash it.
4. If the hash matches the catalog expected SHA, return a fresh artifact record
   built from the current catalog and local bytes.
5. If the hash differs, return hash mismatch.

License acceptance stays separate from file classification, but install should
check it before declaring the full profile already installed.

Verification:

- Unit tests cover missing, verified, and mismatched files using tiny fixture
  models through the injectable catalog/model input.
- Verified classification does not require an existing `models/artifacts.toml`.

## Phase 2 - Make Materialization Outcome Explicit

Change `materialize_model` so callers can tell whether work was reused or
downloaded:

```text
MaterializedModel {
  record,
  outcome: Reused | Downloaded,
}
```

The function should reuse the Phase 1 classifier before any downloader call.
When the file is verified, it returns `Reused`. When missing, it downloads and
returns `Downloaded`. When mismatched, it fails unless `force` is true; with
`force`, it downloads and returns `Downloaded`.

Verification:

- A unit test proves a verified file returns `Reused`.
- A unit test proves a mismatched file fails without `force`.
- A unit test proves `force` replaces a mismatched file.

If direct network isolation is awkward, split the downloader into an injectable
function inside `search_models.rs` for tests. Keep that abstraction private to
the module unless another caller genuinely needs it.

For install-level behavior, do not drive `inquire` TTY prompts from ordinary
command tests. Extract a pure install-planning function that accepts current
search config, accepted-license state, artifact classifications, and prompt
decisions, then returns planned actions: reuse, prompt-license-only, download,
repair-records, disable-search, or print-cleanup-guidance. Integration tests
should cover CLI wiring with non-download paths; pure tests cover the state
matrix.

## Phase 3 - Skip Download Prompt When Complete

Update `configure_enabled_search` to inspect the selected profile before it
prints download instructions:

1. Load current accepted-license records.
2. Classify all models required by the selected profile.
3. If every required model is verified and every required license record is
   current, skip the download consent prompt.
4. Print a concise reuse message and write fresh control-plane records.
5. If some models are missing, list only those under "Models to download and
   verify".
6. If models are verified but license records are missing or stale, prompt for
   license/terms acknowledgement only; do not say models will be downloaded.
7. If some models are already verified, optionally list them under a separate
   reused section in verbose diagnostics rather than noisy normal output.
8. If any model is hash-mismatched and `--force` is absent, fail before asking
   for download consent.

The install flow should still record accepted licenses before any actual
download. When all models are already installed and current license records
exist, it can preserve or rewrite equivalent accepted-license records without
asking the user to re-acknowledge the same terms.

Verification:

- Install-level test seeds verified model files and current license records,
  runs the enabled install path, and asserts no downloader call occurred.
- Test stale or missing `models/artifacts.toml` with verified files and current
  licenses; install should repair the record without downloading.
- Test a missing model plus a verified model; only the missing model is
  downloaded.
- Test verified files plus missing/stale license records prompts for license
  acknowledgement without invoking the downloader or using download wording.

## Phase 4 - Preserve Force And Repair Semantics

Lock the edge cases:

1. `--force` does not blindly re-download every valid model.
2. `--force` does replace mismatched artifacts.
3. Missing or stale artifact records are repaired from verified files.
4. License record mismatch requires the consent path before the profile is
   treated as enabled.
5. Search config writes remain atomic and unchanged in shape.

Verification:

- Focused tests for force behavior and license mismatch.
- `git diff --check`.

## Phase 5 - Disable And Cleanup Command Semantics

Clarify and test the search-type transition path:

1. `install --configure-search` can switch an enabled semantic/hybrid profile
   to lexical-only/no-LLM search.
2. The disabled profile is written as the complete install/configure action.
   Search behavior changes immediately, and semantic/hybrid readiness stops
   consulting model artifacts.
3. Model artifacts under `~/.llm_wiki/models/` and semantic indexes under
   `~/.llm_wiki/indexes/` are not removed by install/configure.
4. When disable leaves model or semantic index artifacts present, install prints
   concise guidance for the explicit cleanup command.
5. The explicit targeted cleanup command removes only LLM-search-owned global
   state:
   - `~/.llm_wiki/models/`, including `models/artifacts.toml`
   - `~/.llm_wiki/accepted-licenses.toml`
   - semantic/vector sidecars named `semantic-index.json` and
     `semantic-vectors.json` under `~/.llm_wiki/indexes/`
   - model-scoped threshold records that cannot be valid without the removed
     artifacts, or the whole global threshold file if records are not yet
     individually removable
6. Targeted search-artifact cleanup does not remove lexical qmd-rs stores by
   default, does not rewrite `search.toml`, and does not remove installed
   skills. It may include a later explicit broader cache flag, but that is
   outside this plan.
7. If either global search config section, `[project_default]` or
   `[global_search]`, still enables LLM search, targeted search-artifact cleanup
   refuses unless `--force` is present. With `--force`, it deletes the requested
   artifacts and leaves config unchanged; subsequent semantic/hybrid use must
   fail readiness until the user reruns `install --configure-search`.
8. Full `llm-wiki uninstall` removes global framework-owned state, including
   installed skills, manifest, backups, registry state at
   `~/.local/share/llm-wiki/projects.json`, search config, accepted licenses,
   model files, lexical and semantic indexes under the managed index root, and
   legacy cache roots owned by llm-wiki where they can be proven safe. It leaves
   project-local repos and project-local `.llm_wiki/` folders alone.
   `--include-binary` remains the explicit managed-binary deletion switch unless
   a later uninstall decision changes that.
9. The non-interactive `--disable-llm-search` path remains deterministic and
   does not prompt. It writes the disabled profile, removes active license
   readiness by invalidating it for the disabled profile, and prints cleanup
   guidance when artifacts remain.

Verification:

- Test enabled-to-disabled profile switch writes disabled search config.
- Test disabled switch does not delete model artifacts.
- Test the targeted cleanup command removes managed model/index artifacts and
  leaves install manifest, managed binary, skills, registry, and wiki files
  untouched.
- Test targeted cleanup refuses while LLM search remains enabled, and that
  `--force` is required to delete artifacts in that state.
- Test targeted cleanup refuses when either `[project_default]` or
  `[global_search]` is enabled.
- Test targeted cleanup preserves lexical qmd-rs stores while removing semantic
  sidecars and model/license files.
- Test full uninstall removes global framework-owned runtime state while leaving
  project-local `.llm_wiki/` folders untouched.
- Test `--disable-llm-search` is non-interactive and does not delete artifacts.

## Phase 6 - Documentation And Regression Tests

After implementation, update the relevant durable docs:

1. Add the idempotent model-install rule to
   `wiki/references/llm-search-model-licensing.reference.md` or the promoted
   search install contract page.
2. Update `wiki/plans/project-registry-search-artifacts.plan.md` to mark its
   old "uninstall leaves registry/index/model cache untouched" clause as
   superseded by this new uninstall decision, or promote the new behavior to a
   decision and link both plans to it.
3. If the behavior changes user-facing install prompts, update the semantic
   search plan or any accepted decision that describes install posture.
4. Append a wiki log entry with verification commands.

Full verification target:

```bash
cargo test install search_models --workspace
cargo test --test install
cargo clippy --all-targets --all-features -- -D warnings
git diff --check
```

## Acceptance Criteria

- Running enabled LLM-search install twice with unchanged verified model files
  does not download either required balanced-profile model on the second run.
- The second run still leaves `search.toml`, `accepted-licenses.toml`, and
  `models/artifacts.toml` present and current.
- `install --configure-search` can switch from semantic/hybrid to lexical-only
  search.
- Disabling LLM search stops LLM readiness and retrieval from using model
  artifacts immediately.
- Disabling LLM search invalidates active accepted-license state so re-enabling
  later prompts again. The implementation may do this by rewriting control-plane
  state rather than physically deleting `accepted-licenses.toml`; full uninstall
  or targeted cleanup owns file deletion.
- Model and semantic index artifacts are deleted only by an explicit cleanup or
  uninstall command, never as an install/configure side effect.
- Targeted search-artifact uninstall refuses while LLM search is enabled unless
  the user passes `--force`; enabled means either `[project_default]` or
  `[global_search]` is enabled.
- Full uninstall removes global framework-owned model/search/cache state while
  preserving project-local repositories and project-local `.llm_wiki/` folders.
- A missing model is downloaded without touching already verified models.
- A corrupt model is never silently used.
- Search, search-all, index, index-all, doctor, eval, and wiki-query remain
  non-download runtime surfaces.
- Runtime readiness may trust accepted artifact records plus file existence for
  performance. The mandatory SHA-256 byte check is install-time verification;
  `doctor` may add a deeper rehash diagnostic, but ordinary search/index
  readiness is not required to rehash model files on every run.
