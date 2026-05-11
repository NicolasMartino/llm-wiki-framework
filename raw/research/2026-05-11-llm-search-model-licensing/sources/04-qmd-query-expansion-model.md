# Source 04: QMD Query Expansion GGUF Candidate

- Source URLs:
  - https://huggingface.co/tobil/qmd-query-expansion-1.7B-gguf
  - https://huggingface.co/tobil/qmd-query-expansion-1.7B-gguf/tree/main
  - https://huggingface.co/tobil/qmd-query-expansion-1.7B-gguf/blob/7fa0891c5f335b8f16ba373c480fdde87c1d4fad/qmd-query-expansion-1.7B-q4_k_m.gguf
- Retrieved: 2026-05-11
- Source type: Model card and file metadata

## Relevant Facts

- Candidate repository: `tobil/qmd-query-expansion-1.7B-gguf`.
- Candidate file: `qmd-query-expansion-1.7B-q4_k_m.gguf`.
- Candidate role: query-expansion model required for baseline hybrid mode.
- Repository page declares license `mit`.
- File tree reported the q4_k_m file size as 1.28 GB.
- Pinned file page reported SHA-256:
  `000dfb1c06efa6a049e9f64ba921c3740e2454f62abab6fa10e77bd30bb2bcc0`.
- Pinned file page reported Xet hash:
  `c3815846a946bfc89f95aeed122539a3286c91800084dd71398bcbf19a73f872`.
- The model card describes structured expansion output using `lex:`, `vec:`,
  and `hyde:` lines for QMD's BM25, vector, and HyDE retrieval branches.
- The model card identifies Qwen3 1.7B lineage and a two-stage SFT/GRPO
  training path. The model tree shows base `Qwen/Qwen3-1.7B-Base`, finetuned
  `Qwen/Qwen3-1.7B`, then the quantized GGUF artifact.

## Implication For llm-wiki

Baseline hybrid requires this model when the selected profile uses qmd-rs
query expansion. The installer must present the license and model lineage,
verify the q4_k_m artifact hash after download, and keep downloads explicit.
