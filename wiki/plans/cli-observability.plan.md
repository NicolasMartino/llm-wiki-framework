# Plan: CLI Verbose Diagnostics

- Document Class: Plan
- Status: Active
- Date: 2026-05-11
- Category: CLI UX, operational diagnostics
- Scope: Implement staged `llm-wiki -v/--verbose` diagnostics, proving the shared surface first with `search` and `search-all`, then extending concise command-specific diagnostics across the binary.
- Sources: wiki/proposals/cli-observability.proposal.md, wiki/decisions/llm-wiki-binary-distribution.decision.md, wiki/decisions/binary-path-bootstrap.decision.md, wiki/specs/documentation-model.spec.md, src/cli.rs, src/main.rs, src/search/commands.rs, src/search/qmd_rs.rs, src/search/sanitize.rs, src/search/adapter.rs, src/registry/mod.rs, tests/search_commands.rs, tests/install.rs, tests/status_doctor.rs, tests/registry.rs, tests/build.rs, tests/init.rs
- Related: wiki/proposals/cli-observability.proposal.md, wiki/plans/project-registry-search-artifacts.plan.md, README.md

## Deliverable

`llm-wiki` gains a global `-v/--verbose` diagnostic surface. Normal command
results remain stable on stdout, while verbose diagnostics explain the paths,
project selection, state checks, query normalization, and command decisions on
stderr.

Stage 1 is the implementation starting point: add the shared CLI output/context
boundary and prove it through `search` and `search-all`. Stage 2 extends one
concise command-specific diagnostic to every other binary command.

The first externally observable result is:

```text
llm-wiki -v search "query" --project <id>
llm-wiki search "query" --project <id> -v
llm-wiki -v search-all "query"
```

Both commands print the same result output as before while stderr explains the
selected project(s), registry/index paths, backend/index state, raw and FTS
queries, filters, limits, result counts, and no-result cases.

## In Scope

- Add a global root CLI flag: `-v` / `--verbose`, accepted before or after the
  subcommand through Clap global-argument semantics.
- Add a shared `CliContext` boundary passed into command handlers.
- Route verbose diagnostics through `tracing` / `tracing-subscriber`, formatted
  to stderr by the CLI. Do not implement verbose diagnostics with ad-hoc
  `eprintln!` calls.
- Add Stage 1 diagnostics for `search` and `search-all`.
- Preserve existing non-verbose stdout output and failure semantics.
- Keep JSON result output valid JSON with no verbose text or ANSI sequences.
- Use the existing qmd-rs search path; verbose output explains retrieval, not a
  new retrieval algorithm.
- Add integration tests using redirected `HOME` and temp project fixtures.
- Extend Stage 2 diagnostics across `build`, `install`, `init`, `register`,
  `forget`, `projects`, `index`, `index-all`, `path`, `status`, `doctor`, and
  `uninstall`.
- Update README and active specs only after verified user-visible behavior
  exists.

## Out Of Scope

- Remote telemetry.
- Persistent log files.
- `--quiet`.
- `--dry-run`.
- A new structured output format.
- Richer default summaries.
- Semantic, hybrid, stopword, or reranking changes to search.
- Making agent-owned `wiki-ingest`, `wiki-query`, or `wiki-lint` operations part
  of the binary.
- A separate retrospective `--explain` mode.
- A rich numeric exit-code taxonomy.

## Existing Implementation Touchpoints

Inspect and change these first:

- `src/cli.rs` - root parser and command args. It currently has only a
  subcommand field on `Cli`, so the global verbose flag starts here.
- `src/main.rs` - command dispatch. It currently calls handlers directly; this
  is where the shared output/context object should be created and passed.
- `src/search/commands.rs` - Stage 1 command behavior. It selects projects,
  reads registry state, resolves qmd-rs store paths, checks backend status,
  runs `search` / `search-all`, prints text/JSON results, and has the best
  place to compute command-owned diagnostic facts.
- `src/registry/mod.rs` - registry path, registered project metadata,
  `record_index_success`, indexed-file counts, and project selection inputs.
- `src/search/adapter.rs` - `BackendStatus`, `BackendState`, and `Freshness`
  are the search-state facts diagnostics should report.
- `src/search/sanitize.rs` and `src/search/qmd_rs.rs` - current FTS query
  sanitization. Do not duplicate normalization logic in the formatter; expose or
  reuse the same sanitized query used by the backend.
- `tests/search_commands.rs` - Stage 1 fixture and assertion home. It already
  uses redirected `HOME`, temp projects, registration, indexing, JSON parsing,
  and two-project `search-all` fixtures.
- `tests/install.rs`, `tests/status_doctor.rs`, `tests/registry.rs`,
  `tests/build.rs`, and `tests/init.rs` - Stage 2 verbose assertions should land
  beside existing command fixtures rather than in one oversized test file.
- `Cargo.toml` - add `tracing` and `tracing-subscriber`.

## Implementation Constraints

1. The root verbose flag must use Clap global-argument semantics so both
   `llm-wiki -v search ...` and `llm-wiki search ... -v` work.
2. The central abstraction is named `CliContext`.
3. Verbose diagnostic events flow through `tracing` and are formatted by the CLI
   subscriber. Human result summaries stay explicit in command code.
4. Verbose diagnostics go to stderr. Result output stays on stdout.
5. Non-verbose output stays compact and unchanged except for bugs discovered
   while implementing the plan.
6. JSON stdout must remain parseable by `serde_json` and must not contain ANSI
   escape sequences.
7. TTY and color behavior inherits the proposal: human diagnostics may use color
   only when stderr is attached to a TTY, non-TTY output is plain text by
   default, `NO_COLOR` disables color, `CLICOLOR_FORCE` may force color, and JSON
   output never contains ANSI escape sequences.
8. `--help` and `--version` are exempt from verbose diagnostics.
9. `--verbose` must not alter exit success or failure.
10. `--verbose` must not enable dependency logs by default. `RUST_LOG` remains
   the explicit escape hatch for lower-level crate diagnostics.
11. If `Paths::from_env()` or equivalent setup fails before project resolution,
    emit only facts already known from parsed CLI arguments and then propagate
    the error. Do not invent unresolved paths or project state.
12. Tests should assert essential diagnostic facts, not full diagnostic wording.
13. Command handlers should compute report data from real decisions and state.
   Formatters must not re-resolve paths or duplicate command logic.

## Stage 1 - Shared Surface And Search Proof

1. Add root CLI fields:

   ```rust
   #[arg(short = 'v', long = "verbose", global = true)]
   verbose: bool
   ```

   Required forms:

   ```text
   llm-wiki -v <command>
   llm-wiki --verbose <command>
   llm-wiki <command> -v
   llm-wiki <command> --verbose
   ```

2. Introduce `CliContext` with at least:

   ```text
   verbose: bool
   ```

   Prefer a small method such as `diagnostic(...)` or `verbose(...)` so command
   modules do not scatter direct stderr writes.

3. Update every command handler signature in Stage 1 so `main.rs` passes the
   same `CliContext` everywhere. The first behavioral diagnostics still land
   only in `search` and `search-all`, but the context plumbing should be
   complete before Stage 2.

4. Add `tracing` and `tracing-subscriber`, install a CLI subscriber from
   `main.rs`, and keep dependency-level logs disabled unless `RUST_LOG` is set.

5. For `search`, compute and emit diagnostics for:

   - registry path from `Paths::project_registry()`
   - project selection source from `SearchArgs::project` or CWD discovery
   - selected project ID, name, root, and wiki root from `RegisteredProject`
   - qmd-rs store path from `Paths::qmd_rs_store_path(project.id)`
   - backend mode from the adapter contract (`qmd-rs fts`)
   - index state, freshness, indexed file count, and stale/unusable reason from
     `BackendStatus`
   - raw query from CLI args and sanitized FTS query from `sanitize_fts_query`
   - class/status filters and limit from CLI args / `SearchFilters`
   - result count from `SearchExecution`
   - no-result explanation using the taxonomy below

6. For `search-all`, compute and emit diagnostics for:

   - registry path from `Paths::project_registry()`
   - selected project IDs, names, roots, and wiki roots from `RegisteredProject`
   - whether selection came from all registered projects or include/exclude
     filters
   - qmd-rs store path per project from `Paths::qmd_rs_store_path(project.id)`
   - backend/index state per project from `BackendStatus`
   - raw query from CLI args and sanitized FTS query from `sanitize_fts_query`
   - class/status filters and limit from CLI args / `SearchFilters`
   - per-project result counts from each `SearchExecution` before fusion
   - final fused result count and no-result explanation

   Per-project diagnostics must be emitted in deterministic project ID order,
   matching current `select_projects` sorting.

7. No-result explanations must distinguish:

   - zero terms after FTS sanitization
   - ready index with zero indexed files
   - backend returned zero hits before filters
   - filters excluded all matched hits
   - stale-but-searchable index produced zero results

   Missing, corrupt, schema-mismatched, or unavailable indexes remain error
   paths; verbose mode should add context but must not turn them into successful
   no-result cases.

8. Keep existing text and JSON result printers as the stdout contract. If search
   execution needs more report data, add internal structs beside
   `SearchExecution` rather than smuggling diagnostics through result output.

9. Before adding verbose behavior, capture current non-verbose `search` and
   `search-all` stdout behavior as regression assertions in
   `tests/search_commands.rs`.

10. Add tests in `tests/search_commands.rs`:

   - `llm-wiki -v search ...` emits diagnostics on stderr and normal results on
     stdout.
   - `llm-wiki search ... -v` is accepted and equivalent to pre-subcommand
     `-v`.
   - `llm-wiki -v search ... --format json` leaves stdout parseable as JSON and
     keeps verbose text on stderr.
   - `llm-wiki -v search-all ...` emits selected project and per-project
     diagnostic facts.
   - zero-result `search` or `search-all` emits a diagnostic result count and
     no-hit explanation while preserving the existing stdout result shape.
   - non-verbose search tests remain green.

## Stage 2 - Full Command Coverage

After Stage 1 is green, extend the same context boundary to every command.

Minimum command-specific diagnostics:

- `build`: selected build target, output directory, rendered runtime skill counts.
- `install`: current executable path, managed binary path/hash comparison,
  partial marker recovery decision, collision classification, render target per
  skill/runtime.
- `uninstall`: manifest path, each file considered, drift checks, managed-binary
  inclusion decision.
- `init`: project path, blueprint, resolved packs, registration path, copied
  initial source paths.
- `register` / `forget` / `projects`: registry path, requested project ID/path,
  canonical root, validation result, cache deletion decision.
- `index` / `index-all`: project ID, wiki root, store path, lock path, temp store
  path, indexed file count, promotion steps, metadata path, per-project outcome.
- `path` / `status` / `doctor`: managed home, managed binary path, manifest path,
  registry path where relevant, cache/index/model paths where relevant.

Stage 2 tests should prove every command accepts `--verbose` and emits at least
one useful command-specific diagnostic without over-specifying incidental
wording. Put those assertions beside each command's existing fixture coverage:
`tests/build.rs`, `tests/init.rs`, `tests/install.rs`, `tests/registry.rs`,
`tests/status_doctor.rs`, and any command-specific search tests already in
`tests/search_commands.rs`.

## Verification Gates

Stage 1 gates:

1. `cargo fmt`
2. `cargo test --test search_commands`
3. `cargo test --workspace`
4. `just verify`
5. `git diff --check`

Stage 2 gates:

1. `cargo fmt`
2. `cargo test --workspace`
3. `just verify`
4. targeted assertions for every verbose command
5. `git diff --check`

## Pages To Update On Completion

After Stage 1 lands:

- `wiki/plans/cli-observability.plan.md` - record Stage 1 proof and keep status
  Active if Stage 2 remains.
- `wiki/index.md` - update the active plan summary if Stage 1 materially changes
  status, proof, or remaining scope.
- `wiki/log.md` - implementation update.

After Stage 2 lands:

- `wiki/plans/cli-observability.plan.md` - Status -> Completed.
- `wiki/proposals/cli-observability.proposal.md` - remains Accepted; verify the
  accepted outcome still matches implementation.
- `wiki/specs/documentation-model.spec.md` - record validated CLI diagnostics
  only after tests prove behavior.
- `README.md` - document the user-visible `-v/--verbose` surface.
- `wiki/index.md` - update plan status and summaries.
- `wiki/log.md` - completion entry.

## What Closes The Plan

The whole plan closes when every `llm-wiki` binary command accepts global
`-v/--verbose`, each command emits at least one command-specific diagnostic that
explains its resolution path or inspected state, search/search-all diagnostics
cover the motivating retrieval confusion, JSON stdout remains clean, and README
plus active specs document the verified behavior.
