# Plan: qmd-rs Search Backend

- Document Class: Plan
- Status: Completed
- Date: 2026-05-07
- Category: Search infrastructure, framework tooling
- Scope: Implement the internal qmd-rs backend slice for D9 project-local search; user-visible D9 search commands and cross-project `search-all` remain later work.
- Sources: wiki/decisions/search-backend-selection.decision.md, wiki/evals/search-backend-selection.eval.md, wiki/proposals/project-registry-search-artifacts.proposal.md, wiki/references/qmd-rs-search-crate.reference.md
- Related: wiki/proposals/project-registry-search-artifacts.proposal.md, wiki/decisions/search-backend-selection.decision.md

## Deliverable

D9 backend slice: `llm-wiki` wraps qmd-rs behind an internal search adapter that
can index project wiki pages, run project-local searches through internal APIs,
return metadata compatible with the later D9 command surface, and report
backend health through `doctor`.

This plan executes the accepted qmd-rs backend decision. The broader D9 registry
and command surface remains owned by
`wiki/proposals/project-registry-search-artifacts.proposal.md` and any later D9
implementation plan.

This slice should land backend APIs and internal integration tests first. It may
extend the existing `doctor` command for project-aware diagnostics, but it must
not add user-visible `index`, `search`, or `search-all` command behavior before
the registry and D9 command-surface design lands.

## In Scope

- Add qmd-rs as the first concrete search backend behind an internal adapter.
- Index `wiki/**/*.md` only.
- Store or recover canonical relative wiki paths.
- Extract framework metadata: title, document class, status, source path,
  modified time, and body length.
- Sanitize user queries before qmd-rs FTS execution.
- Return stable project-local result fields compatible with future `search` and
  `search-all`: path, title, document class, status, score, snippet,
  adapter-computed match span when available, backend, and freshness marker.
- Add qmd-rs store lifecycle hooks needed by future per-project `index` and
  `search` behavior.
- Add `doctor` checks for qmd-rs stores, schema/version state, stale indexes,
  corrupt stores, and model/cache readiness for semantic modes.
- Keep direct SQLite FTS5 as an explicit fallback path if qmd-rs cannot ship
  safely.
- Add tests using the fixed eval query set.

## Out Of Scope

- Full D9 registry or command implementation, including user-visible `index`,
  `search`, and `search-all` commands.
- Cross-project `search-all`, reciprocal-rank fusion, or registry orchestration.
- Answer synthesis or a `query` command.
- Automatic model downloads without explicit user action or clear reporting.
- Embedding multi-gigabyte GGUF model files into the binary.
- Replacing `wiki/index.md` as the small-wiki orientation entry point.
- Shelling out to a separate search executable in D9 V1.

## Existing Implementation Touchpoints

At implementation start, inspect these sites first:

- `src/main.rs` and command modules: current CLI command registration pattern.
- `src/doctor.rs`: existing diagnostics shape and output style.
- `src/paths.rs`, `src/init/`, and `src/install.rs`: existing filesystem and
  managed-runtime patterns.
- `tests/`: redirected-`HOME`, tempdir, and command assertion patterns.
- `Cargo.toml` / workspace manifests: dependency placement and feature strategy.

The qmd-rs eval harness was temporary and lived under `/private/tmp/qmd-rs-eval`.
Do not treat it as production code, but reuse its findings: raw
`Store::search_fts` needs query sanitization for tokens like `qmd-rs` and
`search-all`.

## Target Module Shape

Treat search as a new subsystem instead of sprinkling qmd-rs calls through
`main.rs` or `doctor.rs`.

Initial module layout:

```text
src/search/
  mod.rs
  adapter.rs
  metadata.rs
  sanitize.rs
  project.rs
  qmd_rs.rs
```

Responsibilities:

- `adapter.rs`: backend trait, result types, filters, and status types.
- `metadata.rs`: wiki H1 and leading bullet-list metadata parsing.
- `sanitize.rs`: qmd-rs FTS query normalization.
- `project.rs`: CWD project discovery and project-key derivation until the
  registry lands.
- `qmd_rs.rs`: qmd-rs implementation behind the internal adapter.

## Historical (Superseded by Addendum)

Phase 0 implementation started on 2026-05-07.

Initial decision: qmd-rs remained feature-gated behind the Cargo feature
`qmd-rs`. Default builds did not compile or ship qmd-rs, llama.cpp, reqwest, or
rusqlite. When the feature was disabled, the internal adapter reported a stable
`FeatureDisabled` state and `doctor` printed that the qmd-rs backend feature was
disabled.

Post-D9 result: this implementation note was superseded by the default-on
qmd-rs release addendum in
`wiki/plans/project-registry-search-artifacts.plan.md`. The feature-gated
adapter was valid for the completed backend and D9 command implementation
slices, but qmd-rs now ships in normal default builds.

Measured dependency impact:

- Adding optional `qmd = 0.3.2` added 169 locked packages.
- The qmd-rs feature path includes `llama-cpp-2`, `llama-cpp-sys-2`, `reqwest`,
  `rusqlite`, `libsqlite3-sys`, and their TLS/platform dependencies.
- `cargo test --workspace --features qmd-rs` completed successfully after the
  initial compile; the first qmd-rs feature test build took about 1m06s.
- `cargo clippy --workspace --all-targets --features qmd-rs -- -D warnings`
  completed successfully; the first feature clippy pass took about 50s.
- `cargo build --release --features qmd-rs` completed successfully on the local
  host and took 1m32s.

Measured artifact impact:

- Default `just release-build` completed successfully for the local
  `aarch64-apple-darwin` artifact in 14.82s.
- Default dist binary size: 1.7 MB.
- Default compressed dist archive size: 530 KB.
- qmd-rs feature release binary size on the local host: 4.9 MB.
- After the default-on addendum, `llm-wiki install` uses the qmd-rs-enabled
  binary by default; any larger-download warning belongs to release polish, not
  the backend adapter slice.

Distribution status:

- `just release-plan` passed with network access and listed the four configured
  cargo-dist targets: `aarch64-apple-darwin`, `aarch64-unknown-linux-gnu`,
  `x86_64-apple-darwin`, and `x86_64-unknown-linux-gnu`.
- Local `just release-build` emitted the host `aarch64-apple-darwin` artifact.
  Full CI execution still needs to confirm all four target artifacts on their
  intended runners before qmd-rs can be enabled by default.
- Running `cargo dist build` directly failed because the installed command is
  `dist`, not the Cargo subcommand `cargo-dist`; the repository `just`
  wrappers are the correct release entry points.

License/distribution note:

- qmd-rs is recorded by the eval as `MIT OR Apache-2.0`, compatible with the
  workspace MIT license.
- Before enabling qmd-rs by default, re-check transitive license/distribution
  constraints for `llama-cpp-2`, `llama-cpp-sys-2`, llama.cpp/ggml linkage,
  SQLite linkage, and model download URIs.

## Adapter Contract

Define an internal adapter boundary before any CLI or diagnostic output code
reaches qmd-rs.

Required backend operations:

1. `index_project(project_id, wiki_root, store_path, options)`
2. `search_project(store_path, query, filters, limit)`
3. `status(store_path, wiki_root)`
4. `doctor(store_path, wiki_root, mode: SearchMode)`
5. `rebuild_or_recover(store_path, wiki_root, force)`

Required result fields:

1. project ID
2. project name when available
3. canonical wiki file path
4. title
5. document class
6. status
7. score
8. snippet
9. adapter-computed match span when available
10. backend name and mode
11. fresh/stale marker

The adapter should make qmd-rs an implementation detail. CLI output must not
expose qmd-rs docids or virtual paths as the canonical citation path.

`SearchMode` should be an enum, not a boolean. Initial variants: `Fts`,
`Semantic`, and `Hybrid`.

This backend slice does not require the full project registry to exist first.
Until registry IDs exist, project discovery is CWD-based: walk upward from the
current directory until a framework-shaped project root is found (`wiki/index.md`
and `wiki/log.md`). If no project root is found, `doctor` should skip
project-search checks and report that no current wiki project was detected. A
later registry implementation can replace the discovered project key with the
registered project ID.

Backend-slice store path:

```text
~/.cache/llm-wiki/indexes/<project-key>/qmd-rs.sqlite
```

For registry-backed projects, `<project-key>` is the registry project ID. Before
the registry lands, derive `<project-key>` from the canonical wiki root path with
a readable slug plus a stable short hash. This keeps the backend slice
unblocked while preserving compatibility with the proposed D9 cache model.

Required `src/paths.rs` additions before qmd-rs wiring:

1. `cache_home`: `XDG_CACHE_HOME/llm-wiki` when `XDG_CACHE_HOME` is set,
   otherwise `$HOME/.cache/llm-wiki`.
2. `index_root`: `<cache_home>/indexes`.
3. `model_cache`: `<cache_home>/models`.
4. `project_index_dir(project_key)`: `<index_root>/<project-key>`.
5. `qmd_rs_store_path(project_key)`:
   `<project_index_dir(project_key)>/qmd-rs.sqlite`.

These helpers must use the existing test path abstraction so redirected-`HOME`
tests stay isolated.

## Phases

### 0. qmd-rs dependency and feature spike

1. Add qmd-rs behind a Cargo feature in the smallest workspace scope that can
   support backend tests.
2. Confirm whether qmd-rs can be compiled behind an optional feature without
   breaking default builds.
3. Record build-time and dependency impact from `llama-cpp-2`,
   `llama-cpp-sys-2`, `reqwest`, and bundled SQLite.
4. Run a license/distribution review for qmd-rs, `llama-cpp-2`,
   `llama-cpp-sys-2`, llama.cpp/ggml linkage, bundled SQLite, and model URIs.
5. Confirm `cargo-dist` can still build all four release targets currently in
   scope.
6. Record release binary and installer size deltas.
7. For the initial feature-gated spike, keep stable feature-disabled stub
   behavior in the adapter: tests and diagnostics should report the backend as
   unavailable because the qmd-rs feature is disabled, not fail with missing
   symbols or partial command behavior.
8. Decide whether the first implementation enables qmd-rs by default or keeps it
   feature-gated during development.
9. Decide whether `llm-wiki install` needs a warning or confirmation when the
   managed binary becomes materially larger.

Verification:

- `cargo check --workspace` passes.
- `cargo dist build` or the equivalent release-target check passes for every
  supported target, or the plan records the blocked target and mitigation.
- Dependency, license, build-time, and artifact-size impact are recorded in
  this plan under Implementation Notes before broad command work starts.
- The initial spike required explicit acceptance in Implementation Notes before
  default-on qmd-rs if the release binary exceeded 100 MB, any compressed
  installer exceeded 75 MB, or any supported target's release build exceeded 15
  minutes in CI. The later default-on addendum superseded this gate after local
  release checks passed.

### 1. Search module, paths, and project discovery

1. Add the `src/search/` subsystem with adapter types, metadata parsing,
   sanitization, project discovery, and qmd-rs implementation modules.
2. Extend `src/paths.rs` with `cache_home`, `index_root`, `model_cache`,
   `project_index_dir`, and `qmd_rs_store_path`.
3. Implement CWD project discovery by walking upward for `wiki/index.md` and
   `wiki/log.md`.
4. Implement project-key derivation from the canonical wiki root path using a
   readable slug plus stable short hash.
5. Add redirected-`HOME` tests proving cache, index, and model paths stay inside
   the test home/cache.

Verification:

- Path helper tests pass with redirected `HOME` and `XDG_CACHE_HOME`.
- Project discovery finds this repo from a nested directory and reports no
  project from an unrelated tempdir.
- The feature-disabled adapter stub has stable, test-covered behavior.

### 2. Metadata extraction and query sanitization

1. Implement metadata parsing for the leading bullet-list metadata block under
   each wiki page H1: document class, status, date, category, scope, sources,
   and related paths. This is net-new wiki-page parsing. Do not reuse
   `crates/llm-wiki-schema/src/frontmatter.rs`, which parses YAML frontmatter
   for skill files.
2. Parse the H1 title, support continuation lines in metadata values, and treat
   missing optional metadata fields as absent rather than parse failures.
3. Implement qmd-rs FTS query sanitization for command-style tokens:
   `qmd-rs`, `search-all`, backtick-wrapped terms, paths, slashes, and quoted
   text.
4. Add unit tests using the eval queries that previously errored:
   `search backend selection qmd-rs eval` and
   `project registry search-all reciprocal rank fusion`.
5. Ensure sanitized queries preserve useful hyphenated terms where qmd-rs can
   support them safely.

Expected sanitizer examples:

| Raw query | FTS-safe query intent |
| --- | --- |
| `qmd-rs` | Search both `qmd` and `rs` without treating `rs` as a column. |
| `search-all` | Search both `search` and `all` without treating `all` as a column. |
| `` `llm-wiki search-all` `` | Strip command quoting and search `llm`, `wiki`, `search`, and `all`. |
| `wiki/proposals/search-backend-selection.proposal.md` | Search path terms without slash or dot parser errors. |

The adapter should preserve the original raw query for display, logging, and
snippet context even when qmd-rs receives the sanitized form.

Verification:

- Raw user search strings cannot produce qmd-rs FTS SQL/parser errors.
- Metadata extraction returns document class and status for all typed wiki pages
  in the current corpus.

### 3. qmd-rs store lifecycle

1. Use the backend-slice store path:
   `~/.cache/llm-wiki/indexes/<project-key>/qmd-rs.sqlite`.
2. Index `wiki/**/*.md` into qmd-rs stores with canonical relative paths.
3. Store enough adapter-owned metadata to avoid reparsing full documents on
   every result when practical.
4. Implement stale detection using file count, max mtime, backend name, and
   schema version.
5. Add force rebuild behavior.

Verification:

- Indexing this repo's `wiki/` creates a rebuildable store.
- Deleting the store and reindexing restores search.
- Stale status changes when a wiki markdown file changes.

### 4. Search result shaping

1. Map qmd-rs results into the stable D9 result object.
2. Fetch document bodies when needed to compute snippets and metadata.
3. Add snippet generation that is centered on the best available match.
4. Compute match spans in the adapter when possible. qmd-rs FTS results do not
   expose match spans directly.
5. Preserve canonical wiki paths for citations.
6. Apply class and status filters at the adapter or post-filter layer.

Verification:

- The fixed eval query set has no regression against the eval baseline: every
  fixed query stays in the top 2 after sanitization.
- Results include path, title, document class, status, score, snippet, backend,
  and freshness marker.
- No result exposes qmd-rs virtual paths as canonical wiki paths.

### 5. Doctor and model/cache reporting

1. Add project-aware doctor behavior for the backend slice using CWD project
   discovery, not the proposed registry.
2. Factor doctor output into distinct diagnostic sections: install, current
   project, search index, and semantic models.
3. Keep the existing managed-runtime install diagnostics and success wording
   scoped to the install section so search-index failures do not make install
   checks misleading.
4. Report missing, stale, corrupt, and schema-mismatched qmd-rs stores for the
   discovered project.
5. Report whether semantic-mode models are available locally.
6. Print explicit guidance for model download/setup without embedding model
   files or silently downloading them.
7. Distinguish FTS-ready from semantic-ready status.
8. If no project root is detected from CWD, leave existing managed-runtime
   doctor checks intact and report that project search checks were skipped.

Verification:

- `doctor` can report FTS-ready with no GGUF models.
- `doctor` can report semantic-not-ready with clear model/cache guidance.
- Corrupt or missing stores produce rebuild guidance.

### 6. Fallback guardrail

1. Do not implement a production SQLite backend preemptively.
2. Define the exact fallback trigger: packaging failure, unacceptable binary
   size/build complexity, qmd-rs API breakage, or model/cache behavior that
   cannot be made explicit enough for users.
3. Keep the adapter isolated enough that direct SQLite FTS5 can be implemented
   only if a fallback trigger fires.
4. Do not implement two full production backends unless the qmd-rs path fails.

Verification:

- The fallback trigger is documented before release.
- The qmd-rs adapter remains isolated enough that direct SQLite can replace it
  without rewriting the registry and command surface.

### 7. Integration tests and eval replay

1. Add tempdir wiki fixtures covering specs, decisions, proposals, plans,
   references, and evals.
2. Replay the fixed query set from
   `wiki/evals/search-backend-selection.eval.md`.
3. Add tests for query sanitization, metadata filters, stale detection, and
   doctor states.
4. Add at least one test proving offline FTS search works after indexing and
   without model files.
5. Keep tests on the internal adapter/API surface until the D9 registry and
   command surface lands.

Verification:

- `cargo test --workspace` passes.
- The fixed eval query set is recorded as passing according to the acceptance
  thresholds in this plan.

## Acceptance Criteria

1. qmd-rs is integrated behind an internal adapter, not called directly from CLI
   output code.
2. `wiki/**/*.md` can be indexed into a per-project qmd-rs store.
3. User queries with hyphens, slashes, backticks, and command-like terms are
   sanitized before qmd-rs FTS execution.
4. The fixed eval query set has no regression against the eval baseline: every
   expected target stays in the top 2 after sanitization.
5. Results include canonical path, title, document class, status, score,
   snippet, backend, and fresh/stale marker.
6. Class and status filters work against framework metadata at the adapter API.
   User-visible `--class` and `--status` CLI flags belong to the later D9
   command-surface implementation.
7. `doctor` distinguishes FTS-ready, semantic-ready, stale, missing, corrupt,
   and model-missing states.
8. FTS search works offline after indexing with no model files present.
9. Semantic model downloads are explicit or clearly reported; model files are
   not embedded in the binary.
10. Direct SQLite FTS5 fallback criteria are documented before the plan closes.
11. No user-visible D9 `index`, `search`, or `search-all` behavior is added by
    this backend-only slice.

## Promotion Targets

When implemented and verified:

- Do not update user-facing command docs to advertise `llm-wiki search` or
  `search-all` until the later D9 command-surface implementation lands.
- Update `wiki/proposals/project-registry-search-artifacts.proposal.md` or the
  D9 implementation plan with any command-surface changes discovered during
  backend implementation.
- Append measured post-implementation results to
  `wiki/evals/search-backend-selection.eval.md`.

## Close Conditions

This plan can be marked Completed when:

1. qmd-rs backend tests pass.
2. The fixed eval query set passes in the production adapter.
3. `doctor` reports the expected backend states.
4. Documentation and wiki bookkeeping are updated.
5. Any fallback decision is either documented as unnecessary or activated with a
   new decision/update.

## Completion Summary

Completed on 2026-05-07.

Implemented commits:

1. `ab00f6e` - Add search backend foundation.
2. `89f7f8d` - Implement qmd-rs search adapter.
3. `8e2bc40` - Cover search doctor diagnostics.
4. `1c9ff89` - Record qmd-rs implementation findings.
5. `edf7bec` - Replay search backend eval queries.
6. `4fbc792` - Record qmd-rs eval replay.

Closed behavior:

- Added `src/search/` with adapter, metadata parser, query sanitizer, project
  discovery, and qmd-rs backend modules.
- Added path helpers for cache, index, model, project-index, and qmd-rs store
  locations.
- Kept qmd-rs feature-gated behind `--features qmd-rs` with stable
  feature-disabled behavior in default builds.
- Implemented qmd-rs FTS indexing/search through the internal adapter, including
  metadata filters, snippets, stale detection, and canonical wiki paths.
- Extended `doctor` into install, current-project, search-index, and
  semantic-model sections.
- Replayed the fixed eval query set against the production adapter and recorded
  the result in `wiki/evals/search-backend-selection.eval.md`.

Verification:

- `just verify` passed.
- `cargo test --workspace --features qmd-rs` passed.
- `cargo clippy --workspace --all-targets --features qmd-rs -- -D warnings`
  passed.
- `just release-plan` passed with network access and listed all four configured
  cargo-dist targets.
- `just release-build` passed for the local `aarch64-apple-darwin` default
  artifact.
- `cargo build --release --features qmd-rs` passed on the local host.

Follow-up:

- User-visible registry, `index`, `search`, `index-all`, and `search-all`
  command behavior moves to
  `wiki/plans/project-registry-search-artifacts.plan.md`.
