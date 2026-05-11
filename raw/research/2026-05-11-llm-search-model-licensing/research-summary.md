# Research Summary: LLM Search Model Licensing

## Question

What must the semantic/hybrid search plan know about model candidates,
licenses, hashes, downloads, cache ownership, and observability before
implementation starts?

## Scope

This bundle covers the three qmd-rs default model candidates identified by the
search backend eval:

- `embeddinggemma-300M-Q8_0.gguf`
- `qwen3-reranker-0.6b-q8_0.gguf`
- `qmd-query-expansion-1.7B-q4_k_m.gguf`

It also checks the landed CLI observability substrate because the semantic
plan treats that work as completed in a separate worktree.

## Key Findings

- qmd-rs 0.3.2 documents FTS, vector semantic search, hybrid RRF, query
  expansion, reranking, and automatic model download from Hugging Face. The
  `llm-wiki` adapter must keep ordinary search from triggering that automatic
  download path.
- EmbeddingGemma is a Gemma-licensed Google model with Hugging Face access
  conditions. Its GGUF conversion is suitable as the default embedding
  candidate only if interactive install records license acknowledgement before
  materializing bytes.
- Qwen3 Reranker 0.6B and the ggml-org Q8_0 conversion are Apache-2.0. Rerank
  remains opt-in, so the model is not required for baseline hybrid.
- `tobil/qmd-query-expansion-1.7B-gguf` declares MIT on Hugging Face and uses a
  Qwen3 1.7B base lineage. It is required for baseline hybrid and should be
  downloaded only during consented install.
- Current remote file hashes were found for all three selected GGUF artifacts.
  Implementation must still verify local bytes after download and store the
  verification result under `~/.llm_wiki`.
- Observability has landed: `src/cli.rs` owns global `--verbose` and
  `CliContext`, `src/main.rs` initializes tracing from the parsed verbose flag,
  and `src/search/commands.rs` already emits search/search-all diagnostics for
  project selection, store/index state, query normalization, filters, counts,
  and no-result explanations.

## Caveats

- The research records source-reported artifact hashes. It does not download
  model bytes. Stage 1 must verify hashes after user consent.
- The query-expansion hash was observed through the pinned Hugging Face file
  page for the q4_k_m artifact because the same file page on `main` returned a
  rate limit during research.
- No thresholds are accepted by this bundle. The eval page records candidate
  calibration inputs and keeps runtime thresholds unset until labels and
  calibration are approved.

## Ingest Readiness

Ready to ingest into:

- `wiki/references/llm-search-model-licensing.reference.md`
- `wiki/evals/natural-language-search.eval.md`
