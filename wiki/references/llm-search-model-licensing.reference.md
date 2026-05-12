# LLM Search Model Licensing

- Document Class: Reference
- Status: Sourced
- Date: 2026-05-11
- Category: Search model licensing, install state, runtime dependencies
- Scope: Stage 0 source record for semantic/hybrid search model candidates,
  licenses, terms, artifact hashes, download/cache implications, and qmd-rs
  resolver constraints.
- Sources: raw/research/2026-05-11-llm-search-model-licensing/manifest.md, raw/research/2026-05-11-llm-search-model-licensing/research-summary.md, https://docs.rs/qmd/latest/qmd/, https://huggingface.co/ggml-org/embeddinggemma-300M-GGUF, https://huggingface.co/api/models/ggml-org/embeddinggemma-300M-GGUF?blobs=true, https://huggingface.co/google/embeddinggemma-300m, https://ai.google.dev/gemma/terms, https://huggingface.co/ggml-org/Qwen3-Reranker-0.6B-Q8_0-GGUF, https://huggingface.co/Qwen/Qwen3-Reranker-0.6B, https://huggingface.co/tobil/qmd-query-expansion-1.7B-gguf, https://huggingface.co/api/models/tobil/qmd-query-expansion-1.7B-gguf?blobs=true
- Related: wiki/plans/semantic-hybrid-search.plan.md, wiki/proposals/search-query-interpretation.proposal.md, wiki/evals/search-backend-selection.eval.md, wiki/evals/natural-language-search.eval.md, wiki/references/qmd-rs-search-crate.reference.md

## Summary

Stage 0 accepts these initial model candidates for implementation planning:

1. Embedding: `ggml-org/embeddinggemma-300M-GGUF` /
   `embeddinggemma-300M-Q8_0.gguf`.
2. Query expansion: `tobil/qmd-query-expansion-1.7B-gguf` /
   `qmd-query-expansion-1.7B-q4_k_m.gguf`.
3. Optional reranker: `ggml-org/Qwen3-Reranker-0.6B-Q8_0-GGUF` /
   `qwen3-reranker-0.6b-q8_0.gguf`.

These are not bundled with the binary. They are consented install-time
downloads only. `llm-wiki` must pin immutable repository revisions before
materialization, verify local bytes by SHA-256, and record license acceptance
and artifact metadata under `~/.llm_wiki`.

## Candidate Matrix

| Role | Repository and file | License/terms | Size | SHA-256 | Stage 0 decision |
| --- | --- | --- | --- | --- | --- |
| Embedding | `ggml-org/embeddinggemma-300M-GGUF` / `embeddinggemma-300M-Q8_0.gguf` | Upstream `google/embeddinggemma-300m` declares `gemma` and requires agreement to Google's usage license before file access on Hugging Face | 333,590,944 bytes | `b5ce9d77a3fc4b3b39ccb5643c36777911cc4eb46a66962eadfa3f5f60490d63` | Accept as default embedding candidate only through interactive install with explicit Gemma terms acknowledgement |
| Query expansion | `tobil/qmd-query-expansion-1.7B-gguf` / `qmd-query-expansion-1.7B-q4_k_m.gguf` | Hugging Face model page declares `mit`; model card records Qwen3 1.7B lineage | 1,282,438,912 bytes | `000dfb1c06efa6a049e9f64ba921c3740e2454f62abab6fa10e77bd30bb2bcc0` | Accept as baseline hybrid expansion candidate; required for `--mode hybrid` readiness |
| Reranker | `ggml-org/Qwen3-Reranker-0.6B-Q8_0-GGUF` / `qwen3-reranker-0.6b-q8_0.gguf` | Repository and upstream Qwen model card declare `apache-2.0` | 639 MB | `22c9979ce4fbcdc5acdc310c6641c32797eff1aa980b8f7a2db8a8ea23429a48` | Accept as optional `--rerank` candidate; not required for baseline hybrid |

## Current Artifact Verification

The original 2026-05-11 raw source recorded the EmbeddingGemma file page hash
then visible on Hugging Face. During the first consented install attempt on
2026-05-11, the downloaded artifact did not match that older hash. A follow-up
Hugging Face API check for `ggml-org/embeddinggemma-300M-GGUF?blobs=true`
reported repository revision `0f741b5a6585bd53aeb15cd1372c56f2a0f65e12`,
file size `333590944`, and LFS SHA-256
`b5ce9d77a3fc4b3b39ccb5643c36777911cc4eb46a66962eadfa3f5f60490d63`.

The implementation pins that immutable revision and treats the API-reported
hash as the expected install hash. The query-expansion artifact is likewise
pinned to repository revision `7816de0b72572c6c860ca1eddf97ba9e7fb8cc65`,
where the q4_k_m file hash remains
`000dfb1c06efa6a049e9f64ba921c3740e2454f62abab6fa10e77bd30bb2bcc0`.

## Artifact Pinning

Stage 1 must not download from a moving `main` reference. Installer profiles
must record:

- model repository
- immutable repository revision or file commit
- file name and download URL
- expected SHA-256
- observed local SHA-256
- file size
- license or terms identifier
- accepted-license record path and timestamp
- qmd-rs version and adapter schema version that consumed the artifact

The hashes above come from source-reported Hugging Face file metadata. They
are suitable as expected hashes, but they are not a substitute for verifying
the downloaded local file after user consent.

## License And Redistribution Implications

The release artifact must not include these model files. The installer may
download them only after an interactive acknowledgement step.

EmbeddingGemma is the strictest case. Because the upstream model is
Gemma-licensed and gated by Google usage-license acceptance on Hugging Face,
install must record the accepted terms before downloading the GGUF file.

The Qwen reranker is Apache-2.0. The query-expansion artifact declares MIT, but
the install UI should still show its Qwen3 lineage and record the accepted
license metadata because it is a derived model artifact used by the retrieval
pipeline.

## qmd-rs Resolver And Cache Implications

qmd-rs 0.3.2 documents automatic Hugging Face model download plus `resolve_model`,
`pull_model`, and `pull_models` APIs. `llm-wiki` must prevent those helpers
from becoming an implicit download path during ordinary runtime commands.

Implementation rule:

1. `llm-wiki install` and `llm-wiki install --configure-search` are the only
   product-owned surfaces that may materialize model bytes.
2. Search, search-all, index, index-all, doctor, and wiki-query must inspect
   existing managed state and fail with readiness guidance rather than pulling
   models.
3. The preferred byte root is `~/.llm_wiki/models/`.
4. If a qmd-rs helper hardcodes a second cache or model location, record it in
   `~/.llm_wiki/external-dependencies.toml` with cleanup guidance instead of
   leaving it invisible.

## Stage 0 Gate Result

Model candidates, licenses, terms, sizes, hashes, and resolver risks are
documented enough to start Stage 1 design. Stage 1 remains blocked from
actually downloading or accepting a completed LLM-search profile until it
implements consent, immutable revision pinning, hash verification, and managed
runtime records.
