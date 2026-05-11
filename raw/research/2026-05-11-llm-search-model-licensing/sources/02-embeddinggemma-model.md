# Source 02: EmbeddingGemma GGUF Candidate

- Source URLs:
  - https://huggingface.co/ggml-org/embeddinggemma-300M-GGUF
  - https://huggingface.co/ggml-org/embeddinggemma-300M-GGUF/tree/main
  - https://huggingface.co/ggml-org/embeddinggemma-300M-GGUF/blob/main/embeddinggemma-300M-Q8_0.gguf
  - https://huggingface.co/google/embeddinggemma-300m
  - https://ai.google.dev/gemma/terms
- Retrieved: 2026-05-11
- Source type: Model card and terms reference

## Relevant Facts

- Candidate repository: `ggml-org/embeddinggemma-300M-GGUF`.
- Candidate file: `embeddinggemma-300M-Q8_0.gguf`.
- Candidate role: embedding model for semantic and hybrid modes.
- File page reported size: 329 MB.
- File page reported SHA-256:
  `f470220f84b6235197541352d22f10bf00098a8242c18eaacea9c8a4add557bc`.
- File page reported Xet hash:
  `2e463c12c4d9b8970e650867162d7212b4006d7e6470d48c541cfcc113facff8`.
- Model tree identifies the base model as `google/embeddinggemma-300m`.
- The upstream Google model card declares license `gemma`, requires users to
  review and agree to Google's usage license on Hugging Face before file
  access, and reports output embedding dimension 768 with truncation options.

## Implication For llm-wiki

EmbeddingGemma cannot be treated like a plain permissive artifact. The
interactive install flow must present the Gemma terms, record explicit user
acknowledgement, download only after consent, and store the accepted-license
record with the artifact hash under `~/.llm_wiki`.
