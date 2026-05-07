# Plan: Project Registry and Search Artifacts

- Document Class: Plan
- Status: Completed
- Date: 2026-05-07
- Category: Search infrastructure, framework tooling
- Scope: Implement D9 user-visible project registration, indexing, project-local search, and explicit cross-project search commands on top of the qmd-rs backend adapter.
- Sources: wiki/proposals/project-registry-search-artifacts.proposal.md, wiki/plans/qmd-rs-search-backend.plan.md, wiki/decisions/search-backend-selection.decision.md, wiki/evals/search-backend-selection.eval.md, wiki/specs/documentation-model.spec.md
- Related: wiki/proposals/project-registry-search-artifacts.proposal.md, wiki/plans/qmd-rs-search-backend.plan.md, wiki/roadmaps/framework-v1.roadmap.md

## Deliverable

D9 adds a host-local project registry and search command surface to
`llm-wiki`.

The qmd-rs backend is already behind an internal adapter. This plan connects
that adapter to user-visible commands while preserving the framework boundary:
markdown under `wiki/` remains canonical, search stores are rebuildable caches,
and cross-project retrieval is explicit through `search-all`.

## In Scope

- Add host-local project registry persistence at
  `~/.local/share/llm-wiki/projects.json`.
- Add project registration commands: `register`, `forget`, and `projects`.
- Add search lifecycle commands: `index`, `index-all`, `search`, and
  `search-all`.
- Add `init --no-register` and default auto-registration after successful
  project scaffolding.
- Reuse the qmd-rs adapter from `src/search/` for project-local indexing and
  search.
- Preserve the qmd-rs Cargo feature gate unless a separate release decision
  enables it by default.
- Add text and JSON output formats for project and search results.
- Add class/status filtering using framework metadata.
- Add explicit cross-project result fusion with per-project retrieval caps and
  reciprocal rank fusion.
- Update `doctor` to prefer registry-backed project IDs when available.
- Update specs after commands are implemented and verified.
- Define and test default-release behavior for search commands while qmd-rs
  remains feature-gated.

## Out Of Scope

- Answer synthesis or a `query` command.
- Automatic use of `search-all` by `knowledge-query`.
- Indexing `raw/` by default.
- Enabling qmd-rs by default in release builds without a separate release
  decision.
- Semantic/hybrid model downloads or automatic model setup.
- Direct SQLite FTS5 fallback implementation unless the documented fallback
  trigger fires.
- Cross-machine registry sync.
- Self-update or binary installer changes.

## Existing Implementation Touchpoints

Inspect these first:

- `src/cli.rs` and `src/main.rs`: command registration and dispatch.
- `src/init/`: successful scaffold path and new `--no-register` integration.
- `src/paths.rs`: managed runtime paths plus cache/index/model helpers.
- `src/search/`: qmd-rs adapter, metadata parser, sanitizer, CWD discovery.
- `src/doctor.rs`: current install/current-project/search-index/semantic-model
  diagnostic sections.
- `tests/status_doctor.rs`, `tests/init.rs`, `tests/install.rs`: redirected
  `HOME`, `TempDir`, command assertion, and no-user-home-write patterns.
- `justfile`: use `just verify` as the main pre-commit gate.

## Data Model

Registry path:

```text
~/.local/share/llm-wiki/projects.json
```

Initial schema:

```json
{
  "version": 1,
  "projects": [
    {
      "id": "llm-wiki-framework",
      "name": "LLM Wiki Framework",
      "root": "/absolute/project/root",
      "wiki_path": "wiki",
      "registered_at": "2026-05-07T00:00:00Z",
      "last_indexed_at": null,
      "last_indexed_wiki_max_mtime": null,
      "indexed_file_count": 0,
      "backend": "qmd-rs",
      "index_schema_version": 1
    }
  ]
}
```

Registry rules:

1. Registry state is host-local tool state, not project knowledge.
2. Registry writes must be atomic.
3. Absolute project roots are stored; missing roots are reported by `projects`
   and `doctor`.
4. Project IDs are stable local IDs:
   - explicit `--id` wins;
   - otherwise slugified `--name`;
   - otherwise slugified project directory basename;
   - collisions append `-2`, `-3`, and so on.
5. Canonical project roots are unique by default. Re-registering the same
   canonical root returns or updates the existing registry entry instead of
   creating a duplicate.
6. `--id` may intentionally name a same-root registration only if it passes an
   explicit conflict check and does not create duplicate `search-all` results by
   default. V1 should reject same-root/different-ID duplicates unless a later
   use case proves they are needed.
7. `llm-wiki uninstall` must not remove the registry, search indexes, or model
   cache.

## Default-Release Contract

qmd-rs remains feature-gated until a separate release decision enables it by
default. D9 can still ship command plumbing in default binaries only if the
default behavior is explicit and tested:

1. `register`, `forget`, and `projects` must work in default builds because
   they do not require the qmd-rs backend.
2. `index`, `index-all`, `search`, and `search-all` must fail clearly in
   default builds with a qmd-rs-feature-disabled diagnostic. They must not
   pretend that search is available.
3. `doctor` must report the registry state and the qmd-rs feature-disabled
   search state separately.
4. Feature-enabled builds must pass the full search behavior tests with
   `cargo test --workspace --features qmd-rs`.

D9 may be marked Completed with qmd-rs still feature-gated only if both default
diagnostic tests and feature-enabled behavior tests pass. Enabling qmd-rs by
default remains a separate release decision.

## Command Surface

### `register`

```text
llm-wiki register <path> [--name <name>] [--id <id>]
llm-wiki register --update <id> <path>
```

Registers or updates a framework-shaped project. Validation requires
`wiki/index.md`, `wiki/log.md`, and at least one orientation file from the
recognized set: `project_guidelines.md`, `CLAUDE.md`, or `AGENTS.md`.

Registering the same canonical root twice is idempotent: it reports the
existing project ID and updates mutable fields such as name when explicitly
provided. It must not create duplicate entries.

### `forget`

```text
llm-wiki forget <project-id> [--delete-cache]
```

Removes one registry entry. `--delete-cache` also deletes that project's
rebuildable index directory and leaves project files untouched.

### `projects`

Lists registered projects with ID, name, root, index status, missing-root
status, stale/fresh state, backend, and cache size when known. Support
`--format text|json`.

### `index`

```text
llm-wiki index [--project <id>] [--force]
```

Indexes the current project by default. Outside a framework-shaped project,
`--project <id>` is required. Indexing writes through the internal search
adapter and updates registry staleness fields after success.

### `search`

```text
llm-wiki search "query" [--project <id>] [--class <type>] [--status <status>] [--limit <n>] [--format text|json]
```

Retrieves ranked project-local results. It must never cross project boundaries.

Missing indexes are not searchable: print clear guidance to run
`llm-wiki index` and return a nonzero exit for text and JSON modes.

Stale indexes are searchable-with-warning. Text output prints a stale-index
warning before results. JSON output includes the freshness marker on every
result and a top-level warning field. This preserves retrieval during active
work while making staleness visible to agents and scripts.

### `index-all`

Reindexes all registered projects one at a time. A failure for one project is
reported without corrupting other project indexes.

### `search-all`

```text
llm-wiki search-all "query" [--include <id> ...] [--exclude <id> ...] [--class <type>] [--status <status>] [--limit <n>] [--format text|json]
```

Retrieves across explicitly registered projects. Every result includes project
identity. If neither `--include` nor `--exclude` is present, all registered
projects are searched. The current project gets no ranking boost.

Default fusion:

1. retrieve top 20 per selected project;
2. fuse globally with reciprocal rank fusion using `k=60`;
3. truncate to `--limit` after fusion.

## Output Contracts

Text output should be compact, deterministic, and citation-friendly.

Search JSON objects must include:

1. project ID
2. project name
3. canonical wiki file path
4. title
5. document class
6. status
7. score
8. snippet
9. backend
10. mode
11. freshness marker

Do not expose qmd-rs docids or virtual paths as canonical citations.

## Phases

### 0. Registry foundation

1. Extend `src/paths.rs` with `data_home` and registry path helpers:
   `~/.local/share/llm-wiki/projects.json` by default, respecting relevant
   platform conventions where already modeled.
2. Add `src/registry/` with schema structs, atomic read/write, project
   validation, slug/collision handling, and cache path lookup.
3. Add unit tests for ID generation, collision suffixes, validation failures,
   same-root idempotency, atomic persistence, redirected `HOME`, and missing
   registry behavior.

Verification:

- `just verify` passes.
- Registry tests do not write outside redirected temp homes.

### 1. Register, forget, and projects commands

1. Add CLI variants and command modules for `register`, `forget`, and
   `projects`.
2. Implement text and JSON output for `projects`.
3. Ensure `forget --delete-cache` removes only rebuildable cache artifacts.
4. Add integration tests for command behavior and diagnostics.

Verification:

- Registering a fixture project creates `projects.json`.
- Registering the same canonical root twice does not create duplicate entries.
- Invalid paths are refused with clear diagnostics.
- Updating a moved project preserves the ID.
- Forget leaves project files intact.

### 2. Index command

1. Wire `llm-wiki index` to the qmd-rs adapter.
2. Resolve the selected project from CWD or `--project`.
3. Add per-project lockfile behavior.
4. Build indexes through a temporary store and atomically promote on success.
5. Update registry staleness metadata after successful indexing.

Verification:

- `index` builds a qmd-rs store from `wiki/**/*.md`.
- Re-running `index` is idempotent.
- `index --force` rebuilds.
- A simulated index failure leaves the previous usable index in place.

### 3. Project-local search command

1. Wire `llm-wiki search` to the adapter.
2. Add `--project`, `--class`, `--status`, `--limit`, and
   `--format text|json`.
3. Report missing/stale indexes with guidance.
4. Add integration tests for text output, JSON output, filters, stale markers,
   feature-disabled diagnostics, and no boundary crossing.

Verification:

- Fixed eval queries pass through the user-visible `search` command for a
  registered project.
- JSON output is stable and includes all required fields.

### 4. Init auto-registration

1. Add `llm-wiki init --no-register`.
2. Auto-register newly scaffolded projects by default after successful init.
3. Print the generated project ID.
4. Ensure init failure does not write a registry entry.
5. If scaffolding succeeds but registry write fails, leave the scaffolded files
   in place, print a warning with the failed registry path, print the manual
   `llm-wiki register <path>` recovery command, and return success. The project
   exists; registration is recoverable tool state.

Verification:

- Existing init snapshots are updated intentionally.
- `init --no-register` leaves the registry untouched.
- Successful init creates a valid registry entry.
- Registry-write failure after successful scaffolding is tested as a
  partial-success warning, not a scaffold rollback.

### 5. Index-all and search-all

1. Add `index-all` over all registry entries.
2. Add `search-all` with `--include`, `--exclude`, filters, limit, and formats.
3. Implement RRF fusion with `k=60` and top-20 per-project retrieval.
4. Add two-project integration fixtures proving labels, filters, include,
   exclude, stale handling, and no current-project ranking boost.

Verification:

- `search-all` labels every result with project identity.
- Include/exclude flags constrain searched projects by ID.
- RRF behavior is deterministic in tests.

### 6. Doctor and docs

1. Update `doctor` to report registry file status, registered project count,
   missing roots, stale indexes, and qmd-rs feature state.
2. Update `wiki/specs/documentation-model.spec.md` so scale search points to
   `llm-wiki search` / `search-all` instead of direct QMD use.
3. Update skill specs only if the command guidance changes.
4. Append implementation measurements to the search eval if user-visible
   command replay produces new data.

Verification:

- `doctor` reports install, registry, current project, search index, and
  semantic model states without conflating them.
- `just verify` passes.
- `cargo test --workspace --features qmd-rs` passes.

## Acceptance Criteria

1. `register` records framework-shaped projects and refuses invalid paths.
2. Project IDs are deterministic and collision-safe.
3. Canonical roots are unique; repeated registration of the same root is
   idempotent and does not duplicate `search-all` results.
4. `register --update` updates moved project roots.
5. `forget` removes registry entries; `--delete-cache` removes only rebuildable
   search artifacts.
6. `projects` lists registered projects with missing/stale/fresh status and
   supports text and JSON output.
7. `index` builds per-project qmd-rs stores from `wiki/` only in
   feature-enabled builds and reports qmd-rs-feature-disabled in default builds.
8. `index` uses lock/temp/promote behavior so failed rebuilds do not corrupt
   previous indexes.
9. `search` retrieves project-local results and never crosses project
   boundaries.
10. `search` refuses missing indexes, searches stale indexes with explicit
    warnings/freshness markers, and reports qmd-rs-feature-disabled clearly in
    default builds.
11. `search` supports class/status filters and stable text/JSON output.
12. `init` auto-registers successful scaffolds by default and supports
    `--no-register`.
13. `init` treats registry-write failure after successful scaffolding as a
    recoverable partial-success warning with a manual register command.
14. `index-all` indexes registered projects independently.
15. `search-all` searches only registered projects, labels every result, and
    supports include/exclude filters.
16. Cross-project result fusion uses top 20 per project and RRF `k=60`.
17. `doctor` reports registry and search states clearly.
18. `uninstall` leaves registry, indexes, and model cache untouched.
19. `just verify` passes before completion.
20. `cargo test --workspace --features qmd-rs` passes before completion.

## Promotion Targets

When implemented and verified:

- Update `wiki/specs/documentation-model.spec.md` to make `llm-wiki search`
  and `llm-wiki search-all` the documented scale-search surface.
- Update `wiki/roadmaps/framework-v1.roadmap.md` with D9 proof and completion
  details.
- Update `wiki/evals/search-backend-selection.eval.md` with user-visible
  command replay if it changes measured behavior.
- Decide whether `knowledge-query` should mention `search-all` as explicit
  user-requested scope expansion. Do not make it automatic.

## Implementation Result

D9 was implemented in the Rust `llm-wiki` binary on 2026-05-07.

Implemented command surface:

- `register`, `forget`, and `projects` manage the host-local project registry
  at `~/.local/share/llm-wiki/projects.json`.
- `init` auto-registers successful scaffolds by default and supports
  `--no-register`.
- `index` and `index-all` build qmd-rs stores from `wiki/**/*.md` in
  feature-enabled builds and report qmd-rs-feature-disabled in default builds.
- `search` retrieves project-local results with class/status filters and
  text/JSON output.
- `search-all` searches explicitly registered projects, supports include/exclude
  filters, labels every result with project identity, and fuses per-project
  results with RRF `k=60`.
- `doctor` now reports install, registry, current-project, search-index, and
  semantic-model sections separately.

Verification:

- `just verify`
- `cargo test --workspace --features qmd-rs`

## Close Conditions

This plan was marked Completed when:

1. Registry commands are implemented and tested.
2. Index/search commands are implemented and tested.
3. Init auto-registration is implemented and tested.
4. `index-all` and `search-all` are implemented and tested with at least two
   registered projects.
5. Doctor reports registry and search states.
6. Specs, roadmap, index, and log are updated.
7. Default builds either provide clear feature-disabled diagnostics for
   search-backed commands or a separate release decision enables qmd-rs by
   default.
8. Required gates pass.
