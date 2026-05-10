# Search Backend Selection

- Document Class: Decision
- Status: Accepted
- Date: 2026-05-07
- Category: Search infrastructure, framework tooling
- Scope: Use qmd-rs as the D9 backend for `llm-wiki search` and `llm-wiki search-all`.
- Sources: wiki/evals/search-backend-selection.eval.md, wiki/proposals/search-backend-selection.proposal.md, wiki/proposals/project-registry-search-artifacts.proposal.md, wiki/references/qmd-rs-search-crate.reference.md
- Related: wiki/proposals/search-backend-selection.proposal.md, wiki/proposals/project-registry-search-artifacts.proposal.md

## Choice

Use qmd-rs as the D9 search backend.

`llm-wiki` will wrap qmd-rs behind an internal search adapter. The adapter owns
query sanitization, metadata extraction, snippets, result output, model/cache
diagnostics, and the command contract. Search stores remain rebuildable caches
derived from `wiki/`; markdown wiki pages remain canonical.

## Why

The project expects to need LLM-enhanced search. Choosing a BM25-only backend
first would reduce short-term complexity, but it would push the real adapter
work into a later migration and risk proving the wrong backend boundary.

qmd-rs is the better product-aligned choice because it keeps the Rust
one-binary direction while leaving room for BM25, vector search, hybrid fusion,
query expansion, and reranking behind one framework-owned adapter.

The eval found real integration work, but none of it invalidates qmd-rs:

1. qmd-rs FTS indexed the current 37-file wiki corpus in 40 ms.
2. qmd-rs FTS searched the fixed query set in about 1 ms per query.
3. It found all fixed-query targets after query sanitization.
4. It returns canonical relative paths, title, score, source, hash, docid,
   collection, modified timestamp, and body length.
5. It supports local GGUF model paths and Hugging Face model download helpers
   for the later semantic path.

The adapter work is not optional if the framework wants LLM-enhanced search.
Therefore D9 should pay that cost directly instead of landing a BM25-only
backend and replacing it later.

## Alternatives Considered

### Direct SQLite FTS5 BM25 Adapter

Direct SQLite FTS5 is a viable fallback, but it is not the selected backend.

The eval showed that a first-party SQLite prototype is simple and deterministic:
it indexed the 37-file corpus in 38 ms, searched in 0-1 ms per query, and
returned canonical paths, document class, status, score, and snippets with no
model downloads or external runtime.

The reason not to choose it is product direction, not technical failure. It is
BM25-only and does not advance the framework toward the LLM-enhanced search path
the project expects to need.

### External Shell-Out Adapter

An external shell-out adapter is not the right first backend behind the Rust
binary.

The eval found that a shell-out backend weakens the one-binary experience and
adds extra operational boundaries around executable discovery, cache state,
path mapping, and concurrent index access.

### Defer Search

Deferring D9 search is not necessary. qmd-rs gives the project a Rust-native
path toward the intended hybrid search capability.

## Consequences

1. D9 search starts with qmd-rs FTS and should keep the adapter shaped for qmd-rs
   vector, hybrid fusion, query expansion, and reranking.
2. `llm-wiki search` and `search-all` should expose retrieval results, not
   synthesized answers.
3. The implementation must use an internal adapter trait so direct SQLite FTS5
   remains available as a fallback.
4. Result metadata must be added at the adapter boundary: project ID, project
   name, wiki file path, title, document class, status, score, snippet, and
   freshness marker.
5. Query sanitization belongs in the adapter boundary before FTS execution.
6. `llm-wiki doctor` should report qmd-rs store existence, schema/version state,
   stale/missing index state, corrupt stores, required local model artifacts for
   semantic modes, and rebuild/download guidance.
7. Model downloads must stay explicit or clearly reported. The binary must not
   embed multi-gigabyte GGUF model files.

## Revisit When

- qmd-rs integration exposes packaging or runtime issues that cannot be handled
  behind the adapter.
- Cross-project search is evaluated on at least two real registered projects.
- Direct SQLite FTS5 proves materially easier to maintain while meeting observed
  retrieval needs.
- qmd-rs releases a materially new version.
