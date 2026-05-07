# Project Registry and Search Artifacts

- Document Class: Proposal
- Status: Proposed
- Date: 2026-05-06
- Category: Search infrastructure, framework tooling
- Scope: Add project registration, centralized per-project search artifacts, and explicit cross-project search commands to the `llm-wiki` binary after D8 ships.
- Sources: wiki/plans/llm-wiki-binary.plan.md, wiki/decisions/llm-wiki-binary-distribution.decision.md, wiki/specs/documentation-model.spec.md, wiki/references/qmd-search-engine.reference.md, wiki/references/qmd-rs-search-crate.reference.md, user discussion 2026-05-06
- Related: wiki/proposals/llm-wiki-binary.proposal.md, wiki/proposals/search-backend-selection.proposal.md, wiki/roadmaps/framework-v1.roadmap.md

## Question

After the D8 `llm-wiki` binary exists, should it own local search artifacts for
registered projects and expose a distinct `search-all` command for
cross-project wiki search?

## Proposal

Yes. Add a post-D8 search layer to `llm-wiki` that keeps each project's
markdown wiki as the canonical source of truth while storing rebuildable search
artifacts in a centralized cache. The binary registers projects explicitly,
indexes each project's `wiki/` into a per-project search store, and exposes
both project-local and cross-project retrieval commands.

This proposal is about the registry, command surface, cache model, lifecycle,
and output contracts. It does **not** choose the search backend. qmd-rs, Tobi
QMD shell-out, and a direct BM25-only SQLite implementation are backend options
covered by `wiki/proposals/search-backend-selection.proposal.md`.

The command split is intentional:

```text
llm-wiki register <path> [--name <name>] [--id <id>]
llm-wiki register --update <id> <path>
llm-wiki forget <project-id> [--delete-cache]
llm-wiki projects
llm-wiki index
llm-wiki index-all
llm-wiki search "query"
llm-wiki search-all "query"
```

`search` means one project. `search-all` means every explicitly registered
project unless narrowed by project IDs. Boundary crossing is visible in command
history, agent traces, and documentation.

## Roadmap Position

This proposal should not be folded into D8. D8 has shipped the implementation
baseline: skill installation, deterministic project scaffolding, canonical
skill projection, manifest ownership, `status`, `doctor`, `uninstall`, tests,
and distribution.

If accepted, this should become a new deliverable after D8, probably:

```text
D9 - Project Registry and Search Artifacts
```

D9 is a large surface-area bet. It roughly doubles the binary's command surface
after D8's six commands by adding registry, indexing, and retrieval commands.
That is justified only if the deliverable stays focused on tool-owned search
state and avoids answer synthesis, agent judgment, or backend experimentation
inside the same proposal.

Backend choice is a separate decision. If qmd-rs evaluation fails, D9 can still
proceed with a different adapter, a BM25-only first version, or a reduced plan
that lands registry/index lifecycle before hybrid search.

## Why

The current framework already identifies QMD as the scale solution once
`wiki/index.md` stops being enough. Today that solution is external: install
QMD separately, index each wiki separately, and teach agents when to invoke it.
That works for one project, but it does not give the framework a durable
project registry or a clean cross-project search surface.

A binary-owned registry and search cache gives the framework four useful
properties:

1. **One tool owns project discovery.** `llm-wiki` knows which projects are
   registered, where their wikis live, and where their search artifacts live.
2. **Per-project indexes stay isolated.** Each project gets its own search
   store, preserving project boundaries while allowing global orchestration.
3. **Cross-project search becomes explicit and repeatable.** `search-all`
   retrieves across registered projects without asking an agent to manually
   discover directories.
4. **The canonical source remains markdown.** Search databases are disposable
   artifacts. Rebuild them from `wiki/`; never treat them as durable knowledge.

This creates a clean path for "find similar decisions across all my projects",
"show every project that solved skill projection drift", or "search all
registered wikis for accepted decisions about model caching" without violating
the framework rule that `wiki/` is agent-owned compiled knowledge.

## Artifact Model

Use a host-local registry plus one search store per project:

```text
~/.local/share/llm-wiki/projects.json
~/.cache/llm-wiki/indexes/<project-id>/
~/.cache/llm-wiki/models/
```

The registry lives in `~/.local/share/llm-wiki/` because it is local tool state,
not user-edited configuration. It stores absolute paths and is explicitly
host-local. Copying dotfiles or syncing `~/.local/share` across machines may
produce stale registry entries; `llm-wiki projects` and `llm-wiki doctor`
should report missing roots.

`projects.json` records registration metadata:

```json
{
  "projects": [
    {
      "id": "llm-wiki-framework",
      "name": "LLM Wiki Framework",
      "root": "/Users/nicolasmartino/Documents/local_llm_wiki/llm_wiki_framework",
      "wiki_path": "wiki",
      "registered_at": "2026-05-06T12:00:00Z",
      "last_indexed_at": "2026-05-06T12:05:00Z",
      "last_indexed_wiki_max_mtime": "2026-05-06T12:04:30Z",
      "indexed_file_count": 42,
      "backend": "qmd-rs",
      "index_schema_version": 1
    }
  ]
}
```

Project IDs are stable local identifiers:

1. `--id <id>` wins when provided.
2. Otherwise, slugify `--name <name>`.
3. Otherwise, slugify the project directory basename.
4. On collision, append `-2`, `-3`, etc. The generated ID is printed and stored.

Search stores are rebuildable caches. If a search store is missing, stale, or
corrupt, `llm-wiki index` rebuilds it from `wiki/`. No ingest, query, or lint
operation may write durable knowledge only into the search store.

D8's `llm-wiki uninstall` should continue to mean "remove installed framework
skills". It must leave `projects.json`, search indexes, and model caches alone.
Reinstalling the binary should rediscover previously registered projects. Cache
cleanup belongs to `forget --delete-cache` or a future explicit cache command,
not to skill uninstall.

## Staleness

Staleness is defined in V1 by timestamps and file count. When indexing
completes, the registry records `last_indexed_at`,
`last_indexed_wiki_max_mtime`, and `indexed_file_count` for `wiki/**/*.md`.

A project is stale when:

1. the current max wiki mtime is newer than `last_indexed_wiki_max_mtime`;
2. the current wiki markdown file count differs from `indexed_file_count`;
3. the registered root no longer exists; or
4. the index backend/schema version does not match the current binary.

This model has known false negatives. Operations that preserve mtimes, such as
certain `git checkout`, backup restore, or sync-tool flows, can change content
without tripping the timestamp signal. `llm-wiki index --force` is the escape
valve. A later implementation may replace timestamp + count with a hash
manifest.

## Command Semantics

### `register`

```text
llm-wiki register <path> [--name <name>] [--id <id>]
llm-wiki register --update <id> <path>
```

Adds or updates a project in the registry. The path must contain a
framework-shaped wiki: `wiki/index.md`, `wiki/log.md`, and at least one
schema/orientation file from the framework's recognized schema-file set. The
current set is `project_guidelines.md`, `CLAUDE.md`, and `AGENTS.md`. The set
should be declared in one framework constant and reflected in the
documentation-model spec so future runtime support (Cursor/Aider/Amp) updates
the validation in one place.

`register --update <id> <path>` changes the root for an existing project ID and
is the recovery path after a project move or rename.

### `forget`

```text
llm-wiki forget <project-id> [--delete-cache]
```

Removes a project from the registry. By default it leaves the search cache in
place so accidental forget is cheap to recover from. `--delete-cache` removes
the project's rebuildable index artifacts. It never modifies the project
directory.

### `projects`

Lists registered projects with ID, name, root path, wiki path, index status,
last indexed time, wiki max mtime at last index, current wiki max mtime,
staleness status, backend, and cache size when known.

### `index`

```text
llm-wiki index [--project <id>] [--force]
```

Indexes the current project by default. Outside a project, `--project <id>` is
required. The command indexes `wiki/` only, extracts document metadata from each
page, and stores enough path metadata to let results cite exact wiki files.

Index writes use a per-project lockfile. Rebuilding writes to a temporary store
and atomically swaps it into place after success. A crash during `index` or
`index-all` leaves the previous index intact when one exists; otherwise the
project remains unindexed and `projects` reports that state.

### `index-all`

Reindexes all registered projects, one project transaction at a time. A failure
for one project is reported but does not corrupt other project indexes.

### `search`

```text
llm-wiki search "query" [--project <id>] [--class <type>] [--status <status>] [--format text|json]
```

Retrieves ranked results from one project. In a project directory, the default
project is the current one. Outside a project, the user must pass
`--project <id>`.

`--class` and `--status` use the document class and status vocabulary from
`wiki/specs/documentation-model.spec.md`. The documentation-model spec remains
the source of truth for those values.

Output formats:

- `text` (default): stable human-readable rows for agents and terminals.
- `json`: stable machine-readable objects for programmatic use.

Result fields: project ID, project name, wiki file path, title, document class,
status, score, snippet, and stale/fresh marker for the searched project.

Snippets are centered on the best match span when the backend exposes one.
Default length is 200 characters with `...` ellipses on truncation. If the
backend cannot provide a match span, use the first relevant chunk returned by
the backend and truncate it to the same budget.

### `search-all`

```text
llm-wiki search-all "query" [--include <id> ...] [--exclude <id> ...] [--class <type>] [--status <status>] [--format text|json]
```

Retrieves across registered projects. The command is always explicit. Results
include project identity on every row.

`--include` and `--exclude` accept project IDs only. They are repeated flags,
not comma-separated lists. If neither is present, all registered projects are
searched. Running `search-all` from inside a registered project includes that
current project in the searched set like any other registered project. It does
not get a ranking boost; the result project label is the disambiguation.

Default fusion:

1. retrieve top 20 results per selected project;
2. normalize each project result list by rank;
3. fuse globally with reciprocal rank fusion using `k=60`;
4. optionally rerank the globally fused top-k if the selected backend supports
   reranking.

The top-20 per-project cap prevents a large project from dominating merely
because it can return many candidates. The cap should become a flag only if
real usage shows it needs tuning.

## `search` Versus `query`

Reserve `query` for a possible later command that synthesizes answers with
citations. In this proposal:

```text
search      = retrieve local ranked passages/pages
search-all  = retrieve across registered projects
query       = future synthesis surface, not part of this proposal
```

This keeps retrieval and answer-writing separate. It matches the framework's
existing boundary: tools can find candidate pages, but the agent still reads
wiki pages and synthesizes cited answers.

## Init Integration

After D9 exists, `llm-wiki init <path>` should auto-register newly scaffolded
projects by default and print the generated project ID. Add `--no-register` to
opt out.

This default is acceptable because `init` is already an explicit framework
action and `search-all` remains explicit before cross-project retrieval occurs.
Existing projects added to the framework still require `llm-wiki register`.

## Privacy And Scope Control

Cross-project search is powerful enough to be dangerous if it is implicit.
Agents can accidentally mix private, irrelevant, or stale context from another
project into an answer. The command surface therefore uses four controls:

1. Projects must be registered before they can participate.
2. Cross-project retrieval requires the separate `search-all` command.
3. Results always include project identity.
4. `search-all` filters by explicit project IDs.

The default behavior of `knowledge-query` remains project-local. Updating the
skill to mention `search-all` is out of scope for the first D9 implementation
unless the accepted plan explicitly adds it. If added later, it should present
`search-all` only as an explicit user-requested scope expansion.

## Backend Boundary

D9 needs a search backend, but this proposal does not own the backend decision.
The accepted D9 backend is qmd-rs, selected in
`wiki/decisions/search-backend-selection.decision.md` based on
`wiki/evals/search-backend-selection.eval.md`.

The implementation should still depend on an internal adapter trait so direct
SQLite FTS5 can remain a fallback and Tobi QMD can be compared later.

Because qmd-rs can use local GGUF model artifacts for semantic modes,
`llm-wiki doctor` should report store existence, schema/version state,
stale/missing index state, corrupt stores, required local model artifacts, and
rebuild/download guidance.

Execution plan for the backend slice:
`wiki/plans/qmd-rs-search-backend.plan.md`.

## Non-Goals

- Do not change D8 scope.
- Do not choose the search backend in this proposal.
- Do not replace `wiki/index.md` as the primary orientation file for small
  projects.
- Do not store canonical project knowledge in SQLite or vector stores.
- Do not make cross-project search implicit.
- Do not add answer synthesis in the first search deliverable.
- Do not update `knowledge-query` to use `search-all` automatically.
- Do not index `raw/` by default. The framework's query surface runs against
  compiled wiki knowledge, not raw source dumps.
- Do not embed multi-gigabyte model files in the binary.

## Risks

| Risk | Likelihood | Impact | Mitigation |
| --- | --- | --- | --- |
| Cross-project search leaks context across project boundaries | Medium | High | Separate `search-all` command, explicit registration/auto-registration visibility, project labels on every result, ID filters |
| qmd-rs eval fails | Medium | Medium | Backend choice is separate; fallback to Tobi QMD shell-out, direct BM25-only SQLite, or defer hybrid search |
| Registry rots after project moves | High | Medium | `register --update <id> <path>`, `forget`, and `projects` missing-root diagnostics |
| Concurrent indexing corrupts state | Medium | High | Per-project lockfile, temp index build, atomic swap |
| Crash mid-index leaves partial state | Medium | Medium | Per-project transaction boundary; old index remains until swap succeeds |
| Federated result scores are not comparable across projects | Medium | Medium | Retrieve top 20 per project, fuse ranked lists with RRF (`k=60`), expose project identity |
| Indexes go stale after ingest/lint | High | Medium | Store `last_indexed_wiki_max_mtime` and `indexed_file_count`; `projects` reports stale when current wiki state differs; `index --force` handles false negatives |
| Host-local registry is copied to another machine | Medium | Low | Registry stores absolute paths and is documented as host-local; `projects` reports missing roots |
| Index cache grows too large across many projects | Medium | Low | Per-project stores are easy to delete/rebuild; `forget --delete-cache` removes a project's cache |

## Acceptance Criteria

1. `llm-wiki register <path>` records a framework-shaped project in
   `projects.json` and refuses non-framework paths with clear diagnostics.
2. Project IDs are deterministic and collision-safe: explicit `--id`, else
   slugified `--name`, else slugified basename, with `-2`/`-3` suffixes.
3. `llm-wiki register --update <id> <path>` updates a moved project root.
4. `llm-wiki forget <project-id>` removes a project from the registry;
   `--delete-cache` also removes its rebuildable search artifacts.
5. D9-era `llm-wiki init <path>` auto-registers new projects by default and
   supports `--no-register`.
6. `llm-wiki projects` lists all registered projects with stale/fresh status,
   missing-root status, backend, and cache size when known.
7. `llm-wiki index` builds a per-project search store from `wiki/` only.
8. Concurrent `index` runs against one project are serialized by a lockfile.
9. A crash or failure during `index` or `index-all` does not corrupt a previous
   working index.
10. `llm-wiki search "query"` retrieves ranked project-local results and never
    crosses project boundaries by default.
11. `llm-wiki search-all "query"` retrieves across registered projects and
    labels every result with project identity.
12. `search-all --include <id>` and `search-all --exclude <id>` use repeated
    project-ID flags to constrain the searched project set.
13. Cross-project results retrieve at most top 20 per project before global RRF
    fusion with `k=60`.
14. `search` and `search-all` support `--format text|json`, with `text` as the
    default and stable JSON for programmatic use.
15. Results include project ID, project name, wiki file path, title, document
    class, status, score, snippet, and stale/fresh marker.
16. Snippets are 200 characters centered on the best match span when available,
    with `...` ellipses on truncation.
17. `--class` and `--status` filters use the vocabularies from
    `wiki/specs/documentation-model.spec.md`.
18. Search artifacts are rebuildable: deleting an index and running
    `llm-wiki index` recreates it without touching project wiki content.
19. `llm-wiki uninstall` leaves registry and cache state untouched.

## Implementation Outline

1. Add project registry module and host-local `projects.json` persistence.
2. Add `register`, `forget`, `projects`, `index`, `index-all`, `search`, and
   `search-all` subcommands.
3. Add D9-era `init --no-register` and default auto-registration after a
   successful scaffold.
4. Create an internal search adapter trait; choose the concrete backend through
   `wiki/proposals/search-backend-selection.proposal.md`.
5. Add metadata extraction from wiki pages so filters work on document class
   and status.
6. Add staleness detection using current `wiki/**/*.md` max mtime and file
   count compared against registry metadata.
7. Add per-project lockfile, temp index build, and atomic swap.
8. Add RRF (`k=60`) fusion for `search-all` with top-20 per-project cap.
9. Add text and JSON output formats.
10. Add integration tests with two tempdir projects and synthetic wiki pages.
11. Update `wiki/specs/documentation-model.spec.md` so the scale-search surface
    points to `llm-wiki search` / `llm-wiki search-all` rather than directly to
    QMD MCP once D9 lands.
12. Decide separately whether `knowledge-query` should mention `search-all` as
    an explicit scope expansion; do not make it automatic.

## Transitional Note

Until D9 lands, `wiki/specs/documentation-model.spec.md` remains correct to
describe QMD MCP as the documented scale-search mechanism. If D9 is accepted
and implemented, that spec must be updated because the framework-facing surface
becomes `llm-wiki search` and `llm-wiki search-all`; QMD or qmd-rs becomes an
implementation detail behind the binary.

## Revisit When

- D8 has shipped and `llm-wiki` is the actual operating model.
- A backend decision has been accepted.
- The framework has at least two real registered projects, making cross-project
  search testable on meaningful data.
- Agents need higher-level answer synthesis; at that point consider a separate
  `llm-wiki query` command, not an overloaded `search-all`.
