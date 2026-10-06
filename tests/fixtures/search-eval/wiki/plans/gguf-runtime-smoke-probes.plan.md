# Plan: GGUF Runtime Smoke Probes

- Document Class: Plan
- Status: Completed
- Date: 2026-05-25
- Category: Search runtime, install diagnostics, doctor diagnostics
- Scope: Add install-time and doctor-time GGUF runtime smoke probes for the managed semantic/hybrid search profile without changing model materialization, hidden-download, search readiness, or hybrid retrieval semantics.
- Sources: user request 2026-05-25; implementation pass 2026-05-25; real managed-model dogfood run 2026-05-25; wiki/plans/gguf-runtime-portability.plan.md; wiki/decisions/semantic-hybrid-search-mode.decision.md; wiki/checklists/observability-contract.checklist.md; wiki/plans/idempotent-search-model-install.plan.md; wiki/plans/noninteractive-llm-search-install.plan.md; src/install.rs; src/doctor.rs; src/paths.rs; src/search/runtime_probe.rs; src/search/gguf_runtime.rs; src/search_models.rs; src/search_profile.rs; src/uninstall.rs; tests/install.rs; tests/status_doctor.rs
- Related: wiki/plans/gguf-runtime-portability.plan.md, wiki/decisions/semantic-hybrid-search-mode.decision.md, wiki/checklists/observability-contract.checklist.md, wiki/plans/idempotent-search-model-install.plan.md, wiki/plans/noninteractive-llm-search-install.plan.md

## Deliverable

`llm-wiki install --configure-search` and enabled-search install paths verify
that the configured GGUF runtime can load the managed model artifacts, create a
context, and run one minimal inference for each required runtime role before
promoting the enabled search profile as healthy.

`llm-wiki doctor` reports both the most recent recorded runtime probe and a
current runtime probe result so a user can distinguish:

1. model artifacts are missing or invalid;
2. licenses/profile/thresholds are missing;
3. the qmd-rs lexical index is unavailable;
4. the GGUF runtime itself cannot execute on the current machine;
5. the runtime executed through accelerator auto mode, CPU fallback, or forced
   CPU mode.

Probe records are advisory. Search, search-all, index, and index-all still use
their own execution-time checks and must not trust a stale install-time probe
as readiness proof.

## Implementation Progress 2026-05-25

The code slice is implemented, verified, and dogfooded with the managed GGUF
artifacts:

- `src/search/runtime_probe.rs` owns the versioned probe store, stale checks,
  deterministic probe hook, role probes, and current-run records.
- `Paths::search_runtime_probes()` maps the managed store to
  `~/.llm_wiki/search-runtime-probes.toml`.
- Enabled-search install writes accepted licenses before materialization,
  artifact records after materialization, then writes runtime probe records
  before writing enabled `search.toml`.
- Failed required probes leave accepted-license and artifact records reusable
  but abort before promoting the enabled search profile.
- `doctor` reports last recorded probe status and current runtime probe status
  without downloading or repairing model artifacts.
- `uninstall --search-artifacts` and full uninstall remove the runtime probe
  store with other managed LLM-search artifacts.
- Test-only hooks cover fake model downloads and deterministic probe
  pass/fail paths without network access or real GGUF loads.

Verified:

- `cargo fmt --check`
- `cargo check`
- `cargo clippy` (passes with the pre-existing `print_search_json`
  argument-count warning)
- `cargo test runtime_probe`
- `cargo test gguf_runtime`
- `cargo test --test install`
- `cargo test --test status_doctor`
- `cargo test --test search_commands`
- `cargo test`
- `git diff --check`

Real managed-model dogfood used an isolated HOME under
`/private/tmp/llmwiki-probe-dogfood.Z4VvGN`, copied the already-installed
managed model artifacts, and forced `LLM_WIKI_GGUF_RUNTIME=cpu` so no active
user cache or project registry was mutated. The run verified:

- non-interactive enabled-search install reused verified artifacts, probed
  embedding and query-expansion, wrote `search-runtime-probes.toml`, and only
  then wrote enabled `search.toml`;
- probe records captured `requested_backend=cpu`, `used_backend=cpu`,
  `fallback=false`, artifact hashes, sizes, target triple, qmd-rs version, and
  probe durations;
- `doctor --verbose` reported the last recorded probe and current probe as
  passed under forced CPU;
- `index --force` produced a fresh qmd-rs store, semantic metadata, and 433
  semantic vectors for 74 indexed wiki files;
- `search --mode hybrid --format json "what is project update"` returned
  `wiki/proposals/project-update-command.proposal.md` first with
  `runtime_backend_requested=cpu`, `runtime_backend_used=cpu`, and
  `runtime_backend_fallback=false`;
- `search --mode semantic --format json "what is project update"` returned the
  same proposal first with a real embedding score of `0.5600948333740234`;
- `search-all --mode hybrid --format json "what is project update"` returned
  the same first result through the cross-project surface;
- forced-CPU stderr showed CPU devices and `offloaded 0/... layers to GPU`, with
  no Metal command-queue failure.

Additional auto-mode dogfood showed `search --mode hybrid` and `doctor
--verbose` still succeed when `LLM_WIKI_GGUF_RUNTIME` is unset in the sandbox:
the JSON/doctor metadata reports `runtime_backend_requested=auto`,
`runtime_backend_used=cpu`, and `runtime_backend_fallback=true`. The same runs
still expose llama.cpp's failed Metal attempt on stderr before fallback; that is
a diagnostics-noise follow-up for the parent portability work, not a blocker for
the install/doctor smoke-probe contract.

Review follow-up on 2026-05-25 tightened the completed slice:

- failed required probes now write a disabled `search.toml` with a
  `runtime_probe_failed:*` reason, so a previously enabled profile is not left
  active after a failed install probe;
- the fake model-download env hook was removed from production code; tests now
  inject reuse/download state without forging durable artifact records;
- probe failure messages truncate on character boundaries;
- probe records carry an explicit `required` flag, with embedding and query
  expansion required and rerank advisory;
- install and doctor use the same probe-target selection path;
- probe store staleness uses Cargo's real build target triple instead of an
  `arch-os` approximation.

Post-review dogfood reran in an isolated HOME at
`/private/tmp/llmwiki-probe-review.Pg1H7U`. Forced-CPU enabled install reused
real managed GGUF artifacts, wrote probe records with
`target_triple = "aarch64-apple-darwin"` and `required = true`, and enabled
`search.toml` after both required probes passed. Forced-CPU index, hybrid
search, semantic search, and search-all all returned the project-update
proposal first with CPU backend metadata and no Metal command-queue logs. A
second install with `LLM_WIKI_TEST_GGUF_RUNTIME_PROBE=fail:embedding` against
that already-enabled config exited nonzero and rewrote both global profiles to
`llm_search_enabled = false` with
`reason = "runtime_probe_failed:embedding:embedding:backend_unavailable"`.

Second review follow-up on 2026-05-25 added a doctor preflight before current
runtime probing: every selected artifact record must point to an existing file
whose SHA-256 still matches `observed_sha256`; otherwise doctor reports a
skipped current probe instead of invoking llama.cpp. Unit tests that set
`LLM_WIKI_TEST_GGUF_RUNTIME_PROBE` now share a crate-level env guard so
install and runtime-probe tests cannot race under parallel execution.

## Non-Negotiable Constraints

1. No command outside install/configure-search downloads model files.
2. Search, search-all, index, index-all, eval, wiki-query, and doctor do not
   repair or materialize model artifacts.
3. Runtime probe records do not replace artifact hashes, accepted-license
   records, scoped threshold records, semantic vector compatibility, or
   qmd-rs completed-store status.
4. `LLM_WIKI_GGUF_RUNTIME=cpu` is honored and recorded as the requested
   backend. A CPU failure must not be reported as `auto`.
5. Auto mode may fall back to CPU for accelerator/context failures through the
   existing `src/search/gguf_runtime.rs` boundary.
6. Normal stdout stays human-readable. JSON stdout from search commands must
   remain parseable and unchanged except where already covered by the runtime
   portability plan.
7. Probe records are versioned and treated as stale when the binary version,
   target triple, qmd-rs version, adapter schema version, profile, model ID,
   artifact hash, or requested backend no longer match.

## In Scope

- Add a versioned runtime probe store under the managed home, for example
  `~/.llm_wiki/search-runtime-probes.toml`.
- Add `Paths::search_runtime_probes()` for that store.
- Add a focused runtime probe module, such as `src/search/runtime_probe.rs`,
  that reuses `src/search/gguf_runtime.rs` instead of constructing qmd engines
  directly.
- Probe the required roles for the active enabled profile:
  - embedding: create engine and run `embed_query("llm wiki runtime probe")`;
  - query expansion: create engine and run one short expansion;
  - rerank: skip for the current balanced profile because the accepted
    semantic/hybrid decision keeps reranking opt-in and the shipped profile has
    no default reranker.
- Record requested backend, used backend, fallback, role, stage, error kind,
  artifact identity, duration, timestamp, and stale/match metadata.
- Integrate install/configure-search so required probe failures prevent writing
  a newly enabled `search.toml`.
- Preserve already accepted licenses and downloaded artifacts when a runtime
  probe fails; the user should not need to download the same bytes again.
- Integrate doctor so it reports last recorded probe status and current probe
  status when the configured profile has the artifacts and licenses needed to
  run.
- Add deterministic test hooks so install and doctor tests can exercise pass
  and fail paths without loading real GGUF files.

## Out Of Scope

- Cross-platform CI proof for Linux, Windows, and macOS release artifacts.
- Any hybrid gating, threshold, chunking, page-embedding, or retrieval quality
  redesign.
- New model profiles or per-project model selection.
- Making reranking part of the default balanced profile.
- Background probe refresh daemons.
- Benchmarking model speed or choosing accelerator policy by performance.
- A new hidden recovery path that downloads or replaces model files outside
  install/configure-search.

## Runtime Probe Store

Add a small versioned store with one current record per `(profile, role,
model_id, artifact_sha256, requested_backend)` tuple.

Suggested shape:

```text
RuntimeProbeStore
  schema_version = 1
  updated_at
  binary_version
  target_triple
  qmd_rs_version
  adapter_schema_version
  records[]

RuntimeProbeRecord
  profile
  role
  model_id
  model_path
  artifact_sha256
  artifact_size_bytes
  requested_backend
  used_backend
  fallback
  outcome = passed | failed | skipped
  failure_stage
  failure_kind
  message
  duration_ms
  probed_at
```

Staleness is computed, not manually edited. A record is stale when any identity
field differs from the current binary/runtime/profile/artifact context.

Do not store full llama.cpp logs. Store a short classified message and rely on
verbose diagnostics or redirected stderr for deeper runtime logs.

## Install Behavior

The enabled-search install flow currently writes accepted licenses, materializes
or reuses models, writes `models/artifacts.toml`, then writes enabled
`search.toml`. Insert runtime probing after model artifact records are written
and before enabled `search.toml` is written.

Required behavior:

1. Build the same required model list from the selected profile.
2. Record accepted-license state after consent and before any download or
   materialization, matching the idempotent install contract.
3. Materialize/reuse models exactly as the idempotent install plan specifies.
4. Write artifact records after successful materialization.
5. Run runtime probes for embedding and query expansion using the managed
   artifact paths.
6. Write the probe store with passed or failed records.
7. If any required probe fails, abort before writing the newly enabled
   `search.toml`; report the role, failure stage, failure kind, requested
   backend, used backend if known, and recovery hint.
8. If probes pass, write enabled `search.toml` and external dependency records
   as today.

The abort-after-artifacts ordering is intentional. Model bytes and license
records are expensive install-owned state and can be reused on the next run;
the enabled profile is the part that must not be promoted as healthy after a
runtime failure.

Disabled-search install paths skip probes and keep any old probe records as
advisory history.

## Doctor Behavior

Extend `print_search_profile_diagnostics` to include a `GGUF runtime:` section
after search profile, accepted license, model artifact, and threshold reporting.

The section should show:

- configured profile and whether LLM search is enabled;
- last recorded probe status, timestamp, profile, backend requested/used, and
  stale reason if stale;
- current probe status for each required runtime role when artifacts and
  licenses are present;
- skipped reason when probes cannot run because profile, licenses, artifacts,
  or model files are missing;
- failure role/stage/kind and concise recovery hint when current probing fails.

Doctor is a diagnostics command, so running the current probe may load the GGUF
models. Keep status/projects/search commands lightweight; do not move current
runtime probing into those commands.

## Observability Contract

Follow `wiki/checklists/observability-contract.checklist.md`:

- normal install/doctor stdout names the status and next action;
- `-v/--verbose` reports resolved paths, selected profile, model IDs, artifact
  hashes, requested backend, used backend, fallback, stale reason, and duration;
- dependency logs stay disabled unless `RUST_LOG` is set;
- stdout remains separate from diagnostics;
- tests assert essential facts, not full prose.

## Execution Plan

### Stage 1 - Store And Staleness

1. Add `Paths::search_runtime_probes()`.
2. Add the probe store structs, read/write helpers, schema validation, and
   atomic TOML write path.
3. Add stale-reason helpers for binary version, target triple, qmd-rs version,
   adapter schema version, profile, role, model ID, artifact hash, and requested
   backend.
4. Add unit tests for round-trip serialization, missing store, unsupported
   schema, and stale-reason precedence.

Gate: stale records are clearly advisory and cannot make readiness succeed.

### Stage 2 - Probe API

1. Add `src/search/runtime_probe.rs`.
2. Resolve active profile requirements from `SearchConfig`, `ModelArtifacts`,
   and `AcceptedLicenses`.
3. Reuse `gguf_runtime::embedding_engine`, `embed_query`,
   `generation_engine`, and `expand_query` so CPU fallback and runtime error
   classification are shared with real search execution.
4. Add a deterministic test hook such as
   `LLM_WIKI_TEST_GGUF_RUNTIME_PROBE=pass|fail:<role>|skip:<role>` for tests
   that must not load real GGUF files.
5. Map runtime errors into probe records without losing role, stage, kind,
   backend requested/used, and fallback.

Gate: probe unit tests cover success, forced role failure, forced CPU backend,
auto-to-CPU fallback metadata, and skipped optional reranker.

### Stage 3 - Install Integration

1. Insert probing between artifact-record writes and enabled `search.toml`
   writes in `configure_enabled_search`.
2. Preserve no-write preflight behavior for missing non-interactive download or
   license confirmations.
3. On probe failure, write failed probe records and return a clear install
   error before enabling LLM search.
4. On probe success, write passed probe records and continue with enabled
   search config.
5. Add verbose diagnostics for probe start, result, duration, and store path.

Gate: install tests prove a failed probe leaves model/license/artifact records
reusable but does not write an enabled `search.toml`.

### Stage 4 - Doctor Integration

1. Read and report the last runtime probe store.
2. Run current probes only when the active profile, accepted licenses, and
   model artifact records make that possible.
3. Report skipped probes when prerequisite state is missing, not as runtime
   failures.
4. Print stale reasons for recorded probes.
5. Add verbose tests for doctor runtime diagnostics.

Gate: doctor distinguishes missing setup from runtime execution failure and
does not download or mutate model artifacts.

### Stage 5 - Verification And Dogfood

1. Run formatting and static checks.
2. Run focused install, doctor, and runtime probe tests.
3. Run the existing search command tests to ensure JSON runtime metadata still
   works.
4. Run a real local dogfood with managed models:
   - `LLM_WIKI_GGUF_RUNTIME=cpu llm-wiki install --configure-search`
   - `llm-wiki doctor -v`
   - `llm-wiki search --mode hybrid --format json "what is project update"`
5. Record the result in the portability plan or a follow-up eval note.

Gate: install/doctor prove runtime execution separately from search execution,
and real search still returns parseable JSON with the existing runtime fields.

## Automated Verification

Minimum local checks before completion:

```text
cargo fmt --check
cargo check
cargo clippy
cargo test gguf_runtime
cargo test runtime_probe
cargo test --test install
cargo test --test status_doctor
cargo test --test search_commands
cargo test
git diff --check
```

If the full suite is too slow during development, run focused tests first, but
the plan is not complete until the full command set above has been attempted
and any skipped or environment-dependent checks are recorded.

## Closure Criteria

This plan is complete when:

1. enabled-search install probes embedding and query-expansion runtime execution
   before writing enabled `search.toml`;
2. failed probes leave reusable artifact/license state but do not promote a
   broken enabled profile as healthy;
3. doctor reports last and current runtime probe status with stale reasons and
   clear skipped states;
4. forced CPU mode is preserved in probe records and failure diagnostics;
5. no command outside install/configure-search downloads, repairs, or replaces
   model artifacts;
6. automated tests cover pass, fail, skipped, stale, forced CPU, and
   auto-fallback metadata paths;
7. a real dogfood run records whether CPU probes and hybrid search work with
   the managed GGUF artifacts.
