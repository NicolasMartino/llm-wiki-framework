# CLI Observability and Dry-Run UX

- Document Class: Proposal
- Status: Proposed
- Date: 2026-05-10
- Category: CLI UX, operational diagnostics
- Scope: Improve `llm-wiki` command-line feedback so real installs, indexing, scaffolding, and removals explain what changed, what was skipped, and what to do next.
- Sources: user discussion 2026-05-10, src/cli.rs, src/install.rs, src/uninstall.rs, src/init/command.rs, src/search/commands.rs, src/registry/mod.rs, wiki/decisions/llm-wiki-binary-distribution.decision.md, wiki/decisions/binary-path-bootstrap.decision.md, wiki/specs/documentation-model.spec.md
- Related: wiki/decisions/llm-wiki-binary-distribution.decision.md, wiki/decisions/binary-path-bootstrap.decision.md, wiki/plans/llm-wiki-binary.plan.md, wiki/specs/documentation-model.spec.md

## Question

After running a real install from the debug binary, should `llm-wiki` expose a
clearer logging and preview model for operators, instead of relying on sparse
command output plus errors?

## Proposal

Yes. Add a small, consistent CLI observability layer with four parts:

1. concise default summaries for mutating commands
2. global `-v/--verbose` and `--quiet`
3. `--dry-run` for destructive or high-impact commands
4. structured output where it already fits command semantics

The first target is `install`, because it mutates the user's real
`~/.llm_wiki/`, `~/.claude/skills/`, and `~/.codex/skills/` state. The same
model should then extend to `uninstall`, `init`, `index`, and `index-all`.

This proposal is not about adding telemetry or remote logging. All output stays
local to the command invocation.

## Output Model

Separate three kinds of output:

1. **Result output**: stable command results intended for humans or scripts.
   This remains `stdout`.
2. **Progress and diagnostics**: phase information, decisions, skipped work,
   and warnings. This goes to `stderr` or a tracing subscriber configured for
   CLI diagnostics.
3. **Errors**: actionable failure messages with enough context to recover.
   Existing `anyhow::Context` remains the main error-enrichment mechanism.

Default output should be useful without being noisy. A successful `install`
should say what state it touched:

```text
Installing llm-wiki 0.1.0
Managed binary: ~/.llm_wiki/bin/llm-wiki (copied)
Claude skills: 5 written, 0 unchanged
Codex skills: 6 written, 0 unchanged
Manifest: ~/.llm_wiki/manifest.json
Done.
```

`--verbose` adds internal decisions:

```text
resolved current executable: target/debug/llm-wiki
managed binary differs: copying
rendered wiki-init for claude
rendered wiki-init for codex
collision wiki-query/codex: up-to-date
writing manifest schema_version=2
```

`--quiet` suppresses non-error output. It is for scripts and tests that care
only about exit status or structured output.

## Global Flags

Add global options on the root CLI:

```text
llm-wiki -v ...
llm-wiki --verbose ...
llm-wiki --quiet ...
```

Initial semantics:

- default: print concise summaries for commands that change state
- `--verbose`: include phase and decision details
- `--quiet`: suppress summaries and diagnostics except errors
- `--quiet` and `--verbose` conflict

Use a single `CliOutput` or `CliContext` object passed to command handlers
rather than ad-hoc flag checks in each module.

Do not make `--verbose` a Cargo-style repeatable verbosity counter in the first
implementation. One verbose level is enough until real usage proves the need
for trace/debug separation.

## Dry Run

Add `--dry-run` to commands where a preview materially reduces risk:

```text
llm-wiki install --dry-run
llm-wiki uninstall --dry-run
llm-wiki uninstall --include-binary --dry-run
llm-wiki index --dry-run
llm-wiki index-all --dry-run
```

`install --dry-run` should perform discovery, rendering, collision
classification, and manifest comparison, then print the planned changes without
writing files:

```text
Would copy binary to ~/.llm_wiki/bin/llm-wiki
Would write 5 Claude skill files
Would write 6 Codex skill files
Would write manifest ~/.llm_wiki/manifest.json
No changes made.
```

`uninstall --dry-run` should list manifest-owned files and whether drift would
block a real uninstall.

`index --dry-run` should report the selected project, target store path, current
freshness, and file count that would be indexed. It should not build a temp
store.

`init --dry-run` is useful but lower priority: its output would be a planned
file/folder list and resolved blueprint/packs. It can follow after install and
uninstall because `init` normally targets an explicit new project path and
already refuses framework-artifact collisions.

## Structured Output

Do not force JSON onto every command immediately. Keep the existing
`--format text|json` on `projects`, `search`, and `search-all`.

Add JSON later where the command has a stable result object:

- `status --format json`
- `doctor --format json`
- `install --dry-run --format json`
- `uninstall --dry-run --format json`

Avoid promising JSON for real `install` or `uninstall` until the operation
summary structs are stable. Preview mode is the safer first structured surface.

## Implementation Shape

Use `tracing` for diagnostics rather than scattering raw `eprintln!` calls.
The CLI should still own human summaries explicitly; `tracing` is for
diagnostic events and verbose mode.

Suggested dependencies:

```toml
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["fmt", "env-filter"] }
```

Implementation steps:

1. Add a root `CliOutput` context with `verbose` and `quiet`.
2. Add global flags to `src/cli.rs` and initialize the output/tracing layer in
   `src/main.rs`.
3. Refactor `install` to return an `InstallReport` containing binary action,
   per-runtime skill counts, manifest path, backup path if any, warnings, and
   dry-run status.
4. Add `install --dry-run` using the existing preflight and collision
   classification machinery without writing the managed binary, skill files,
   backups, manifest, or partial marker.
5. Add concise default `install` summaries and verbose diagnostics.
6. Add `uninstall --dry-run` and an `UninstallReport`.
7. Extend the same report pattern to `init`, `index`, and `index-all`.
8. Update integration tests to assert summary output, quiet output, dry-run
   non-mutation, and verbose detail.

The core rule: command handlers should compute a report from real decisions,
then a presentation layer prints it. Avoid duplicating decision logic in the
formatter.

## Command-Specific Targets

### `install`

Default summary:

- binary action: copied, verified, unchanged, or skipped self-copy
- skill files written/unchanged per runtime
- manifest path
- backup snapshot path when created
- next step when PATH guidance is relevant

Verbose detail:

- current executable path
- managed binary path and hash comparison outcome
- partial marker recovery decision
- collision classification per file
- render target per skill/runtime

Dry run:

- no managed binary write
- no skill writes
- no backup writes
- no manifest writes
- no partial marker writes

### `uninstall`

Default summary:

- number of files removed
- manifest removed or absent
- managed binary left in place unless `--include-binary`

Verbose detail:

- manifest path
- each file considered
- drift checks

Dry run:

- list files that would be removed
- report drift blockers before mutation

### `init`

Default summary:

- project path
- blueprint
- resolved packs
- whether registration succeeded or was skipped
- initial source bundle path when sources were copied

Verbose detail:

- folder/file count
- manifest path
- copied source paths

### `index` / `index-all`

Default summary:

- project id
- indexed file count
- freshness before/after when available
- store path

Verbose detail:

- lock path
- temp store path
- promotion steps
- metadata path

## Non-Goals

- Remote telemetry.
- Persistent log files.
- A daemon or background service.
- Making agent-owned operations (`wiki-ingest`, `wiki-query`, `wiki-lint`) part
  of the binary.
- Replacing `doctor`; improved command logging should complement `doctor`, not
  turn every command into a full diagnostic report.

## Risks

1. **Noisy defaults.** If default output becomes too chatty, scripts and humans
   both suffer. Keep default summaries short and reserve per-file detail for
   `--verbose`.
2. **Dry-run drift.** If dry-run has separate decision logic, it will lie. The
   implementation must share classification and planning with real execution.
3. **JSON contract lock-in.** Structured output becomes compatibility surface.
   Add JSON only for report types that are likely to stay stable.
4. **Progress output in tests.** Integration tests should use `--quiet` or
   assert exact summaries where summaries are the contract.

## What Closes This Proposal

Promotion to a plan covering:

1. root CLI flags and output context
2. `tracing` initialization policy
3. `InstallReport` and `install --dry-run`
4. `UninstallReport` and `uninstall --dry-run`
5. default summaries for `install`, `uninstall`, `init`, `index`, and
   `index-all`
6. tests for quiet, verbose, dry-run non-mutation, and default summaries
7. documentation updates in README and affected specs once behavior lands

## Open Questions

1. Should `--verbose` imply `RUST_LOG=debug` style tracing, or should explicit
   `RUST_LOG` continue to be the only way to enable dependency-level logs?
   Lean: `--verbose` should enable framework diagnostics only.
2. Should `install --dry-run` return success when a real install would be
   blocked by a collision, or fail after printing the blocker? Lean: fail with
   the same exit code semantics as real install, but without mutation.
3. Should default install summaries go to `stdout` or `stderr`? Lean:
   summaries to `stdout`, diagnostics to `stderr`, errors to `stderr`.
