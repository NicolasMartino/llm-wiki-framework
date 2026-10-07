# Observability Contract

- Document Class: Checklist
- Status: Active
- Date: 2026-05-11
- Category: CLI UX, operational diagnostics, review gate
- Scope: Review checklist for future `llm-wiki` binary changes that add or alter user-visible command behavior.
- Sources: wiki/specs/documentation-model.spec.md, wiki/plans/cli-observability.plan.md, wiki/proposals/cli-observability.proposal.md
- Related: src/cli.rs, src/main.rs, tests/search_commands.rs

## When To Apply

Use this checklist for any implementation plan, code review, or merge review
that adds or changes:

- a `llm-wiki` command or command argument
- registry, install, runtime-home, path, status, doctor, index, search, or
  project-selection behavior
- command safety decisions, skipped work, fallback paths, recovery guidance, or
  user-facing error handling
- structured stdout or JSON output contracts

## Checklist

- The change states what `-v` / `--verbose` should explain for the affected
  command path.
- Diagnostics are emitted through `CliContext` and the CLI tracing subscriber,
  not ad-hoc verbose `eprintln!` calls.
- Diagnostic facts come from command-owned decisions and already-resolved state.
- The implementation does not re-resolve paths or duplicate command logic only
  to format diagnostics.
- Normal stdout is unchanged unless the plan explicitly calls out a user-facing
  output change.
- JSON stdout remains parseable and contains no diagnostic text or ANSI escape
  sequences.
- Install's long-running byte-oriented steps (hashing, downloading and
  verifying search models) report normal progress on stderr even without
  `--verbose`: attended terminals may redraw a single line, while non-TTY
  stderr uses bounded append-only milestones with no ANSI or carriage returns.
  Other commands are not held to this yet: `doctor` hashes every model artifact
  and `index` builds embeddings, both silently.
- Progress totals come from command-owned validated metadata, and progress
  reporting does not change stdout, JSON, interactivity, success, failure, or
  exit-code behavior.
- `--verbose` does not change success, failure, or exit-code behavior.
- `--help` and `--version` stay exempt from diagnostics.
- Dependency logs stay disabled unless `RUST_LOG` is set.
- CLI tests that assert stderr clear inherited `RUST_LOG`.
- A verbose assertion lands beside the existing command tests for each changed
  command path.
- Tests assert essential facts rather than full diagnostic wording.

## Pass Criteria

- The implementation can answer "what will a user learn with `-v` that they
  cannot learn from normal output?"
- The answer is backed by at least one automated assertion for the affected
  command path.
- Review finds no path where diagnostics can contaminate stdout, JSON output, or
  success/failure semantics.
