# Sandbox-Safe Search Cache Reads

- Document Class: Proposal
- Status: Accepted
- Date: 2026-05-23
- Category: Search infrastructure, sandboxed agents, qmd-rs adapter
- Scope: Make read-only search and diagnostics work against managed search
  caches, for example `~/.llm_wiki`, without requiring cache write
  permissions.
- Sources: conversational input 2026-05-23,
  review reproduction and architecture notes 2026-05-23,
  wiki/decisions/search-backend-selection.decision.md,
  wiki/decisions/semantic-hybrid-search-mode.decision.md,
  wiki/proposals/project-update-command.proposal.md,
  wiki/specs/wiki-query-skill.spec.md
- Related: wiki/plans/sandbox-safe-search-cache-reads.plan.md,
  wiki/plans/qmd-rs-search-backend.plan.md,
  wiki/plans/project-registry-search-artifacts.plan.md,
  wiki/checklists/observability-contract.checklist.md

## Source Capture

This proposal currently cites conversational input plus review reproduction and
architecture notes because the issue was found while dogfooding
`llm-wiki search` from a sandboxed Codex session. The review evidence includes:

- qmd-rs `Store::open` against a chmod read-only cache fails with `attempt to
  write a readonly database`.
- SQLite `mode=ro` against a WAL-backed store on read-only media still fails
  with `attempt to write a readonly database`.
- SQLite `mode=ro&immutable=1` against the same store succeeds for ordinary
  reads.
- The immutable path must be the normal completed-store read path, not a
  fallback after a writable qmd-rs open fails, so query behavior and ranking do
  not depend on sandbox write permissions.
- A read-write to immutable cascade is useful as a diagnostic idea but is not
  acceptable as the normal read-command architecture because successful reads
  would still write in writable environments and could make behavior differ by
  permission context.
- Current promotion moves the SQLite file, metadata, WAL, and SHM files one at
  a time, and current comments already allow readers to observe transient
  missing state during promotion. Completed-store immutable reads therefore need
  an explicit writer-side completion proof and publication contract during
  implementation.

This proposal was accepted for implementation because the dogfooding failure
blocked the framework's search-first query workflow. The completed plan and
regression tests now act as the durable reproduction and implementation record
for the validated behavior.

## Question

Should `llm-wiki search`, `search-all`, `doctor`, and registry status checks
open existing qmd-rs search stores through an immutable read-only path so
sandboxed agents can use managed caches under the llm-wiki managed home, while
keeping cache writes owned by `index`, `index-all`, and future project-scoped
`update --reindex`?

## Observed Problem

The framework now asks `wiki-query` to attempt
`llm-wiki search --mode auto --format json` for every registered-project query.
That dogfoods search, but it exposes a mismatch between search's read intent
and the current qmd-rs store opening behavior.

The managed index lives under host-local state, commonly:

```text
~/.llm_wiki/indexes/<project-id>/qmd-rs.sqlite
```

On platforms that use a different managed application-data home, the same
problem applies to the equivalent managed index directory.

In a sandboxed agent environment, the project workspace can be readable while
`~/.llm_wiki` is outside the writable sandbox. Existing lexical search and
doctor checks can then report the qmd-rs index as corrupt or unusable, even
when the same store is healthy outside the sandbox. The issue is not that the
cache is in the wrong place. The issue is that read-only commands use an open
path that can attempt schema initialization, WAL setup, trigger creation, lock
or side-file writes, or other SQLite writes before the command does its real
read.

The current qmd-rs crate version used by the framework does not expose a
read-only store-open path. Its public store constructor is write-initializing:
it creates parent directories, opens SQLite with read-write/create behavior,
sets WAL mode, and creates schema objects before search can proceed. A plain
SQLite `mode=ro` open is also insufficient for this bug class because WAL
databases can still require writable access to WAL/SHM side files. For the
existing store format, sandbox-safe completed-store reads require immutable
SQLite URI semantics.

That turns an access-mode problem into a false cache-corruption diagnosis and
breaks the search-first `wiki-query` workflow.

## Proposal

Add a read-only qmd-rs adapter path for commands whose public contract is to
read an existing index. For the current WAL-backed qmd-rs store, that read path
must open completed, promoted stores with SQLite URI `mode=ro&immutable=1` or
an equivalent upstream helper with the same no-write behavior.

The immutable completed-store read path is the normal query and status path
regardless of whether the managed cache directory is writable. It must not be
implemented as a fallback that first attempts the write-initializing qmd-rs
store constructor and only switches to immutable SQLite after a permission
failure. Search behavior, result ranking, readiness labels, and diagnostics
must not vary based on whether the process happens to have cache write
permission.

This proposal intentionally rejects a read-write to immutable cascade for
ordinary `search`, `search-all`, `doctor`, and registry/status reads. Although
that cascade can help classify failures structurally, it lets read commands
write on success in writable environments and keeps two permission-dependent
read paths alive. Read-write qmd-rs opens remain appropriate for index-writing,
repair, and explicit migration flows whose command contract says they may
mutate managed cache state.

Read-only consumers:

- `llm-wiki search` lexical branch
- `llm-wiki search-all` lexical branch
- `llm-wiki doctor` qmd-rs store checks
- `llm-wiki projects` / registry status checks that inspect index health
- semantic and hybrid readiness checks that only need lexical store status

Write-owning consumers:

- `llm-wiki index`
- `llm-wiki index-all`
- future `llm-wiki update --reindex`
- any explicit repair command that states it writes target-project cache state

The adapter should not call a write-initializing qmd-rs store constructor for
read-only commands. A qmd-rs read-only API is not available in the current
dependency version. The preferred mechanism is a narrow qmd-rs immutable
read-only constructor, contributed upstream or vendored locally if that is
small enough for the implementation plan. That helper would open SQLite with
read-only immutable semantics, skip schema initialization, and reuse qmd-rs's
existing pure-read query methods so ranking and schema ownership stay with qmd.
If that is too invasive, an adapter-owned immutable SQLite path is acceptable
with parity tests. Acceptable implementation paths are:

1. Contribute or vendor a narrow qmd-rs read-only open helper that uses
   immutable SQLite semantics for completed stores.
2. Add an adapter-owned immutable SQLite query path for the qmd-rs schema.
3. Adopt a future upstream qmd-rs read-only API only after verifying that it
   reuses the same query semantics, does not write side files, and works on
   read-only managed cache media.

The adapter remains responsible for query sanitization, metadata extraction,
snippets, freshness markers, and stable result JSON. The implementation should
not expose qmd-rs internals as the user-facing citation path.

## Read Path Architecture

The implementation must choose one permission-independent read architecture for
completed qmd-rs stores:

1. Always use a qmd-rs immutable read-only constructor if such a helper is
   contributed, vendored, or later released upstream.
2. Always use an adapter-owned immutable SQLite path for query and status
   reads, while qmd-rs remains the writer/indexer.

The adapter must not mix a writable `Store::open` query path in writable
environments with an immutable SQLite fallback in read-only environments. Any
adapter-owned SQL path must be guarded by query-parity tests against accepted
search expectations so schema or ranking drift is caught before release.

The qmd immutable constructor path is preferred because it lets both writable
and non-writable environments call the same qmd search implementation without
using a write-capable connection. Adapter-owned SQL is the pragmatic fallback,
not the ideal long-term shape.

## Completed Store And Publication Contract

The term "completed store" means more than "a SQLite file exists." A completed
qmd-rs search store must satisfy all of these gates before read commands treat
it as queryable:

- the live SQLite main database exists at the promoted store path.
- the matching llm-wiki metadata file exists beside it and matches the same
  project, backend, schema version, and indexed wiki snapshot.
- `index` or `index-all` has checkpointed, closed, or otherwise proved that the
  SQLite main database contains all committed index content needed by immutable
  readers.
- an immutable open of the candidate store succeeds before the candidate is
  published as live.

Writer commands must own that proof. `index`, `index-all`, and future
`update --reindex` must checkpoint or otherwise verify the temp store's main DB
through the same immutable read path before live promotion. A store must not be
promoted merely because the write-capable qmd-rs connection finished without
returning an error.

Read commands should inspect llm-wiki metadata before opening SQLite whenever
possible. Missing, mismatched, or incomplete metadata means the store is not a
completed store and should be reported as missing, stale, schema-mismatched, or
transient according to the observed condition. Read commands must not open
stray, active, or metadata-less SQLite files as valid qmd-rs stores just
because the SQLite file exists.

Promotion must publish the SQLite main DB plus llm-wiki metadata as one
consistent live unit, or readers must have explicit retryable transient-state
semantics for the current multi-file promotion window. The proposal must not
claim strict old-or-new visibility unless the implementation actually provides
atomic publication for the complete sqlite+metadata unit. If publication remains
multi-file, read commands should classify mixed old/new, missing metadata, or
half-promoted observations as retryable transient state instead of corruption.

## Immutable Store Semantics

Immutable SQLite reads must be limited to completed stores that have already
been promoted into the live index location. They must not be used against an
active temporary index build.

`immutable=1` tells SQLite to treat the database file as unchanging and avoids
touching WAL/SHM side files. That is the property needed for sandboxed readers,
but it also means uncheckpointed WAL content is ignored. Completed
`llm-wiki index` runs must close, checkpoint, or otherwise prove the main DB is
fully readable through the immutable path before promotion. If a crashed or
interrupted writer leaves uncheckpointed WAL content, an immutable reader can
observe an older main database state. Readiness checks must therefore continue
to validate llm-wiki metadata and freshness rather than treating any readable
SQLite file as ready.

Read-only commands rely on the completed-store publication contract. During
`index --force`, a reader should see either a completed old store, a completed
new store, or a retryable transient publication state. It must not report a
half-promoted sqlite+metadata observation as corruption, and it must not inspect
an active temporary build as immutable.

A fresh immutable read of a completed, metadata-current store is not degraded
merely because it is read-only. Degraded status should be reserved for stale,
fallback, incomplete, or otherwise reduced-confidence cases.

## Error Semantics

Search and diagnostics should distinguish at least these states:

- `missing`: store file or metadata is absent.
- `transient`: promotion appears to be in progress or the reader observed a
  mixed sqlite+metadata unit that should be retried.
- `stale`: store opens read-only and metadata no longer matches wiki files.
- `permission_denied`: the store exists but cannot be read because of access or
  sandbox policy.
- `corrupt`: the store is readable enough to inspect but SQLite/qmd-rs cannot
  parse it as a valid store.
- `schema_mismatch`: llm-wiki metadata or qmd-rs schema is incompatible with
  the current binary.

`permission_denied` must not be reported as `corrupt`. It should provide scope
accurate guidance such as "allow read access to the managed cache" or "run the
command outside the sandbox" rather than "rebuild the index".

The permission state must be threaded through every consumer that currently
collapses backend-open failures into corruption or forced reindex guidance:

- search readiness and `search_attempt` handling must not map
  `permission_denied` to the `ForceReindex` path.
- `doctor` must report access failure separately from corruption and must not
  recommend rebuilding a healthy but inaccessible managed cache.
- registry and project-status checks must expose a distinct access-failure
  status or equivalently explicit message.
- freshness/status label helpers must preserve the difference between
  unreadable, stale, corrupt, and schema-mismatched stores.

When `search --format json` encounters a read-access failure, JSON output
should preserve the existing readiness/fallback contract instead of mixing
diagnostic logs into stdout. Human-readable diagnostics belong on stderr or in
explicit JSON metadata fields.

For `search-all --format json`, one project's `permission_denied` or retryable
`transient` state should not make the whole command emit non-JSON output. The
command should skip that project, include a per-project report with readiness
metadata and warning text, and continue merging results from other ready
projects. A non-zero process exit may be reserved for command-level failures
that prevent forming the JSON envelope at all, not for ordinary per-project
cache access failures.

JSON metadata should expose the completed-store read path, for example
`open_mode: "read_only_immutable"` or equivalent, and may include an access
reason when permission policy affects availability. It should not mark a fresh
completed immutable read as `degraded: true` solely because the cache was opened
read-only.

The JSON schema location should be backend-status metadata, surfaced at the
top level for single-project `search` and within each `projects[]` entry for
`search-all`. Per-result backend/freshness fields may continue to describe the
origin of each result, but `open_mode` belongs to the store/readiness report,
not individual results.

## Relationship To Project Update

`wiki/proposals/project-update-command.proposal.md` is related but not a
complete fix.

That proposal correctly assigns target-project cache rebuilds to
project-scoped host-local state and gives `update --reindex` a natural place to
repair or refresh a stale cache. This proposal covers the complementary read
contract: ordinary search and doctor checks should be able to use an already
built managed cache without write permission.

The two proposals should stay separate:

- `update --reindex` answers "who may rebuild this project's cache?"
- sandbox-safe cache reads answer "how do read-only commands inspect and query
  the cache without mutating it?"

## Non-Goals

- Do not move qmd-rs indexes back into project roots.
- Do not make search commands implicitly rebuild indexes.
- Do not let `search` or `doctor` accept licenses, download model artifacts, or
  mutate global runtime state.
- Do not solve Metal / local LLM query-expansion runtime failures in this
  proposal. Those belong to semantic/hybrid readiness and model-runtime
  diagnostics.
- Do not replace qmd-rs as the selected backend.

## Acceptance Criteria

- A fresh `llm-wiki search --mode lexical --format json "<query>"` can read a
  fresh managed qmd-rs store when the process has read access to
  the managed index directory but no write access.
- The read-only qmd-rs adapter path uses SQLite `mode=ro&immutable=1` or an
  equivalent verified no-write helper for completed promoted stores. Plain
  SQLite `mode=ro` is not considered sufficient.
- The same immutable completed-store read path is used for search/status in
  writable and non-writable cache environments; immutable reads are not a
  permission-error fallback after `Store::open`.
- Ordinary read commands do not use a read-write to immutable cascade and do
  not perform write-capable qmd-rs opens on the success path.
- `llm-wiki index` and `index-all` checkpoint or otherwise prove the SQLite
  main DB is complete by verifying the candidate through the immutable read
  path before live promotion.
- A completed store is concretely defined as a live SQLite main DB plus
  matching llm-wiki metadata for the same project/backend/schema/snapshot, and
  read commands inspect metadata before SQLite open whenever possible.
- `llm-wiki doctor` reports the same store as ready or stale, not corrupt, in a
  read-only cache environment.
- A truly unreadable cache reports `permission_denied` or equivalent guidance,
  not `corrupt`.
- `permission_denied` bypasses the current corrupt/schema-mismatch forced
  reindex path in search, doctor, registry status, and freshness labels.
- `llm-wiki index` and `index-all` still own all qmd-rs schema initialization
  and cache writes.
- Search racing with `index --force` sees either a completed old store, a
  completed new store, or a retryable transient publication state; it does not
  inspect an active temporary build as immutable and does not classify
  half-promoted observations as corruption.
- JSON search output remains parseable and does not include qmd-rs or
  llama.cpp diagnostic logs on stdout.
- `search-all --format json` reports per-project `permission_denied` and
  retryable transient cache states in the `projects[]` readiness metadata and
  warnings while continuing to return a parseable JSON envelope for other ready
  projects.
- JSON metadata identifies the read path/open mode without labeling a fresh
  immutable completed-store read as degraded solely because it is read-only;
  `open_mode` is reported in backend-status metadata, top-level for
  single-project search and per-project for `search-all`.
- Regression tests cover immutable read-only cache search, read-only
  doctor/status, permission-denied/corrupt distinction, the forced-reindex
  bypass for permission failures, metadata-first completed-store validation,
  immutable pre-promotion verification, promotion-race transient handling,
  `search-all` per-project JSON behavior, and query parity for any
  adapter-owned SQL read path.

## Review Questions

- Should the initial plan choose adapter-owned immutable SQLite reads for all
  completed-store queries/status checks, or first attempt a narrow qmd-rs
  immutable read-only helper? The helper is preferred if it is cheap enough to
  land because it preserves qmd-owned query semantics; in either case, the
  chosen read path must be used in both writable and non-writable cache
  environments.
- Should `projects` expose a distinct `index-permission-denied` status, or is a
  status message on `index-unknown` enough?
- How aggressively should `doctor` flag possible crashed-writer artifacts such
  as unexpected WAL side files beside a promoted store?
- Should promotion move to an atomic directory/sentinel publication model for
  the sqlite+metadata unit, or keep file-by-file promotion and make transient
  retry behavior part of the read contract?
