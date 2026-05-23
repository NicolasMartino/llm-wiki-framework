# Plan: Sandbox-Safe Search Cache Reads

- Document Class: Plan
- Status: Active
- Date: 2026-05-23
- Category: Search infrastructure, sandboxed agents, qmd-rs adapter
- Scope: Implement immutable completed-store reads for qmd-rs-backed search,
  doctor, and registry status paths so read-only commands can inspect managed
  caches without mutating `~/.llm_wiki` state.
- Sources: wiki/proposals/sandbox-safe-search-cache-reads.proposal.md,
  wiki/decisions/search-backend-selection.decision.md,
  wiki/decisions/semantic-hybrid-search-mode.decision.md,
  wiki/checklists/observability-contract.checklist.md, src/search/qmd_rs.rs,
  src/search/adapter.rs, src/search/commands.rs, src/doctor.rs,
  src/registry/mod.rs, tests/search_commands.rs, tests/status_doctor.rs,
  local command reproduction 2026-05-23 after `llm-wiki index --force`,
  review findings 2026-05-23 on JSON failure envelopes, project-aware status,
  semantic/hybrid status paths, permission-preserving path checks, and strict
  candidate verification
- Related: wiki/proposals/sandbox-safe-search-cache-reads.proposal.md,
  wiki/plans/qmd-rs-search-backend.plan.md,
  wiki/plans/project-registry-search-artifacts.plan.md,
  wiki/plans/semantic-hybrid-search.plan.md,
  wiki/checklists/observability-contract.checklist.md

## Deliverable

`llm-wiki search`, `search-all`, `doctor`, and project-status reads can use a
completed managed qmd-rs cache when the process has read access to the cache
but no write access to the managed runtime home.

The implementation uses one permission-independent read path for completed
stores:

```text
SQLite URI: file:<store-path>?mode=ro&immutable=1
Open mode: read_only_immutable
```

This path is used for query and status reads in both writable and non-writable
cache environments. Read commands do not first attempt `qmd::Store::open`,
because that constructor is write-initializing in the current qmd-rs version.

Initial implementation choice: add an adapter-owned immutable SQLite read path
inside the llm-wiki qmd-rs adapter. Continue to use qmd-rs as the writer and
indexer. Do not wait for an upstream qmd read-only constructor before fixing
the dogfooding failure. If qmd later exposes a verified no-write read API, a
follow-up can replace the adapter-owned SQL path behind the same tests.

## Dogfood Evidence 2026-05-23

After this plan was created, the local managed binary reproduced the cache-read
failure on the current framework project:

- `/Users/nicolasmartino/.llm_wiki/bin/llm-wiki index --force` completed and
  reported `Indexed project: llm-wiki-framework-semantic-search (72 files)`.
- Immediately afterward,
  `/Users/nicolasmartino/.llm_wiki/bin/llm-wiki search --mode lexical --format json "sandbox safe search cache reads"`
  exited with `search index unusable for project
  llm-wiki-framework-semantic-search` and forced-reindex guidance.
- `llm-wiki doctor` reported the current qmd-rs FTS index as corrupt at
  `/Users/nicolasmartino/.llm_wiki/indexes/llm-wiki-framework-semantic-search/qmd-rs.sqlite`
  while semantic metadata and vectors existed for 411 chunks.
- `llm-wiki projects --format json` reported several existing registered
  projects, including the current project, as `index-unusable` with
  `qmd-rs store could not be opened`.

That evidence confirms the plan's implementation priority: the current read
path collapses qmd-rs open failures into corruption/force-reindex guidance even
after a fresh index run, so the first fix must separate immutable completed
reads, permission/access classification, and real corruption.

## In Scope

- Add read-path metadata to backend status, including `open_mode`.
- Add explicit backend states for `transient` and `permission_denied`.
- Make qmd-rs read/status APIs project-aware so metadata project/collection
  validation has the expected project id available.
- Make qmd-rs status metadata-first for completed stores.
- Open completed stores through immutable SQLite for lexical/status reads.
- Preserve qmd-rs as the write/index path.
- Add writer-side completed-store proof before live promotion.
- Treat mixed sqlite/metadata observations during promotion as retryable
  transient state, not corruption.
- Keep JSON stdout parseable for `search` and `search-all`.
- Report per-project cache access failures in `search-all` without dropping the
  whole JSON envelope.
- Update doctor and registry/project status labels so permission failures are
  not described as corruption or forced-reindex cases.
- Add verbose diagnostics for selected read path, store path, metadata
  classification, immutable open outcome, retry decisions, and refusal
  guidance.

## Out Of Scope

- Moving indexes into project roots.
- Implicit rebuilds from read commands.
- Downloading models, accepting licenses, or changing search profiles from
  `search`, `search-all`, `doctor`, or project-status commands.
- Fixing Metal, llama.cpp, embedding, query-expansion, or reranker runtime
  diagnostics.
- Replacing qmd-rs as the selected backend.
- Building a new project-update command.
- Switching qmd-rs promotion to a directory-pointer or sentinel publication
  model unless the file-by-file transient contract proves insufficient during
  implementation.

## Implementation Touchpoints

- `Cargo.toml`: add a direct `rusqlite` dependency if the adapter-owned SQL
  path is used. Pin to the qmd-rs-compatible version already present in
  `Cargo.lock` unless cargo resolution requires otherwise.
- `src/search/adapter.rs`: extend `BackendStatus` and `BackendState`.
- `src/search/adapter.rs`: carry the expected project id and read purpose into
  status/search calls, either by changing the trait signatures or by adding
  project-aware qmd-rs helper APIs used by every qmd-rs consumer.
- `src/search/qmd_rs.rs`: implement immutable completed-store status and
  lexical query reads, completed-store proof helpers, permission/corruption
  classification, and query parity tests.
- `src/search/commands.rs`: update index promotion, search retry behavior,
  JSON backend-status metadata, `search-all` per-project reports, and forced
  reindex routing.
- `src/doctor.rs`: report ready/stale/transient/permission-denied distinctly.
- `src/registry/mod.rs`: expose index labels and status messages for
  permission-denied and transient states.
- `tests/search_commands.rs`: add CLI integration coverage for immutable
  reads, JSON behavior, search-all partial failures, and promotion races.
- `tests/status_doctor.rs` and `tests/registry.rs`: add status/doctor/project
  label coverage.

## Phase 1 - Backend State Contract

Extend the backend status model before changing storage behavior.

1. Add `BackendState::Transient` for mixed sqlite/metadata observations during
   promotion or other retryable publication windows.
2. Add `BackendState::PermissionDenied` for access-policy failures where the
   store exists but cannot be read.
3. Add an open-mode field to `BackendStatus`, with at least:
   - `read_only_immutable`
   - `read_write_indexing`
   - `not_opened`
4. Add a read context that carries:
   - expected project/collection id
   - read purpose: live read/status vs pre-promotion candidate proof
   - store location: managed or legacy, when known by the caller
5. Update the `SearchBackend` trait or add project-aware qmd-rs helper methods
   so `status`, `doctor`, and `search_project` cannot validate metadata
   without knowing the expected project id. `StoreMetadata.project_id` must be
   compared against caller intent, not merely trusted because the file path
   was selected by convention.
6. Keep `Ready`, `Stale`, `Missing`, `Corrupt`, and `SchemaMismatch` semantics
   intact.
7. Update all state label helpers in search, doctor, and registry code.
8. Update `freshness_for_status` so only `Ready` is fresh, `Stale` is stale,
   and permission/transient/corrupt/schema/missing states remain unknown.

Verbose diagnostics must explain:

- selected project and store path
- chosen backend status path
- metadata classification
- immutable open success or failure class
- retryable transient observations
- permission-denied guidance

Tests:

- pure state-label tests for every new state
- verbose assertions beside changed search, doctor, and registry tests
- JSON stdout remains diagnostic-free

## Phase 2 - Immutable Completed-Store Reader

Implement a narrow immutable reader in `src/search/qmd_rs.rs`.

1. Read llm-wiki metadata before opening SQLite whenever the metadata file is
   expected to exist.
2. Classify a missing sqlite file or missing metadata during promotion as
   `Transient` when related store files or backups indicate publication is in
   flight; classify an ordinary absence as `Missing`.
3. Do not use lossy `Path::exists()`-style checks for readiness
   classification. Use `try_exists`, `fs::metadata`, metadata-file reads, and
   SQLite open/read attempts that preserve `PermissionDenied` and other
   filesystem error kinds for both the sqlite file and the llm-wiki metadata
   file.
4. Validate metadata backend, schema version, expected project/collection,
   file count, and wiki snapshot before reporting `Ready` or `Stale`.
5. Open the SQLite store with URI `mode=ro&immutable=1` and read-only flags.
6. Classify filesystem or sandbox access failures as `PermissionDenied`.
7. Classify SQLite parse/schema failures as `Corrupt` or `SchemaMismatch`.
8. Implement lexical query reads through the immutable connection.

The query SQL should be the smallest llm-wiki-owned equivalent of the qmd-rs
FTS query currently used by `Store::search_fts` and `Store::get_document`.
Before coding that SQL, inspect a generated qmd-rs fixture schema and keep the
adapter constants private to `src/search/qmd_rs.rs`. The query path must return
the same canonical wiki paths, titles, document classes, statuses, snippets,
and score ordering that existing lexical tests expect.

Tests:

- immutable open succeeds against a completed store when the cache directory
  and files are readable but not writable
- ordinary qmd-rs lexical search and immutable SQL search agree on top results
  for existing fixture queries
- a plain unreadable store reports `permission_denied`
- unreadable sqlite or metadata paths do not collapse to `missing`
- a malformed readable sqlite file reports `corrupt`
- stale metadata still reports `stale`, not permission or corruption
- metadata schema mismatch reports `schema_mismatch`
- mismatched metadata project id reports `schema_mismatch` or another explicit
  project-mismatch state, not `ready`

## Phase 3 - Completed-Store Proof And Promotion

Make writer commands prove a candidate store is immutable-readable before live
promotion.

1. After qmd-rs indexing finishes, close/drop the write store before proof.
2. Checkpoint or otherwise force the candidate main SQLite file to contain all
   committed index content needed by immutable readers.
3. Run immutable verification against the temp store in strict candidate-proof
   mode, with the expected project id and current wiki snapshot.
4. Fail the index command before promotion unless candidate proof reports a
   fully ready completed store for the expected project.
5. Keep the existing file-by-file promotion initially.
6. Teach readers to retry `Transient` observations in the same place they
   already retry the current missing/open-race window.
7. Keep rollback behavior for failed promotion and extend tests to include the
   metadata file and new transient status.

Do not claim atomic old-or-new visibility unless the implementation moves to a
true atomic sqlite+metadata publication unit. The first implementation may keep
file-by-file promotion if readers classify half-promoted states as retryable
`transient` and tests prove that behavior.

Candidate proof and live-read classification are separate contracts. The live
read path may classify mixed live files as retryable `Transient` because a
reader can race with promotion. The pre-promotion candidate proof must treat
missing metadata, mixed files, `Transient`, `PermissionDenied`, `Corrupt`,
`SchemaMismatch`, project mismatch, and stale snapshot observations as fatal.
A candidate build is not published until immutable verification proves the
candidate is complete and current.

Tests:

- candidate promotion refuses to publish when immutable verification fails
- candidate proof treats transient, missing metadata, permission denied,
  project mismatch, stale snapshot, corrupt, and schema mismatch as fatal
- `index --force` still restores the old live store on simulated promotion
  failure
- `search` racing with promotion retries transient state and then succeeds
- half-promoted sqlite+metadata observations are not reported as corrupt
- completed new stores are reported with `open_mode: read_only_immutable`

## Phase 4 - Search, Doctor, Registry, And JSON Behavior

Thread the new states through every read consumer.

1. `search_attempt` maps:
   - `Missing` to missing-index guidance
   - `Transient` to retry, then unavailable guidance
   - `PermissionDenied` to cache-read-access guidance
   - `Corrupt` and `SchemaMismatch` to force-reindex guidance
   - `Ready` and `Stale` to query execution
2. `perform_project_search` must not route `PermissionDenied` through the
   existing `ForceReindex` path.
3. Single-project `search --format json` must print a parseable JSON envelope
   for cache access failures before returning, rather than bailing before
   `print_search_json`. `PermissionDenied`, exhausted `Transient`, and other
   backend-readiness failures should appear as readiness/status metadata in
   the same JSON family used for existing readiness failures. The process may
   still exit non-zero, but stdout must remain structured JSON.
4. Single-project successful `search --format json` keeps the existing
   top-level fields and adds backend-status metadata, including `state`,
   `open_mode`, and a concise access or freshness message.
5. `search-all --format json` reports backend-status metadata per project.
   Per-project `permission_denied` and `transient` states become project
   warnings/readiness reports while other ready projects still contribute
   results.
6. `semantic_base_status`, `perform_semantic_project_search`, and hybrid
   lexical branch setup must use the same state mapping and JSON-envelope
   behavior as lexical search. Semantic/hybrid readiness checks must not bail
   with missing/force-reindex guidance before JSON shaping when the failure is
   a cache access, transient, permission, or completed-store status failure.
7. Human-readable `doctor` output distinguishes ready, stale, missing,
   transient, permission denied, corrupt, and schema mismatch.
8. `projects`/registry status exposes permission failures without labeling the
   index as corrupt or suggesting forced reindex for a healthy but inaccessible
   cache.

Tests:

- `permission_denied` guidance names read access to the managed cache, not
  `--force`
- corrupt/schema mismatch still uses force-reindex guidance
- `llm-wiki search --format json` returns parseable readiness/status JSON for
  single-project `permission_denied`
- `llm-wiki search --format json` returns parseable readiness/status JSON for
  exhausted single-project `transient`
- `search-all --format json` remains parseable with one inaccessible project
  and one ready project
- semantic and hybrid JSON searches preserve parseable readiness/status output
  for cache access failures before semantic runtime loading or fusion
- top-level and per-project JSON include `open_mode`
- text output remains concise and stdout is not contaminated by diagnostics

## Phase 5 - Observability And Documentation

Apply `wiki/checklists/observability-contract.checklist.md` explicitly.

Verbose diagnostics should answer:

- which cache root and store path were inspected
- whether the command used managed or legacy store location
- whether metadata was missing, mismatched, stale, or current
- which open mode was attempted
- whether the immutable open succeeded
- whether a transient publication observation was retried
- why a permission failure is not corruption

Documentation updates:

- Update the sandbox-safe proposal to accepted and link this plan.
- After implementation, update the qmd-rs/search decisions only with validated
  behavior.
- Update `wiki/specs/documentation-model.spec.md` and
  `wiki/specs/wiki-query-skill.spec.md` only if the user-facing query/search
  workflow changes.
- Keep `wiki/index.md` and `wiki/log.md` current for each wiki mutation.

## Verification

Minimum implementation proof:

```text
cargo fmt --check
cargo test --test search_commands
cargo test --test status_doctor
cargo test --test registry
cargo test --workspace
git diff --check
```

Manual proof:

```text
llm-wiki index --force
chmod -R a-w ~/.llm_wiki/indexes/<project-id>
llm-wiki search --mode lexical --format json "search cache reads"
llm-wiki doctor
```

Restore permissions after manual proof.

Acceptance proof:

- lexical JSON search succeeds against a completed managed qmd-rs cache with
  read access but no write access
- doctor and registry status report read-only completed caches as ready or
  stale
- true access failures report `permission_denied`
- permission failures never route to forced reindex guidance
- single-project `search --format json` emits parseable JSON on cache access
  failure
- promotion races report retryable transient states, not corruption
- `search-all --format json` remains parseable with per-project access
  failures
- adapter-owned immutable SQL maintains accepted qmd-rs lexical query parity
