# Plan: Init Rerun Schema Drift Recorded In Wiki

- Document Class: Plan
- Status: Completed
- Date: 2026-05-14
- Category: Tooling, project scaffolding, rerun observability
- Scope: Extend `llm-wiki init` rerun behavior so that when the resolved pack set or resolved pack composition changes, the rerun records schema drift in `wiki/log.md` and a minimal `wiki/index.md` drift section. Orphan-folder content stays on disk; users or future lint cleanup decide whether to archive, rehome, or remove it.
- Sources: src/init/scaffold.rs, src/init/answers.rs, src/init/manifest.rs, src/init/compose.rs, src/search/qmd_rs.rs (collect_wiki_documents), wiki/decisions/composable-project-init.decision.md, wiki/specs/wiki-init-skill.spec.md, wiki/plans/composable-project-init.plan.md
- Related: wiki/decisions/composable-project-init.decision.md (2026-05-13 rerun revision), wiki/specs/wiki-init-skill.spec.md (Rerun Behavior section)

## Deliverable

A rerun of `llm-wiki init` that changes the resolved pack set or changes the
resolved folder composition for the same pack set:

1. Reads the previous `.llm_wiki/init.toml` before overwrite to capture the old blueprint, pack list, and recorded resolved folders.
2. Compares the previous resolved schema to the current `compose` output. Pack-set changes and same-pack folder composition changes both count as schema drift.
3. Creates newly claimed folders and refreshes framework-owned schema files.
4. Appends a deterministic entry to `wiki/log.md` describing the drift: previous and current blueprint, added packs, removed packs, newly claimed folders, and folders the current schema no longer claims.
5. Updates `wiki/index.md` only by adding or refreshing a small schema-drift section. Existing catalog entries are preserved; orphaned entries are not silently removed.
6. Performs no filesystem deletion. Orphan folders and their content stay on disk. Markdown under `wiki/` remains searchable because `collect_wiki_documents` recursively scans `wiki/`; non-wiki orphan folders are preserved but are not part of wiki search unless a future indexer includes them.

Reruns where the resolved pack set and resolved folder composition are
identical to the manifest do not append a log entry and do not edit
`wiki/index.md`.

## Existing Implementation Touchpoints

Inspect these sites before changing code:

- `src/init/scaffold.rs` — `create_project` already reads `init_mode` and gates collision/preserve behavior. The drift audit mutation is a new step that must be computed before manifest overwrite and completed before the success print.
- `src/init/manifest.rs` — `InitManifest::read_from_project` is the right entry point for loading the previous manifest. Extend new manifests with the previous run's resolved folder list so future reruns can detect pack-catalog composition drift even when the pack set is unchanged.
- `src/init/compose.rs` — `compose` already produces the post-rerun `folders` and `resolved_packs`. The drift summary compares previous manifest packs/folders to `output.resolved_packs` and `output.folders`.
- `src/init/answers.rs` — no pack-default behavior change; `prompt_pack_defaults` (the blueprint-aware preselect added 2026-05-14) already produces the right user-facing flow. If corrupted manifests are meant to be recoverable, this file must stop propagating `InitManifest::read_from_project` parse errors before scaffold can apply the "no drift entry" fallback.
- `src/init/command.rs` — no behavior change; the schema-drift audit is internal to scaffold.
- `tests/init.rs` — `init_rerun_refreshes_schema_and_preserves_wiki_content` is the closest existing rerun test. The new tests sit alongside and assert the log entry shape, index drift section shape, and no-drift no-op case.
- `wiki/specs/wiki-init-skill.spec.md` — Rerun Behavior section must describe the schema-drift audit contract.
- `wiki/decisions/composable-project-init.decision.md` — 2026-05-13 dogfooding revision already names rerun as a narrow edit surface. Add a one-paragraph 2026-05-14 revision recording the schema-drift audit behavior.

## Implementation Clarifications

- The log entry is appended, never inserted. `wiki/log.md` is preserved across rerun, so the append is concatenation onto whatever the user has there.
- The drift entry uses a fixed structure so future tooling (e.g. `wiki-lint`) can parse it without a markdown DOM:
  ```
  ## [YYYY-MM-DD] init | schema drift | <old-blueprint> -> <new-blueprint>

  Added packs: <comma-separated, or "none">
  Removed packs: <comma-separated, or "none">
  Trigger: <pack-set change, composition change, or both>
  Added folders: <comma-separated relative paths newly claimed by the current schema, or "none">
  Orphaned folders: <comma-separated relative paths no longer claimed by the current schema, or "none">
  Index update: schema drift section refreshed; existing catalog entries preserved

  Orphan content is preserved on disk. Markdown under wiki/ remains searchable; non-wiki folders are preserved but not indexed by wiki search.
  ```
  The heading date matches the same date source the rest of init uses today (filtered to `[date]` in snapshots).
- "Added folders" means folders that were not recorded in the previous resolved folder set. "Orphaned folders" means folders that were recorded previously and are not claimed by the current `compose` output. Both are computed against schema intent, not disk state, so an orphaned folder may already have been deleted manually by the user.
- The change-detection predicate is `(old pack set != new pack set) OR (old resolved folders != new resolved folders)` using set equality on `Pack` values and folder strings. Pure blueprint-only changes that do not change packs or folders are not drift for the purposes of this entry.
- Manifests written by this plan record `resolved_folders = [...]` in addition to the existing setup answers. For legacy manifests without `resolved_folders`, rerun can still detect pack-set drift and can seed the new field, but same-pack pack-catalog drift is only detectable after one manifest exists with recorded resolved folders.
- When drift is detected, `wiki/index.md` receives or refreshes a small `## Schema Drift` section with the latest drift date, newly claimed folders, orphaned folders, and a pointer to `wiki/log.md`. Existing page entries remain byte-for-byte unless they are inside that generated drift section.
- `wiki/index.md` is a project-knowledge catalog, not the qmd-rs search manifest. Search staleness comes from per-file wiki snapshots, including content hash and modified time; the plan must not rely on parent `wiki/` directory mtime.
- To avoid losing drift evidence, compute the drift before overwriting `.llm_wiki/init.toml`, append/update audit entries before replacing the manifest, and make the audit write idempotent for an exact already-present latest entry.

## In Scope

- Reading the previous manifest in `scaffold.rs` before overwrite.
- Extending new init manifests with `resolved_folders`.
- Computing the pack/folder diff against the post-rerun `compose` output.
- Appending a structured entry to `wiki/log.md` when the diff is non-empty.
- Minimally adding or refreshing a `wiki/index.md` schema-drift section while preserving existing catalog entries.
- Integration tests for: pack-set drift case, same-pack folder-composition drift case, no-drift case, missing-previous-manifest/fresh-init case, legacy-manifest-without-resolved-folders case, and search staleness caused by changed wiki files rather than parent-directory mtime.
- Spec update in `wiki/specs/wiki-init-skill.spec.md` Rerun Behavior section.
- Decision revision in `wiki/decisions/composable-project-init.decision.md` recording the 2026-05-14 schema-drift audit behavior.
- Log entry on completion noting the change.

## Out Of Scope

- Deleting orphan folders or files from disk. Rerun stays additive.
- Rewriting or pruning the existing `wiki/index.md` catalog. Init may only own the generated schema-drift section; index cleanup remains a manual or future `wiki-lint --fix orphan-folders` operation.
- Dropping existing index entries just because their folders are no longer claimed by the current schema.
- Teaching the indexer to filter by pack membership. Markdown orphan content under `wiki/` stays searchable; non-wiki orphan folders remain preserved but outside wiki search.
- A `wiki-lint` rule for orphan folders. That is a separate proposal; this plan only writes the breadcrumb.
- Reflowing or sectioning `wiki/log.md`. Append-only.
- A user-facing opt-out flag. The audit entry is always written when the diff is non-empty.

## Phases

### 1. Capture previous manifest

1. In `scaffold.rs`, before `write_manifest` overwrites `.llm_wiki/init.toml`, read the previous manifest into an `Option<InitManifest>`. Keep this in a local; do not pass it through public types.
2. Extend `InitManifest` with `resolved_folders: Vec<String>` using a serde default so legacy manifests parse.
3. On `InitMode::Fresh` the previous manifest is always `None`. On `InitMode::Rerun` it should be `Some` for any project written by the rerun-aware framework; if recoverable corrupted-manifest behavior is desired, `answers.rs` must also stop propagating parse errors before scaffold can treat the previous manifest as missing.

### 2. Diff and format

1. Add a private drift helper in `scaffold.rs` that accepts the previous manifest, current blueprint, current packs, and current folders. It returns `None` when there is no pack-set or folder-composition drift.
2. Diff packs as sets; diff folders as sets. Stable ordering: sort by name.
3. For legacy manifests with no recorded folders, reconstruct the previous folder set from the previous manifest's blueprint/packs only as a best-effort fallback and mark same-pack composition drift as unavailable for that first rerun.
4. Format the log entry using the fixed structure above. The date comes from the same source the rest of init uses (today the snapshot harness filters `\d{4}-\d{2}-\d{2}` to `[date]`).

### 3. Audit and manifest write

1. After folder creation and framework-owned file refreshes succeed, but before replacing `.llm_wiki/init.toml`, apply the drift audit if the helper returned `Some`.
2. Open `wiki/log.md` in append mode and write the entry. Existing content is preserved (it is in `preserves_project_knowledge`, so the file already exists if it was preserved across rerun, or was just written if rerun is happening against an older project that lacked one).
3. The append needs a leading blank line if the existing file does not already end with `\n\n`. Use a small helper rather than reading the whole file; check the trailing bytes.
4. Add or refresh a generated `## Schema Drift` section in `wiki/index.md`. Preserve all existing page entries and sections outside that generated section.
5. Replace `.llm_wiki/init.toml` after the audit mutation so a failed audit does not cause the next rerun to miss the previous schema. Make the audit write idempotent for an exact already-present entry so a manifest failure after audit does not duplicate the breadcrumb on retry.

### 4. Tests

1. Add a `init_rerun_appends_schema_drift_log_and_index_entry` test in `tests/init.rs`. Init a generic project, rewrite `wiki/log.md` and `wiki/index.md` with user content (existing helper pattern), rerun with `--blueprint web-product --pack api`. Assert `wiki/log.md` ends with the drift entry, `wiki/index.md` preserves the seeded catalog content while adding the schema-drift section, the previous user log content is still present, and `.llm_wiki/init.toml` shows the new pack list plus resolved folders.
2. Add a same-pack composition drift test by seeding `.llm_wiki/init.toml` with the same packs but an older `resolved_folders` list. Rerun with identical args and assert drift is recorded because the folder composition changed.
3. Add a `init_rerun_with_identical_answers_and_composition_does_not_drift_log_or_index` test. Init then rerun with identical args; assert `wiki/log.md` and `wiki/index.md` equal the seeded user content with no appended drift entry or generated drift section.
4. Keep a legacy-manifest test where `resolved_folders` is absent. It should parse, seed the new field on write, and only record drift if the pack set changes.
5. Reuse `init_rerun_refreshes_schema_and_preserves_wiki_content` as a regression for preserved project knowledge; adjust it so `wiki/index.md` existing entries survive while the drift section is the only expected automatic index mutation.
6. For search, assert the backend reports staleness after the changed `wiki/log.md` / `wiki/index.md` file snapshots, not because of parent `wiki/` directory mtime.

### 5. Documentation

1. Update `wiki/specs/wiki-init-skill.spec.md` Rerun Behavior: state that schema drift is recorded in `wiki/log.md`, `wiki/index.md` gets a minimal schema-drift section while preserving existing entries, orphan content stays on disk, and search freshness is based on changed wiki markdown file snapshots.
2. Add a 2026-05-14 paragraph to the Dogfooding Revision section of `wiki/decisions/composable-project-init.decision.md`.
3. Append a completion entry to `wiki/log.md` (the framework wiki, not generated projects).
4. Update `wiki/index.md` to mark this plan Completed when the work lands.

## Verification Gates

1. `cargo test --workspace` is green, including the three new init rerun assertions.
2. `cargo insta test --accept` shows only the expected fresh-init snapshot
   churn from adding `resolved_folders` to `.llm_wiki/init.toml`.
3. A manual smoke: init a `generic` project, write a marker line into `wiki/index.md` and `wiki/log.md`, rerun with `--blueprint ml-research`, observe (a) the existing marker/catalog content in `wiki/index.md` is preserved, (b) `wiki/index.md` has a schema-drift section naming newly claimed and orphaned folders, (c) `wiki/log.md` ends with a `schema drift | generic -> ml-research` entry naming `ml`, `data`, `research`, `code` as added packs and listing the new folders, and (d) `llm-wiki search "schema drift"` can surface the changed log/index after reindex because those wiki markdown files changed.
4. Outside the generated schema-drift section, `wiki/index.md` is byte-identical between the pre-rerun and post-rerun state in the drift test.

## Completion Evidence

Completed on 2026-05-14. The implementation records `resolved_folders` in
fresh and rerun init manifests, computes rerun drift before manifest overwrite,
appends an idempotent structured `wiki/log.md` entry, and adds or refreshes only
the generated `## Schema Drift` section in `wiki/index.md`.

Verification passed with `cargo test --test init init_rerun`,
`cargo test -p llm-wiki-rs search::qmd_rs::tests::staleness_tracks_wiki_markdown_file_snapshots`,
`cargo insta test --accept`, `cargo test --workspace`, and
`cargo clippy --workspace --all-targets`. The manual smoke initialized a
generic project, preserved marker content in generated wiki files, reran as
`ml-research`, indexed the project, and confirmed `llm-wiki search "schema
drift"` returned both `wiki/log.md` and `wiki/index.md`.

## Pages To Update On Completion

- `wiki/plans/init-rerun-pack-drift.plan.md` — Status → Completed.
- `wiki/specs/wiki-init-skill.spec.md` — Rerun Behavior section reflects the schema-drift audit contract.
- `wiki/decisions/composable-project-init.decision.md` — Dogfooding Revision section gains a 2026-05-14 paragraph.
- `wiki/index.md` — entry status updated.
- `wiki/log.md` — completion entry.

## What Closes The Plan

The schema-drift integration tests pass, `wiki/index.md` preserves existing catalog entries while exposing schema drift, the spec's Rerun Behavior section names the log and index drift section as the audit trail, and a manual rerun produces parseable, byte-stable drift evidence.
