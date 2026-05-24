# Plan: GGUF Runtime Portability

- Document Class: Plan
- Status: Active
- Date: 2026-05-24
- Category: Search runtime, semantic retrieval, cross-platform support
- Scope: Make GGUF-backed semantic and hybrid search reliable across supported platforms by adding typed runtime failures, runtime smoke probes, and a CPU-safe execution baseline before any hybrid-quality redesign.
- Sources: user request 2026-05-24 for a detailed plan; review feedback 2026-05-24 on the first GGUF runtime portability / hybrid quality draft; dogfood search pass 2026-05-24 against this repository; Stage 0/1 runtime-boundary implementation pass 2026-05-24; wiki/decisions/semantic-hybrid-search-mode.decision.md; wiki/evals/natural-language-search.eval.md; wiki/evals/natural-language-search-impact.md; wiki/references/llm-search-model-licensing.reference.md; wiki/proposals/full-windows-support.proposal.md; wiki/proposals/search-model-selection.proposal.md; src/search/gguf_runtime.rs; src/search/semantic.rs; src/search/commands.rs; tests/search_commands.rs; qmd 0.3.2 local crate source inspection
- Related: wiki/plans/semantic-hybrid-search.plan.md, wiki/decisions/semantic-hybrid-search-mode.decision.md, wiki/evals/natural-language-search.eval.md, wiki/evals/natural-language-search-impact.md, wiki/references/llm-search-model-licensing.reference.md, wiki/proposals/full-windows-support.proposal.md, wiki/proposals/search-model-selection.proposal.md, wiki/checklists/observability-contract.checklist.md

## Deliverable

`llm-wiki` has an explicit GGUF runtime layer for semantic embeddings, hybrid
query expansion, and future reranking. The layer distinguishes verified model
artifacts from runtime execution, reports typed runtime failures, and provides a
CPU-safe baseline so semantic/hybrid search can work on headless, sandboxed,
CI, Linux, Windows, and non-accelerated environments.

Acceleration remains an optimization. A platform support claim must not depend
on Metal, CUDA, Vulkan, ROCm, or another accelerator being available.

The first observed failure to explain is:

```text
ggml_metal_init: error: failed to create command queue
llama_init_from_model: failed to initialize the context: failed to initialize backend
```

This plan does not assume that the failure is a broken model install. The model
artifacts and accepted-license records can be valid while llama.cpp/qmd runtime
initialization still fails.

## In Scope

- Add an internal GGUF runtime boundary for embedding, query expansion, and
  later reranking.
- Classify runtime failures that currently leak as raw qmd/llama.cpp errors.
- Add structured runtime metadata to JSON and verbose diagnostics.
- Add runtime smoke probes for model load, context creation, and one short
  inference call.
- Prove or disprove whether zero GPU layers plus disabled context offload is
  sufficient to avoid Metal initialization in the failing environment.
- Preserve install-owned model materialization, license acceptance, immutable
  revision pinning, SHA-256 verification, and no hidden downloads.
- Preserve deterministic test hooks for embeddings and query expansion.
- Keep semantic/hybrid readiness fail-closed when artifacts, licenses,
  thresholds, or indexes are missing or stale.
- Separate automated CI checks from manual real-model eval runs.
- Record platform proof for CPU semantic/hybrid execution before making any
  support claim.

## Out Of Scope

- Hosted LLM APIs.
- Silent model downloads from `search`, `search-all`, `index`, `index-all`, or
  `doctor`.
- Lexical-only natural-language improvements.
- Hybrid gating redesign before real GGUF runtime evidence exists.
- Page-level embeddings, semantic sidecar schema changes, or chunking strategy
  changes.
- Making accelerator support a prerequisite for platform support.
- Claiming full Windows support before the separate Windows/runtime-path
  deliverable is implemented and verified.
- Promoting reranking into the default profile without a separate calibrated
  eval result.
- Replacing qmd-rs FTS or the existing immutable completed-store read path.

## Current Evidence

The 2026-05-24 dogfood pass used this repository as the corpus with an isolated
runtime home under `/private/tmp`. Lexical search, semantic search with
deterministic embeddings, stale-state reporting, transient publication
reporting, unreadable cache-directory reporting, and `search-all` partial
failure behavior worked.

The real GGUF path did not run to completion. It failed while creating a
llama.cpp context after Metal initialization failed. Because the run happened
inside a sandboxed Codex environment, this may be a sandbox/headless GPU access
symptom rather than a normal-terminal macOS failure.

The current code has no runtime-readiness layer. `readiness_failure` checks
profile, artifact records, file existence, accepted licenses, thresholds,
semantic metadata, freshness, and vector compatibility before runtime. It does
not load GGUF files or create llama.cpp contexts. Runtime failures therefore
are not misclassified as `model_missing`; they are unclassified runtime errors.

The same dogfood pass observed that deterministic hybrid returned zero results
for `what is project update` while deterministic semantic found
`wiki/proposals/project-update-command.proposal.md`. That is not enough
evidence to change hybrid gating. The deterministic embedding implementation is
a hash-based bag-of-words vector, and the real GGUF runtime never executed.

A follow-up dogfood pass on 2026-05-24 after the qmd-rs metadata v2 rebuild
added the missing non-sandbox evidence. Running `cargo run -- index --force`
from this repository rebuilt the current project successfully with the real
GGUF embedding model and Metal device `MTL0` on Apple M1 Pro, indexing 73 wiki
files. Running `cargo run -- search --mode hybrid --format json "what is
project update"` in the normal terminal path returned
`wiki/proposals/project-update-command.proposal.md` as the top result with
semantic rank 0 and semantic score 0.5618374943733215. That confirms the
deterministic hybrid miss does not reproduce under the real runtime on this
machine.

Before the runtime-boundary implementation, the same hybrid command failed when
stdout/stderr were redirected to files for a JSON-parse check. In that
redirected/headless-shaped execution, llama.cpp reported
`ggml_metal_init: error: failed to create command queue`, returned
`Failed to create context`, and emitted no JSON stdout. This confirmed the
portability issue was not just a sandbox artifact: runtime failures could still
occur in non-interactive execution shapes, and the command path let raw
llama.cpp output replace the parseable JSON contract.

After Stage 2, redirected/headless hybrid search with the real managed models
now succeeds. The auto path first attempts the accelerator, observes the Metal
command-queue/context failure, reloads the qmd generation and embedding engines
with CPU runtime options, and returns
`wiki/proposals/project-update-command.proposal.md` as the top result. JSON
reports `runtime_backend_requested=auto`, `runtime_backend_used=cpu`, and
`runtime_backend_fallback=true`. Forced CPU mode via
`LLM_WIKI_GGUF_RUNTIME=cpu` runs semantic search without Metal initialization,
assigns model layers to the CPU device, and reports
`runtime_backend_requested=cpu`, `runtime_backend_used=cpu`, and
`runtime_backend_fallback=false`.

## Implementation Progress 2026-05-24

Stage 0, Stage 1, and Stage 2 are implemented for the current qmd-based
runtime. `src/search/gguf_runtime.rs` owns the runtime boundary around qmd
embedding, query-expansion, and rerank engine calls. The boundary preserves
deterministic test paths, classifies runtime failures by role, stage, and kind,
and keeps Metal command-queue / context-creation failures separate from model
artifact readiness failures.

`search` and `search-all` now route semantic and hybrid runtime failures and
runtime success metadata through parseable command reporting. JSON output
includes:

- `runtime_backend_requested`
- `runtime_backend_used`
- `runtime_backend_fallback`
- `runtime_failure_stage`
- `runtime_error_kind`

The implementation also adds runtime backend selection via
`LLM_WIKI_GGUF_RUNTIME=cpu` and a `LLM_WIKI_TEST_GGUF_RUNTIME_FAILURE` test hook
so runtime failure reporting can be exercised without relying on local GPU,
driver, or sandbox behavior.

The qmd 0.3.2 crate is patched locally under `vendor/qmd-0.3.2` through
`[patch.crates-io]`. The patch adds `RuntimeBackend`, `RuntimeOptions`,
engine constructors/reload methods that accept runtime options, CPU model
params with zero GPU layers, CPU device selection through llama.cpp's backend
device list, and CPU context params with KQV/op offload disabled. A reload path
is required because constructing a second qmd engine after the first accelerator
failure can hit llama.cpp backend singleton initialization.

The first post-Stage-1 redirected/headless hybrid dogfood confirmed reporting,
not execution portability:

```text
cargo run -- search --mode hybrid --format json "what is project update" \
  >/private/tmp/llmwiki-hybrid-runtime.json \
  2>/private/tmp/llmwiki-hybrid-runtime.err
```

The command still exits nonzero in this execution shape, but stdout is now
parseable JSON. The JSON reports `readiness_reason=runtime_backend_failed`,
`runtime_backend_requested=auto`, `runtime_backend_fallback=false`,
`runtime_failure_stage=query_expansion`, `runtime_error_kind=context_creation_failed`,
and `backend_status.state=ready`. The stderr log still contains the underlying
`ggml_metal_init: error: failed to create command queue` and
`Failed to create context` messages. That confirms this slice fixed reporting,
not execution portability.

The Stage 2 CPU fallback dogfood then re-ran:

```text
cargo run -- search --mode hybrid --format json "what is project update" \
  >/private/tmp/llmwiki-hybrid-cpu-fallback.json \
  2>/private/tmp/llmwiki-hybrid-cpu-fallback.err

LLM_WIKI_GGUF_RUNTIME=cpu cargo run -- search --mode semantic --format json \
  "what is project update" \
  >/private/tmp/llmwiki-semantic-cpu-forced.json \
  2>/private/tmp/llmwiki-semantic-cpu-forced.err
```

The hybrid command exits successfully and returns two results with the project
update proposal first. The JSON reports `runtime_backend_requested=auto`,
`runtime_backend_used=cpu`, `runtime_backend_fallback=true`, and no runtime
failure fields. The forced CPU semantic command exits successfully, returns ten
semantic results with the project update proposal first, reports
`runtime_backend_requested=cpu`, `runtime_backend_used=cpu`,
`runtime_backend_fallback=false`, and stderr shows CPU layer assignment with
`offloaded 0/... layers to GPU` and no Metal initialization.

Verified so far:

- `cargo fmt --check`
- `cargo check`
- `cargo clippy` (passes with the existing `print_search_json` argument-count warning)
- `cargo test gguf_runtime`
- `cargo test search::commands`
- `cargo test --test search_commands runtime_failure_returns_parseable_json`
- `cargo test --test search_commands`
- `cargo test --test natural_language_search_eval`
- `cargo test`
- `git diff --check`
- `git diff --cached --check`
- real redirected/headless hybrid dogfood after `cargo run -- index --force`
- real redirected/headless hybrid auto-to-CPU fallback dogfood
- real forced-CPU semantic dogfood

Remaining work:

- Install and doctor runtime smoke probes are not implemented yet.
- Cross-platform CPU semantic/hybrid proof is still pending.
- The current automated tests cover typed reporting and deterministic paths;
  real GGUF execution proof remains environment-dependent.

## Execution Plan

### Stage 0 - Runtime Failure Framing

1. Replace any implementation assumption that runtime failures are
   `model_missing` misclassifications with the correct framing:
   runtime errors are currently unclassified post-readiness failures.
2. Add test coverage around the command/reporting layer so a simulated runtime
   failure produces parseable JSON metadata instead of raw unstructured output.
3. Define runtime metadata fields:
   - `runtime_backend_requested`
   - `runtime_backend_used`
   - `runtime_backend_fallback`
   - `runtime_failure_stage`
   - `runtime_error_kind`
4. Define runtime readiness/error reasons:
   - `runtime_backend_failed`
   - `runtime_backend_unavailable`
   - `runtime_backend_fallback`
5. Confirm whether the Metal failure reproduces outside the sandbox on the same
   machine before treating it as a local machine failure.

Gate:

- The plan and tests describe runtime failures as unclassified runtime errors,
  not model-artifact readiness failures.
- The same machine has a recorded sandbox and non-sandbox runtime observation,
  or the missing non-sandbox observation is explicitly called out.

### Stage 1 - GGUF Runtime Boundary

1. Add a module such as `src/search/gguf_runtime.rs`.
2. Route semantic embedding calls from `src/search/semantic.rs` through the new
   boundary instead of constructing `qmd::EmbeddingEngine` directly.
3. Route hybrid query expansion calls from `src/search/commands.rs` through the
   same boundary instead of constructing `qmd::GenerationEngine` directly.
4. Preserve the existing deterministic paths:
   - `LLM_WIKI_TEST_EMBEDDINGS=deterministic`
   - `LLM_WIKI_TEST_QUERY_EXPANSION=deterministic`
5. Define typed errors for runtime initialization, model load, context
   creation, tokenization, decode, embedding extraction, generation, and query
   expansion parsing.
6. Add verbose diagnostics for selected runtime path and fallback decisions.

Gate:

- Existing deterministic semantic/hybrid tests still pass.
- JSON stdout remains parseable when runtime failures occur.
- Runtime errors carry typed metadata rather than only string context.

### Stage 2 - CPU Baseline Probe

The preferred implementation path is to patch or upstream qmd-rs so its
embedding, generation, and rerank engines expose constructors with llama.cpp
model and context options.

The direct `llama-cpp-2` fallback is not assumed to be safe. It requires a
prototype first because `qmd` already owns internal `LlamaBackend::init()` calls
for its model engines, and a direct dependency could introduce version coupling
or backend singleton interactions.

Hypothesis to test:

- model load with zero GPU layers
- context creation with KQV offload disabled
- context creation with operation offload disabled

Outcome: zero GPU layers plus disabled KQV/op offload was insufficient by
itself. llama.cpp still initialized Metal during context creation in the
redirected/headless failure shape. Explicit CPU backend device selection through
qmd runtime options was required and is now patched locally.

Runtime selection order after the probe is proven:

1. If the user, test, or profile explicitly requests CPU, use CPU only.
2. If the profile requests auto, use an accelerator only when the smoke probe
   has proven it usable in the current environment.
3. If accelerator context creation fails, retry CPU only when the probe has
   proven that CPU mode avoids the same backend failure.
4. If CPU fails, return `runtime_backend_failed` with the stage and error
   chain.

Gate:

- The original sandbox failure is reproduced or an equivalent accelerator
  failure is simulated.
- The CPU settings are empirically proven to avoid accelerator initialization.
- Search does not ask the user to reinstall models when only the runtime
  backend failed.

### Stage 3 - Install and Doctor Runtime Probe

1. Extend `install --configure-search` to run a runtime smoke probe after model
   artifacts and accepted licenses are verified.
2. Probe the embedding model with one short embedding call.
3. Probe the query-expansion model with one short generation or expansion call.
4. Record probe results under managed runtime state with a timestamp, binary
   version, qmd-rs version, target triple, model ID, artifact hash, backend
   requested, backend used, and failure stage if any.
5. Treat recorded probe results as advisory only. Hardware, sandboxing, and
   driver access can change between install and search.
6. Extend `doctor` to report:
   - model artifacts verified
   - licenses accepted
   - semantic index state
   - current runtime smoke status
   - advisory last probe status when present
   - accelerator status when present
7. Keep ordinary `search` and `index` lightweight, but make actual runtime
   failures typed and parseable at execution time.

Gate:

- `doctor` distinguishes current probe results from historical probe records.
- `install --configure-search` does not promote a broken enabled profile as
  healthy without recording runtime failure evidence.
- Existing idempotent install and no-hidden-download contracts remain intact.

### Stage 4 - Cross-Platform Runtime Proof

1. Add a CPU semantic/hybrid smoke fixture that can run without a large real
   user project.
2. Separate aspirational platform proof from current automated gates:
   - macOS arm64 CPU semantic search
   - Linux x86_64 CPU semantic search
   - Windows x86_64 CPU semantic search when Windows support is promoted
3. Treat accelerator checks as optional proof rows, not support blockers.
4. Update the Windows support proposal or future roadmap only after the CPU
   runtime path passes on Windows.

Gate:

- Platform claims distinguish CPU-supported from accelerator-supported.
- A failing accelerator does not block basic semantic/hybrid search when the
  CPU path is supported on that platform.

### Stage 5 - Real-Model Hybrid Triage

This is a triage gate, not a committed hybrid redesign.

1. After the real GGUF CPU runtime works, run:

   ```text
   llm-wiki search --mode semantic --format json "what is project update"
   llm-wiki search --mode hybrid --format json "what is project update"
   ```

2. If real-model hybrid returns `wiki/proposals/project-update-command.proposal.md`,
   close the hybrid-quality track with no gating, schema, or threshold changes.
3. If real-model hybrid still fails, record the candidate evidence:
   - lexical rank
   - lexical score
   - semantic rank
   - semantic score
   - title/path/snippet anchor evidence
   - threshold branch that rejected the target
4. Re-run the real managed-model eval before proposing any gate change:

   ```text
   cargo test --test natural_language_search_eval -- --ignored --nocapture
   ```

5. Any hybrid adjustment must preserve documented no-match precision and exact
   identifier preservation. Anchor-based relaxations are suspect because the
   existing calibration explicitly raised the final floor above anchor-leaking
   no-match evidence.
6. If a real-model miss remains after this triage, create a separate proposal
   or plan for hybrid retrieval quality. That separate work may consider
   threshold recalibration, query expansion changes, reranker promotion, or
   page-level semantic evidence, but this plan does not implement them.

Gate:

- The deterministic-only hybrid miss is either dismissed as a test artifact or
  reproduced under the real model runtime with recorded candidate evidence.
- No schema-breaking retrieval architecture change is made in this plan.

## Verification Gates

Automated checks:

- `cargo fmt --check`
- `cargo test search::commands`
- `cargo test --test search_commands`
- `cargo test --test natural_language_search_eval`
- `cargo test --workspace`
- `git diff --check`

The default `cargo test --test natural_language_search_eval` check only proves
that the query table is well formed. It does not prove real retrieval quality.

Manual or environment-dependent real-model checks:

```text
cargo test --test natural_language_search_eval -- --ignored --nocapture
llm-wiki index --project dogfood --force
llm-wiki search --project dogfood --mode semantic --format json "what is project update"
llm-wiki search --project dogfood --mode hybrid --format json "what is project update"
llm-wiki search --project dogfood --mode hybrid --format json "what is project update" > /tmp/llm-wiki-hybrid.json 2> /tmp/llm-wiki-hybrid.err
llm-wiki search-all --mode auto --format json "project update command"
llm-wiki doctor
```

Expected runtime outcome:

- runtime failures are typed and parseable
- doctor reports current runtime probe status
- model-reinstall guidance appears only for artifact/license readiness failures
- CPU support is proven separately from accelerator support

Expected hybrid-triage outcome:

- if real-model hybrid succeeds, no hybrid-quality implementation is done here
- if real-model hybrid fails, evidence is recorded and a separate retrieval
  quality workstream is created

## Evidence To Record

- Sandbox and non-sandbox runtime observations for the original Metal failure,
  when available.
- Redirected/headless stdout JSON behavior for real-model semantic and hybrid
  commands.
- CPU baseline probe output, including whether zero GPU layers prevents
  accelerator initialization.
- Any qmd-rs upstream patch, fork, or direct `llama-cpp-2` prototype result.
- Runtime probe records from install and doctor.
- Cross-platform CPU semantic smoke output when platform proof is attempted.
- Real-model hybrid triage output for `what is project update`.

## Wiki Updates On Completion

- Update this plan's status and outcome.
- Update `wiki/decisions/semantic-hybrid-search-mode.decision.md` if the
  runtime fallback contract changes durably.
- Update `wiki/references/llm-search-model-licensing.reference.md` only if
  model candidates, artifact hashes, runtime dependencies, or resolver behavior
  change.
- Update `wiki/proposals/full-windows-support.proposal.md` if CPU GGUF runtime
  proof changes the Windows support plan.
- Update `wiki/evals/natural-language-search.eval.md` or
  `wiki/evals/natural-language-search-impact.md` only if real-model hybrid
  evidence changes the accepted eval or calibration story.
- Update `wiki/index.md` and `wiki/log.md`.

## Closure Criteria

This plan closes when:

1. GGUF runtime failures are classified separately from model artifact
   readiness.
2. Runtime failure JSON remains parseable.
3. `doctor` reports current runtime probe status clearly.
4. CPU execution is either proven usable for semantic/hybrid search or the plan
   records the required CPU-only build/device-selection follow-up.
5. Accelerator failure no longer produces misleading model-reinstall guidance.
6. Real-model hybrid triage for `what is project update` is recorded.
7. Any remaining hybrid-quality work is either explicitly dismissed as a
   deterministic artifact or moved into a separate proposal/plan.
