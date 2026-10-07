# CLI Verbose Diagnostics

- Document Class: Proposal
- Status: Accepted
- Date: 2026-05-10
- Category: CLI UX, operational diagnostics
- Scope: Add universal `-v/--verbose` diagnostics so every binary command can explain what it resolved or inspected without changing normal command output.
- Sources: conversational input 2026-05-10 and proposal review 2026-05-11; no raw source file captured yet
- Implementation References: src/cli.rs, src/main.rs, src/build.rs, src/install.rs, src/uninstall.rs, src/init/command.rs, src/search/commands.rs, src/registry/mod.rs, src/path_guidance.rs, src/status.rs, src/doctor.rs
- Related: wiki/decisions/llm-wiki-binary-distribution.decision.md, wiki/decisions/binary-path-bootstrap.decision.md, wiki/plans/llm-wiki-binary.plan.md, wiki/specs/documentation-model.spec.md
- Promoted To: wiki/plans/cli-observability.plan.md
- Promotion Target: wiki/specs/documentation-model.spec.md, README.md

## Question

Should every `llm-wiki` command expose a consistent `-v/--verbose` mode so
operators can see which paths, projects, indexes, inputs, and decisions the
binary used?

## Proposal

Yes. Add global `-v/--verbose` diagnostics for every command in the binary.

The minimum product requirement is that every command in the binary supports
`--verbose` and reports the command-specific resolution path that determines
behavior. Other observability features can wait until the shared verbose
surface exists.

The first implementation target should be `search`, because a real project
query produced confusing output: the user needed to know which project was
selected, which registry and index paths were used, what the index status was,
how the natural-language query was normalized for FTS, and why no hits were
returned. The same model should then extend across the full command set:
`build`, `install`, `init`, `register`, `forget`, `projects`, `index`,
`index-all`, `search`, `search-all`, `path`, `status`, `doctor`, and
`uninstall`.

This proposal is not about adding telemetry or remote logging. All output stays
local to the command invocation.

Accepted outcome: implement this through
`wiki/plans/cli-observability.plan.md` as a staged plan rather than a single
all-command patch:

1. **Stage 1 - shared verbose surface plus search proof.** Add the global flag,
   shared CLI output/context boundary, and verbose diagnostics for `search` and
   `search-all`. Prove default stdout and JSON output remain stable.
2. **Stage 2 - full command coverage.** Extend one concise command-specific
   diagnostic to the remaining commands after the shared surface is proven.

This keeps the product promise universal while making the first deliverable
small enough to verify in one implementation slice.

Implementation check, 2026-05-11: the accepted staged outcome matches the
completed implementation in `wiki/plans/cli-observability.plan.md`. Stage 1
landed the global flag, shared `CliContext`, tracing stderr diagnostics, and
`search` / `search-all` proof. Stage 2 extended command-specific diagnostics
to every remaining binary command and added targeted command fixture tests.

Deferred surfaces:

- `--quiet`
- `--dry-run`
- richer default summaries
- additional structured output beyond existing `--format text|json` surfaces
- a separate `--explain` retrospective mode

## Output Model

Separate three kinds of output:

1. **Result output**: stable command results intended for humans or scripts.
   This remains `stdout` and should not change just because verbose mode exists.
2. **Verbose diagnostics**: phase information, path resolution, project
   selection, normalization, inspected state, decisions, skipped work, and
   warnings. This goes to `stderr` or a tracing subscriber configured for CLI
   diagnostics.
3. **Errors**: actionable failure messages with enough context to recover.
   Existing `anyhow::Context` remains the main error-enrichment mechanism.

Default output should stay compact. `--verbose` is the place for resolution
details that are essential during debugging but too noisy for ordinary use.

For `install`, verbose output can include internal decisions:

```text
resolved current executable: target/debug/llm-wiki
managed binary differs: copying
rendered wiki-init for claude
rendered wiki-init for codex
collision wiki-query/codex: up-to-date
writing manifest schema_version=2
```

For `search`, verbose output should explain the retrieval target before results:

```text
selected project: electric-car (/Users/nicolasmartino/Documents/car/electric)
registry: ~/.local/share/llm-wiki/projects.json
wiki root: /Users/nicolasmartino/Documents/car/electric/wiki
index store: ~/.cache/llm-wiki/indexes/electric-car/qmd-rs.sqlite
index status: ready, freshness=fresh, indexed_files=14
backend: qmd-rs fts
query: what are the most cutting edge battery technologies
fts query: what are the most cutting edge battery technologies
filters: class=<none>, status=<none>, limit=10
results: 0
```

`query` and `fts query` are both shown intentionally. They may be identical for
simple input, but they diverge when sanitization removes punctuation,
deduplicates repeated tokens, normalizes casing, or later adds stopword or
token rewriting. Showing both makes retrieval behavior debuggable without
guessing what qmd-rs actually received.

## Global Flags

Add a global option on the root CLI:

```text
llm-wiki -v ...
llm-wiki --verbose ...
```

Initial semantics:

- `--verbose`: include phase, path, selection, status, normalization, and
  decision details for every command
- default: preserve existing compact command output unless a command already has
  a documented default summary

Use a single `CliOutput` or `CliContext` object passed to command handlers
rather than ad-hoc flag checks in each module.

Do not make `--verbose` a Cargo-style repeatable verbosity counter in the first
implementation. One verbose level is enough until real usage proves the need
for trace/debug separation.

## TTY And Color

Human output may use color only when stdout/stderr is attached to a TTY.
Piped output must be plain text by default. The CLI should honor `NO_COLOR` by
disabling color, and may honor `CLICOLOR_FORCE` to force color for users who
explicitly request it.

Color is presentation only. Tests should prefer plain output unless they are
specifically covering color detection, and JSON output must never include ANSI
escape sequences.

## Exit Codes

Exit semantics should be consistent across commands:

1. `0` means the requested operation completed successfully.
2. nonzero means the command did not complete as requested.
3. `--verbose` must not change success or failure semantics.
4. The first implementation does not need a rich taxonomy of numeric exit
   codes. It should preserve the current nonzero failure behavior and avoid
   adding command-specific exit meanings until there is a documented need.

## Progress

Long-running commands should not invent their own progress behavior. Default
progress should stay as it is unless a later proposal changes default summaries.
Verbose mode may print phase-level progress such as selected project, file
counts, lock acquisition, temp store path, promotion steps, and per-project
`index-all` outcomes.

Avoid per-file progress by default. If indexing large wikis later needs
spinners or per-file progress, that behavior should be gated by TTY detection
and suppressed for non-TTY output unless explicitly requested.

## Tracing Boundary

Summaries are direct writes through `CliOutput` / `CliContext`; they are the
human command contract. Framework diagnostics gated by `--verbose` should flow
through `tracing` at framework-owned levels, then be formatted by the CLI
subscriber. Dependency-level logs are not enabled by `--verbose`; `RUST_LOG`
remains the escape hatch for lower-level crate diagnostics.

This avoids two drifting logging paths: command handlers compute report data and
emit diagnostic events from the same decisions, while presentation code decides
what reaches stdout/stderr.

## Deferred Surfaces

Do not add `--quiet`, `--dry-run`, broader structured output, or richer default
summaries in the first implementation. They are plausible follow-up work, but
this proposal closes only the missing command diagnostics problem.

Keep the existing `--format text|json` on `projects`, `search`, and
`search-all`. Verbose diagnostics must not pollute JSON result output.

## Implementation Shape

Use `tracing` for diagnostics rather than scattering raw `eprintln!` calls.
The CLI should still own human summaries explicitly through the `CliOutput` /
`CliContext` boundary; `tracing` is for diagnostic events and verbose mode.

Suggested dependencies:

```toml
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["fmt", "env-filter"] }
```

Staged implementation shape:

1. Add a root `CliOutput` or `CliContext` with `verbose`.
2. Add the global flag to `src/cli.rs` and initialize the output/tracing layer in
   `src/main.rs`.
3. Implement focused verbose events for `search` and `search-all`.
4. Keep stdout result output unchanged in non-verbose mode.
5. Add focused tests for `search` / `search-all` verbose diagnostics because
   they are the motivating failure case.
6. Add integration tests proving JSON result output stays free of verbose text
   and ANSI color.
7. Extend command-specific verbose events across the remaining commands.
8. Add integration tests proving every command accepts `--verbose` and emits at
   least one useful command-specific diagnostic.

The core rule: command handlers should compute a report from real decisions,
then a presentation layer prints it. Avoid duplicating decision logic in the
formatter.

## Command-Specific Verbose Targets

Every command must emit at least one command-specific verbose diagnostic. The
first implementation can keep each target concise.

### `build`

- selected build target
- output directory
- rendered runtime skill counts

### `install`

- current executable path
- managed binary path and hash comparison outcome
- partial marker recovery decision
- collision classification per file
- render target per skill/runtime

### `uninstall`

- manifest path
- each file considered
- drift checks
- whether the managed binary is included

### `init`

- project path
- blueprint
- resolved packs
- registration path
- copied initial source paths

### `register` / `forget` / `projects`

- registry path
- requested project ID/path
- resolved canonical root
- validation result
- cache deletion decision for `forget --delete-cache`

### `index` / `index-all`

- project id
- wiki root
- store path
- lock path
- temp store path
- indexed file count
- promotion steps
- metadata path
- per-project `index-all` outcome

### `search` / `search-all`

- registry path
- selected project ID(s), names, and roots
- whether project selection came from `--project`, CWD discovery, or
  `search-all` include/exclude filters
- wiki root for each selected project
- qmd-rs store path for each selected project
- backend mode
- index status, freshness, indexed file count, and stale/unusable reason when
  available
- raw query and sanitized FTS query
- class/status filters and limit
- per-project result count before cross-project fusion
- final result count and no-hit explanation when the result set is empty

### `path` / `status` / `doctor`

- managed home
- managed binary path
- manifest path
- registry path where relevant
- cache/index/model paths where relevant

## Non-Goals

- Remote telemetry.
- Persistent log files.
- A daemon or background service.
- `--quiet`.
- `--dry-run`.
- new structured output formats.
- richer default summaries.
- Making agent-owned operations (`wiki-ingest`, `wiki-query`, `wiki-lint`) part
  of the binary.
- Replacing `doctor`; improved command logging should complement `doctor`, not
  turn every command into a full diagnostic report.
- A separate `--explain` retrospective mode in the first implementation.
  `--verbose` is the diagnostic surface for now.

## Risks

1. **Noisy defaults.** If default output becomes too chatty, scripts and humans
   both suffer. Keep existing default output unchanged and reserve diagnostic
   detail for `--verbose`.
2. **JSON pollution.** Verbose diagnostics must not mix into JSON result output.
   Put diagnostics on stderr and keep structured stdout stable.
3. **Progress output in tests.** Integration tests should assert essential
   verbose diagnostics without coupling to incidental progress wording.
4. **Verbose output as accidental API.** Once integration tests or scripts
   assert verbose lines, those lines become compatibility surface. Tests should
   assert the presence of essential diagnostics without over-specifying
   incidental wording.

## What Closed This Proposal

Promotion to `wiki/plans/cli-observability.plan.md`, which owns the staged
acceptance criteria:

Stage 1:

1. root CLI accepts global `-v/--verbose`
2. command handlers receive verbose/output state through a shared
   `CliOutput`/`CliContext` boundary rather than ad-hoc flag plumbing
3. `search` and `search-all` verbose output explain project selection, registry
   and index paths, backend/index state, query normalization, filters, result
   counts, and zero-result cases
4. verbose diagnostics follow the stdout/stderr,
   TTY/color, tracing, RUST_LOG, progress, and exit-code policies in this
   proposal
5. integration tests cover `search` / `search-all` verbose diagnostics,
   unchanged non-verbose result output, and JSON output staying free of verbose
   text and ANSI color

Stage 2:

1. every `llm-wiki` command accepts global `-v/--verbose`
2. every command emits at least one command-specific verbose diagnostic that
   explains its resolution path or inspected state
3. integration tests cover verbose diagnostics for the full command set
4. README and affected specs document the resulting user-visible behavior

## Open Questions

(none)
