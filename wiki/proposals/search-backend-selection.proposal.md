# Search Backend Selection

- Document Class: Proposal
- Status: Proposed
- Date: 2026-05-06
- Category: Search infrastructure, framework tooling
- Scope: Choose the concrete search backend behind future `llm-wiki search` and `llm-wiki search-all` commands.
- Sources: wiki/references/qmd-search-engine.reference.md, wiki/references/qmd-rs-search-crate.reference.md, wiki/proposals/project-registry-search-artifacts.proposal.md, https://docs.rs/qmd/latest/qmd/ (qmd 0.3.2 docs)
- Related: wiki/proposals/project-registry-search-artifacts.proposal.md, wiki/specs/documentation-model.spec.md

## Question

Which backend should power the future `llm-wiki search` and `llm-wiki
search-all` commands?

## Proposal

Treat backend choice as separate from the project registry and command-surface
proposal. The preferred candidate is qmd-rs because it can integrate as a Rust
library inside the `llm-wiki` binary, but it must pass an eval before becoming
the accepted backend.

The registry/search-artifacts proposal should be able to proceed even if qmd-rs
does not pass. The command surface depends on an internal search adapter, not on
one specific engine.

## Backend Options

### Option A: qmd-rs Library Adapter

Use the Rust `qmd` crate directly. This best preserves the one-binary story:
`llm-wiki` owns the registry, index lifecycle, and search calls without a
Node/Bun runtime dependency.

Required eval:

1. Index this repo's `wiki/`.
2. Run a fixed query set covering specs, decisions, proposals, plans, and
   references.
3. Compare top-k relevance against Tobi QMD where practical.
4. Measure model download behavior, cold-start time, index size, and search
   latency.
5. Verify license and release-packaging implications for `llama-cpp-2`, GGUF
   model downloads, and binary distribution.
6. Verify whether qmd-rs has command/MCP parity with Tobi QMD or only
   library-level parity.

### Option B: Tobi QMD Shell-Out Adapter

Shell out to the existing Node/Bun `@tobilu/qmd` implementation per project.
This preserves the currently documented QMD behavior and MCP feature set, but
it weakens the one-binary experience because users still need an external QMD
installation and model cache.

Use this if qmd-rs quality or API maturity is insufficient but hybrid QMD
behavior is required for D9.

### Option C: Direct SQLite FTS5 BM25 Adapter

Implement a small first-party BM25-only backend over SQLite FTS5. This gives D9
a deterministic, model-free, easy-to-package baseline. It does not provide
semantic search, query expansion, or reranking.

Use this if both QMD options are too immature or operationally heavy, and if a
keyword-search first version is still valuable.

### Option D: Defer Search Backend

Defer D9 implementation if no backend clears the acceptance bar. The registry
and command surface should not ship without a useful local search path.

## Decision Rule

Pick the least operationally complex backend that satisfies the D9 retrieval
quality bar:

1. Prefer qmd-rs if quality, packaging, and API stability are good enough.
2. Use Tobi QMD shell-out if qmd-rs fails but current QMD behavior is required.
3. Use direct BM25-only if local keyword search is enough for the first D9
   version.
4. Defer D9 if none of the above is worth maintaining.

## Non-Goals

- Do not change the `search` / `search-all` command semantics.
- Do not make search stores canonical knowledge.
- Do not embed multi-gigabyte model files into the binary.
- Do not require the answer-synthesis `query` command.

## Acceptance Criteria

1. A search-backend eval page records the query set, corpus, ranking judgments,
   latency, index size, and model/cache behavior for each viable backend.
2. The selected backend satisfies project-local search for specs, decisions,
   proposals, plans, and references.
3. The selected backend supports enough result metadata for `search` and
   `search-all`: path, title, document class, status, score, snippet, and match
   span when available.
4. The selected backend works with per-project stores and can be rebuilt from
   `wiki/`.
5. `llm-wiki doctor` can report missing backend requirements such as external
   executables, models, or corrupt stores.
6. Offline search works after required local artifacts are present.

## Revisit When

- qmd-rs releases a materially new version.
- Tobi QMD changes its CLI/MCP behavior.
- D9 implementation planning starts.
- The framework has at least two real projects to use as a cross-project eval
  corpus.
