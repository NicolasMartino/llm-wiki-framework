# Project Registry And Search Artifacts

- Document Class: Proposal
- Status: Proposed
- Date: 2026-05-06
- Category: Search infrastructure, framework tooling
- Scope: Add project registration, centralized per-project search artifacts, and explicit cross-project search commands to the `llm-wiki` binary after D8 ships.
- Sources: wiki/plans/llm-wiki-binary.plan.md, wiki/decisions/llm-wiki-binary-distribution.decision.md, wiki/specs/documentation-model.spec.md, wiki/references/qmd-search-engine.reference.md, https://docs.rs/qmd/latest/qmd/ (qmd 0.3.2 docs), user discussion 2026-05-06
- Related: wiki/proposals/llm-wiki-binary.proposal.md, wiki/roadmaps/framework-v1.roadmap.md

## Question

After the D8 `llm-wiki` binary exists, should it own local search artifacts
for registered projects and expose a distinct `search-all` command for
cross-project wiki search?

## Proposal

Yes. Add a post-D8 search layer to `llm-wiki` that keeps each project's
markdown wiki as the canonical source of truth while storing rebuildable search
artifacts in a centralized cache. The binary registers projects explicitly,
indexes each project's `wiki/` into a per-project QMD store, and exposes both
project-local and cross-project retrieval commands.

The command split is intentional:

```text
llm-wiki register <path> [--name <name>]
llm-wiki projects
llm-wiki index
llm-wiki index-all
llm-wiki search "query"
llm-wiki search-all "query"
```

`search` means the current project by default. `search-all` means every
explicitly registered project and is never implied. Boundary crossing is
visible in command history, agent traces, and documentation.

The search backend should use the Rust `qmd` crate when its API and quality are
ready for this framework. The current qmd crate documentation describes a Rust
library with `Store`, BM25 search through SQLite FTS5, local GGUF embeddings,
hybrid search with query expansion and RRF fusion, reranking, collection
management, and automatic Hugging Face model download. That makes it plausible
for `llm-wiki` to integrate QMD as a library instead of shelling out to the
existing Node/Bun implementation.

## Roadmap Position

This proposal should not be folded into D8. D8 is already the implementation
baseline: skill installation, deterministic project scaffolding, canonical
skill projection, manifest ownership, `status`, `doctor`, `uninstall`, tests,
and distribution.

If accepted, this should become a new deliverable after D8, probably:

```text
D9 - Project Registry And Search Artifacts
```

D9 would turn the binary from framework installer/scaffolder into the owner of
rebuildable search state. It does not change the three-layer architecture. It
adds a derived cache layer next to it.

## Why

The current framework already identifies QMD as the scale solution once
`wiki/index.md` stops being enough. Today that solution is external: install
QMD separately, index each wiki separately, and teach agents when to invoke it.
That works, but it weakens the D8 "one binary" story and keeps cross-project
search out of reach.

A binary-owned registry and search cache gives the framework four useful
properties:

1. **One tool owns project discovery.** `llm-wiki` knows which projects are
   registered, where their wikis live, and where their search artifacts live.
2. **Per-project indexes stay isolated.** Each project gets its own QMD store,
   preserving project boundaries while allowing global orchestration.
3. **Cross-project search becomes explicit and repeatable.** `search-all`
   retrieves across registered projects without asking an agent to manually
   discover directories.
4. **The canonical source remains markdown.** Search databases are disposable
   artifacts. Rebuild them from `wiki/`; never treat them as durable knowledge.

This also creates a clean future path for "find similar decisions across all my
projects", "show every project that solved skill projection drift", or "search
all registered wikis for accepted decisions about model caching" without
violating the framework rule that `wiki/` is agent-owned compiled knowledge.

## Artifact Model

Use a central registry plus one search store per project:

```text
~/.local/share/llm-wiki/projects.json
~/.cache/llm-wiki/indexes/<project-id>/qmd.sqlite
~/.cache/llm-wiki/models/
```

`projects.json` records only registration metadata:

```json
{
  "projects": [
    {
      "id": "llm-wiki-framework",
      "name": "LLM Wiki Framework",
      "root": "/Users/nicolasmartino/Documents/local_llm_wiki/llm_wiki_framework",
      "wiki_path": "wiki",
      "registered_at": "2026-05-06T12:00:00Z"
    }
  ]
}
```

The registry is not project knowledge. It is local tool state. A project remains
valid if it is never registered; it just cannot participate in `search-all`.

Search stores are rebuildable caches. If a QMD store is missing, stale, or
corrupt, `llm-wiki index` rebuilds it from `wiki/`. No ingest, query, or lint
operation may write durable knowledge only into the search store.

## Command Semantics

### `register`

`llm-wiki register <path> [--name <name>]`

Adds a project to the registry. The path must contain a framework-shaped wiki:
`wiki/index.md`, `wiki/log.md`, and `project_guidelines.md` or `CLAUDE.md`.
Registration refuses ambiguous paths and reports which expected artifact is
missing.

### `projects`

Lists registered projects with ID, name, root path, wiki path, index status,
and last indexed time.

### `index`

`llm-wiki index [--project <id>] [--force]`

Indexes the current project by default. Outside a project, `--project <id>` is
required. The command indexes `wiki/` only, extracts document metadata from each
page, and stores enough path metadata to let results cite exact wiki files.

### `index-all`

Reindexes all registered projects. This is operationally distinct from
`index`, mirroring the `search` / `search-all` boundary.

### `search`

`llm-wiki search "query" [--project <id>] [--class <type>] [--status <status>]`

Retrieves ranked results from one project. In a project directory, the default
project is the current one. Outside a project, the user must pass
`--project <id>`.

Output must include project ID, wiki path, document title, document class,
status, score, and a short snippet. It must not synthesize an answer.

### `search-all`

`llm-wiki search-all "query" [--include <id,id>] [--exclude <id,id>] [--class <type>] [--status <status>]`

Retrieves across registered projects. The command is always explicit. Results
include project identity on every row. The implementation may use federated
search over per-project stores, then normalize and fuse scores globally.

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

## Privacy And Scope Control

Cross-project search is powerful enough to be dangerous if it is implicit.
Agents can accidentally mix private, irrelevant, or stale context from another
project into an answer. The command surface therefore uses four controls:

1. Projects must be explicitly registered.
2. Cross-project retrieval requires the separate `search-all` command.
3. Results always include project identity.
4. `search-all` supports `--include` and `--exclude` filters from the first
   version.

The default behavior of `knowledge-query` remains project-local. A future skill
update may mention `search-all`, but only as an explicit user-requested scope
expansion.

## Rust QMD Integration

The qmd crate should be treated as the preferred candidate backend because it
matches the D8 binary's Rust distribution model. The current docs expose the
right primitives for this framework: a `Store`, full-text search, vector search,
hybrid search, reranking types, collection helpers, chunking helpers, model
download helpers, and SQLite-backed local storage.

Before committing to it, D9 needs an eval against the existing QMD behavior
documented in `wiki/references/qmd-search-engine.reference.md`:

1. Index this repo's `wiki/`.
2. Run a fixed query set covering specs, decisions, proposals, plans, and
   references.
3. Compare top-k relevance against the current QMD implementation where
   practical.
4. Measure model download behavior, cold-start time, index size, and search
   latency.
5. Verify licenses and binary distribution implications for `llama-cpp-2`,
   GGUF model downloads, and release packaging.

The models should not be embedded into the `llm-wiki` binary. The binary should
manage a model cache under `~/.cache/llm-wiki/models/` and work offline after
models are present.

## Non-Goals

- Do not change D8 scope.
- Do not replace `wiki/index.md` as the primary orientation file for small
  projects.
- Do not store canonical project knowledge in SQLite or vector stores.
- Do not make cross-project search implicit.
- Do not add answer synthesis in the first search deliverable.
- Do not index `raw/` by default. The framework's query surface runs against
  compiled wiki knowledge, not raw source dumps.
- Do not embed multi-gigabyte model files in the binary.

## Risks

| Risk | Likelihood | Impact | Mitigation |
| --- | --- | --- | --- |
| Cross-project search leaks context across project boundaries | Medium | High | Separate `search-all` command, explicit registration, project labels on every result, include/exclude filters |
| qmd crate API changes while young | Medium | Medium | Put integration behind a small internal adapter; pin versions; run evals before acceptance |
| Model downloads undermine the one-binary experience | Medium | Medium | Binary remains one executable; models are managed cache artifacts with clear `doctor` output |
| Federated result scores are not comparable across projects | Medium | Medium | Normalize per-project scores and apply global RRF/rerank; expose project identity so agents can judge relevance |
| Indexes go stale after ingest/lint | High | Medium | `knowledge-ingest` and `knowledge-lint` specs can later recommend `llm-wiki index` after wiki mutation; `projects` reports stale status |
| Index cache grows too large across many projects | Medium | Low | Per-project stores are easy to delete/rebuild; `doctor` can report cache size and cleanup commands |

## Acceptance Criteria

1. `llm-wiki register <path>` records a framework-shaped project in
   `projects.json` and refuses non-framework paths with clear diagnostics.
2. `llm-wiki projects` lists all registered projects with index status.
3. `llm-wiki index` builds a per-project search store from `wiki/` only.
4. `llm-wiki search "query"` retrieves ranked project-local results and never
   crosses project boundaries by default.
5. `llm-wiki search-all "query"` retrieves across registered projects and
   labels every result with project identity.
6. `search-all --include` and `search-all --exclude` constrain the searched
   project set.
7. Results include wiki file path, document class, status, title, score, and
   snippet.
8. Search artifacts are rebuildable: deleting an index and running
   `llm-wiki index` recreates it without touching project wiki content.
9. Model cache behavior is explicit: first run may download models; subsequent
   offline runs work when models are cached.
10. An eval page records search-quality findings before the backend is accepted
    as the framework's search engine.

## Implementation Outline

1. Add a small project-registry module to `tools/llm-wiki/` after D8 ships.
2. Add `register`, `projects`, `index`, `index-all`, `search`, and
   `search-all` subcommands.
3. Create an internal search adapter trait so qmd crate churn is isolated.
4. Implement the qmd-backed adapter against per-project stores.
5. Add metadata extraction from wiki pages so filters work on document class
   and status.
6. Add integration tests with two tempdir projects and synthetic wiki pages.
7. Add an eval page comparing local and cross-project retrieval quality.
8. If accepted, promote the proposal to a decision and add D9 to the roadmap.

## Revisit When

- D8 has shipped and `llm-wiki` is the actual operating model.
- The qmd crate API has stabilized enough for an internal adapter.
- The framework has at least two real registered projects, making
  cross-project search testable on meaningful data.
- Agents need higher-level answer synthesis; at that point consider a separate
  `llm-wiki query` command, not an overloaded `search-all`.
