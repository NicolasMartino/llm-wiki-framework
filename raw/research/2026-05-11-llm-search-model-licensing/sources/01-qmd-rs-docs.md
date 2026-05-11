# Source 01: qmd-rs Docs

- Source URL: https://docs.rs/qmd/latest/qmd/
- Retrieved: 2026-05-11
- Source type: External documentation

## Relevant Facts

- The docs.rs page is for `qmd` 0.3.2.
- The crate is licensed `MIT OR Apache-2.0`.
- The documented feature set includes BM25/SQLite FTS5, vector semantic search
  with local GGUF embeddings, hybrid search with query expansion and RRF,
  reranking with cross-encoder models, collection management, and automatic
  model download from Hugging Face.
- The public API lists `EmbeddingEngine`, `GenerationEngine`, `RerankEngine`,
  `pull_model`, `pull_models`, `resolve_model`, `hybrid_search_rrf`, and
  `reciprocal_rank_fusion`.

## Implication For llm-wiki

`llm-wiki` can use qmd-rs primitives, but it must own the user-facing download
contract. No ordinary `search`, `search-all`, `index`, or `wiki-query` path may
silently trigger qmd-rs model downloads.
