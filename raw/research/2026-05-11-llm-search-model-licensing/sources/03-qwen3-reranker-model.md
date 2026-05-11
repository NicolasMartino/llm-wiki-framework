# Source 03: Qwen3 Reranker GGUF Candidate

- Source URLs:
  - https://huggingface.co/ggml-org/Qwen3-Reranker-0.6B-Q8_0-GGUF
  - https://huggingface.co/ggml-org/Qwen3-Reranker-0.6B-Q8_0-GGUF/tree/main
  - https://huggingface.co/ggml-org/Qwen3-Reranker-0.6B-Q8_0-GGUF/blob/main/qwen3-reranker-0.6b-q8_0.gguf
  - https://huggingface.co/Qwen/Qwen3-Reranker-0.6B
- Retrieved: 2026-05-11
- Source type: Model card and file metadata

## Relevant Facts

- Candidate repository: `ggml-org/Qwen3-Reranker-0.6B-Q8_0-GGUF`.
- Candidate file: `qwen3-reranker-0.6b-q8_0.gguf`.
- Candidate role: optional reranker when `--rerank` is requested.
- Repository page and upstream Qwen card declare license `apache-2.0`.
- File tree reported size: 639 MB.
- File page reported SHA-256:
  `22c9979ce4fbcdc5acdc310c6641c32797eff1aa980b8f7a2db8a8ea23429a48`.
- File page reported Xet hash:
  `19585ec8f4865d3bcb33b4e26b1cde1e6eb62b69ed02f53f718be293b7379ed7`.
- The GGUF model tree identifies `Qwen/Qwen3-Reranker-0.6B` as the finetuned
  source and `Qwen/Qwen3-0.6B-Base` as the base model.
- The upstream model is a text-ranking model with 0.6B parameters.

## Implication For llm-wiki

The reranker is license-compatible as an Apache-2.0 model candidate, but it
should remain outside the baseline hybrid readiness set. Install downloads it
only when the selected profile includes reranking or the user later enables
rerank support.
