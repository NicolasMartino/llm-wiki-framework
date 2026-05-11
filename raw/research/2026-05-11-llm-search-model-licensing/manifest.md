# Research Manifest: LLM Search Model Licensing

- Date: 2026-05-11
- Research question: Which model artifacts, licenses, hashes, download sources,
  and resolver constraints must Stage 0 record before semantic/hybrid search
  implementation starts?
- Goal: Provide provenance for the Stage 0 licensing reference and natural
  language search eval artifacts.
- Source modes: Web, local wiki/code observation.
- Retrieval date: 2026-05-11.

## Inventory

1. `sources/01-qmd-rs-docs.md` - docs.rs qmd 0.3.2 capabilities and resolver
   implications.
2. `sources/02-embeddinggemma-model.md` - EmbeddingGemma GGUF artifact,
   upstream model terms, dimensions, hash, and consent requirements.
3. `sources/03-qwen3-reranker-model.md` - Qwen3 reranker GGUF artifact,
   upstream license, size, hash, and opt-in rerank role.
4. `sources/04-qmd-query-expansion-model.md` - qmd query-expansion GGUF
   artifact, license, size, hash, and hybrid pipeline role.
5. `sources/05-observability-implementation.md` - local CLI observability
   implementation evidence after the observability work landed on master.

## Selection Rationale

The source set follows the active semantic/hybrid search plan. It uses the
model filenames already identified by the accepted search backend eval and the
official project/model pages for licensing, size, hash, and runtime behavior.
The local observation source records the specific `CliContext` and
`search`/`search-all` diagnostics the plan required before Stage 1 can start.

## Gaps

- Installer implementation must pin immutable repository revisions rather than
  `main` URLs before downloads run.
- Installer implementation must verify downloaded bytes against the recorded
  artifact hashes in the managed runtime home.
- qmd-rs source-level resolver behavior still needs implementation inspection
  before deciding whether all model bytes can be forced under `~/.llm_wiki` or
  whether an `external-dependencies.toml` bridge is required.
