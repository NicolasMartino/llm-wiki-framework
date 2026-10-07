# Always-On Crash Reports

- Document Class: Proposal
- Status: Proposed
- Date: 2026-05-11
- Category: CLI UX, operational diagnostics
- Scope: Capture a crash report for every panic and every post-parse
  error-exit of the `llm-wiki` binary, written to a managed-home directory,
  with no opt-in flag required. Panic reports carry a full origin backtrace;
  error-return reports carry the `anyhow` error chain. CLI parse errors are
  owned by clap and stay out of scope for this iteration.
- Sources: conversational input 2026-05-11 (`-v/--verbose` review follow-up);
  no raw source file captured yet
- Implementation References: src/main.rs, src/cli.rs, src/paths.rs
- Related: wiki/proposals/cli-observability.proposal.md,
  wiki/plans/cli-observability.plan.md,
  wiki/decisions/binary-path-bootstrap.decision.md

## Question

When `llm-wiki` panics or exits with an error, should the binary always write a
self-contained crash report (error chain plus backtrace) to disk so the
operator has a Java-style stacktrace to read or share, without having to re-run
with a debug flag?

## Proposal

Yes. Capture a crash report on every post-parse nonzero termination - both
`panic!` and `Result::Err` returned from the dispatch path - and write it to
`~/.llm_wiki/crashes/<UTC-timestamp>-<pid>-<tid>.log`. On the panic path the
hook calls `std::backtrace::Backtrace::force_capture()` while the panicking
frame is still live, so the report carries a full origin-aware stacktrace
with no `RUST_BACKTRACE=1` required. On the error-return path the report
carries the `anyhow` error chain; origin-frame backtraces for returned
errors are deferred (see Future Improvements). The user-facing stderr
message stays compact; the report path is printed once at the end so it
can be copied or shared.

CLI parse errors are handled by clap (`Cli::parse()` exits internally with
its own actionable message) and are intentionally not converted into crash
reports in this iteration.

This proposal is scoped to *file-based crash reports*. It does not change error
wording on stderr, exit codes, the `-v/--verbose` diagnostic surface, or
telemetry. Reports stay local to the machine.

## Output Model

Three failure surfaces, in order of who reads them:

1. **stderr** keeps the existing compact `anyhow` error chain (printed by
   the binary's own `Error: ...` line under the new `ExitCode` shape; see
   Error Path) and the default panic message (preserved by chaining the
   previous panic hook). No wording changes versus today.
2. **The crash report file** carries the full context. On the panic path it
   includes an origin-aware stacktrace. On the error-return path it
   includes the `anyhow` error chain and a placeholder backtrace section
   (see Crash Report Contents). Operators open the file when the stderr
   message is not enough.
3. **One trailing stderr line** points at the report:
   `crash report: ~/.llm_wiki/crashes/2026-05-11T13-42-08.123456789Z-48211-3.log`.

A panic and an error-return produce the same report *shape*; only the
contents of the `--- backtrace ---` section differ, so the operator does
not need to know which path failed to read the file.

## Crash Report Location

Reports live under managed home:

```text
~/.llm_wiki/crashes/<UTC ISO8601 timestamp with nanoseconds>-<pid>-<tid>.log
```

Both `pid` and the panicking thread id are included so two threads crashing
in the same nanosecond cannot collide on the same filename. If the
nanosecond + pid + tid triple ever proves insufficient under real load, the
fallback is `OpenOptions::create_new(true)` with a numeric suffix (see
Future Improvements).

`~/.llm_wiki/` is already the managed-home root from
`wiki/decisions/binary-path-bootstrap.decision.md` and is resolved by
`Paths::managed_home()`. Putting crashes there means:

- every command writes to the same place regardless of whether a project was
  resolved (`install`, `path`, `doctor`, and command failures before project
  resolution all land in the same directory)
- the directory survives upgrades and is not tied to a CWD
- one retention policy covers the whole binary

Project-local `<project>/.llm_wiki/crashes/` is **not** part of this proposal.
It is a plausible follow-up once we decide which commands always have a
resolved project in context.

If `Paths::from_env()` itself fails (no `HOME`), the binary falls back to
`std::env::temp_dir().join("llm-wiki-crashes")` so a crash is still recorded.

## Crash Report Contents

Plain-text, deterministic, easy to paste into an issue. Example for the
panic path:

```text
llm-wiki crash report
=====================
timestamp:   2026-05-11T13:42:08.123456789Z
binary:      llm-wiki 0.1.0
git_sha:     <build-time SHA if available, else "unknown">
os:          darwin aarch64
pid:         48211
tid:         3
command:     llm-wiki -v search "battery tech" --project electric-car
cwd:         /Users/nicolasmartino/Documents/car/electric
exit_kind:   panic

--- panic message ---
internal invariant violated: project id resolved but root missing

--- backtrace ---
   0: llm_wiki::search::commands::search
             at src/search/commands.rs:354
   1: llm_wiki::main
             at src/main.rs:38
   ...

--- environment (filtered) ---
HOME=<redacted>
PATH=<redacted>
RUST_LOG=<unset>
NO_COLOR=<unset>
CLICOLOR_FORCE=<unset>
LLM_WIKI_TEST_INDEX_SLEEP_MS=200          # safe-key, value shown
LLM_WIKI_OPENAI_TOKEN=<redacted>          # not in safe list
```

The error-return variant differs only in the middle:

```text
exit_kind:   error

--- error chain ---
search index missing for project electric-car
caused by: open qmd-rs store: file not found

--- backtrace ---
<not captured; set RUST_BACKTRACE=1 or see Future Improvements>
```

Notes:

- **Panic backtrace is always captured** via
  `std::backtrace::Backtrace::force_capture()` inside the panic hook, not
  through `RUST_BACKTRACE`. This avoids `env::set_var` (unsafe in edition
  2024) and works for users who do not know the env var exists.
- **Error-return backtrace.** `anyhow::Error::backtrace()` is reused when
  present, but it is only populated when `RUST_BACKTRACE` or
  `RUST_LIB_BACKTRACE` is set. By default it is `Disabled` and the
  `--- backtrace ---` section for an error-return report will say
  `<not captured; set RUST_BACKTRACE=1 or see Future Improvements>`. A
  top-level `force_capture()` after `?` propagates would not include the
  originating frame and is intentionally not used as a fallback.
- **Environment is filtered through a two-layer allowlist.**
  - **Known-safe keys are listed verbatim:** `RUST_LOG`, `NO_COLOR`,
    `CLICOLOR_FORCE`, and the `LLM_WIKI_TEST_*` test hooks.
  - **Other `LLM_WIKI_*` keys are listed with value redacted**, so future
    env additions (potential tokens or auth) do not leak by default.
  - **Any key matching `(?i)token|secret|key|password|auth|credential` is
    always redacted**, even if it would otherwise fall under the safe
    list.
  - `HOME` and `PATH` are listed as `<redacted>` markers; the full process
    environment is never dumped.
- **Command line** comes from `std::env::args_os()`, verbatim.
- **No network, no upload.** Reports stay on disk.

## Panic Path

Install a panic hook in `main` before any work runs, chaining the previous
hook so default panic wording on stderr is preserved:

```rust
let prev = std::panic::take_hook();
std::panic::set_hook(Box::new(move |info| {
    let report = CrashReport::from_panic(info);
    let _ = report.write();        // best-effort
    prev(info);                    // keep default panic wording on stderr
    eprintln!("crash report: {}", report.path.display());
}));
```

The hook is best-effort and must not panic (use `let _ = ...`, never
`expect`). Default panic propagation continues, so the process still exits
with a panic status. The hook fires for panics in spawned threads too;
filenames include the panicking thread id so simultaneous thread panics do
not collide.

## Error Path

Split `main` into a tiny shell that owns process termination and a `run`
helper that owns dispatch:

```rust
fn main() -> ExitCode {
    crash::install_panic_hook();
    let cli = Cli::parse();             // clap exits internally on parse errors
    match run(&cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            let report = CrashReport::from_error(&cli, &error);
            let _ = report.write();
            eprintln!("Error: {error:#}");
            eprintln!("crash report: {}", report.path.display());
            ExitCode::FAILURE
        }
    }
}

fn run(cli: &Cli) -> Result<()> { /* existing dispatch from current main */ }
```

The `ExitCode` shape is required because `fn main() -> Result<()>` would
print the error itself via `Termination::report()`, leaving no place for
the `crash report:` pointer line. With this split, the binary prints its
own compact `Error: ...` (preserving today's wording) and then names the
report file.

`CrashReport::from_error` includes `anyhow::Error::backtrace()` if it is
populated and otherwise marks the backtrace section as not captured. It
does *not* call `Backtrace::force_capture()` at the top level, because the
stack has already unwound through `?` and the result would not include the
originating `bail!` frame; a misleading post-unwind trace is worse than
none.

## Always-On Cost

Capturing a backtrace is moderately expensive (symbol lookup, syscalls).
Mitigations:

1. **Capture happens at most once per process**, inside the panic hook.
   Error-return reports do not call `force_capture()` at all.
2. **No `RUST_BACKTRACE` plumbing on the panic path.** Always-on means
   panic reports are useful even for users who never set the variable.
3. The cost is paid only on the failure path.

## Retention

- After each crash, prune the directory to at most **20** files, oldest first
  by mtime.
- No automatic upload, no automatic deletion before write.
- `llm-wiki doctor` listing recent reports is a deferred follow-up.

## Distinguishing User Errors From Bugs

The original ask was "all the time", and this proposal honors that: every
nonzero exit writes a report. Practical worry: many `llm-wiki` failures are
expected (running `search` without indexing). Mitigations:

1. **Retention cap (20)** keeps the directory bounded.
2. **No noise on stdout.** Scripts that check `exit_status.success()` see no
   behavior change. On the error path the binary already prints
   `Error: ...` today (via `Termination::report`); under the new
   `ExitCode` shape the same line is printed by hand, plus exactly one
   trailing `crash report: ...` pointer. Stdout is untouched.

Finer-grained "this was a user error, do not write a report" is deferred; the
first cut treats every nonzero exit equally so operators do not have to wonder
why a report is missing.

## Crate Choices

Recommended: **zero new crates.** Use
`std::backtrace::Backtrace::force_capture()`, `std::panic::set_hook`, and
`anyhow`'s built-in backtrace.

Considered and rejected:

- **`human-panic`** - opinionated, writes to temp dir with TOML; conflicts
  with the managed-home/plain-text choice. Useful as a reference, not as a
  dep.
- **`color-eyre`** - would require migrating the whole binary from `anyhow`
  to `eyre` just to format backtraces. Out of scope; we only need a textual
  file.

## TTY And Color

The crash report file is always plain text - no ANSI escapes regardless of
terminal state. The trailing stderr pointer follows existing color rules from
`cli-observability.proposal.md`.

## Exit Codes

Unchanged. Panics keep their panic status; error returns keep their existing
nonzero exit; successful runs write nothing.

## Interaction With `-v/--verbose`

Independent surfaces:

- `-v/--verbose` controls **success-path diagnostics on stderr** during a run.
- Crash reports control **failure-path artifacts on disk** after a run.

The report does not embed live verbose diagnostics in the first cut (would
require buffering stderr). That is listed as an open question.

## Non-Goals

- Remote telemetry or automatic upload.
- Project-local `.llm_wiki/crashes/` in this iteration.
- A rich numeric exit-code taxonomy.
- A separate `--debug` or `--explain` flag.
- Persistent log files for successful runs.
- Replacing user-facing error wording.
- Reporting on clap-owned CLI parse errors. clap prints its own actionable
  message and exits; switching to `try_parse()` is listed as a future
  improvement.
- Origin-frame backtraces on the error-return path. The first cut carries
  the `anyhow` error chain; surfacing the originating `bail!` frame is a
  future improvement.

## Risks

1. **Directory clutter.** Friendly errors generate reports too. The 20-file
   cap limits churn; operators can `rm -rf ~/.llm_wiki/crashes/` at any time.
2. **Backtrace cost on hot error paths.** Errors are not hot; the panic
   path captures at most once per process, and the error-return path
   does not call `force_capture()` at all in the first cut.
3. **Privacy of arguments and paths.** The report contains user-typed args and
   CWD. Both are already on the user's machine and never uploaded. Env is
   allowlisted.
4. **Panic-during-panic.** Hook writes best-effort; must not panic itself.
5. **Edition 2024 `env::set_var`.** Do not flip `RUST_BACKTRACE` from inside
   the program; use `Backtrace::force_capture()` directly.
6. **Report wording as accidental API.** Reports should be treated as debug
   output, not stable contract; tests should assert essential facts, not full
   wording.

## Implementation Shape

Sketch:

```rust
// src/crash.rs
pub struct CrashReport {
    pub path: PathBuf,
    body: String,
}

impl CrashReport {
    pub fn from_panic(info: &PanicHookInfo<'_>) -> Self { ... }
    pub fn from_error(cli: &Cli, error: &anyhow::Error) -> Self { ... }
    pub fn write(&self) -> std::io::Result<()> { ... }
}

pub fn install_panic_hook() { ... }
fn crash_dir() -> PathBuf { ... }
fn prune_oldest(dir: &Path, keep: usize) { ... }
fn redact_env(name: &str, value: &str) -> String { ... }
```

`main.rs` switches to `fn main() -> ExitCode`, calls
`crash::install_panic_hook()` first, then runs `Cli::parse()` (clap owns
its own parse-error path) and dispatches through `run(&cli)`; an `Err`
produces a report and the binary prints its own `Error: {error:#}` line
followed by the report pointer.

Tests should cover:

1. A forced panic inside a command writes a file in
   `~/.llm_wiki/crashes/`, the report body contains the panic message and
   a backtrace whose frames include the originating module path (e.g.
   `llm_wiki::search`), and the process exits with a panic status.
2. A failing command (e.g. `search` on a missing index) writes a report
   whose body contains the full `anyhow` error chain. The first cut does
   **not** assert a specific originating frame on the error path; that
   assertion is gated on the origin-backtrace future improvement.
3. The trailing stderr line names the report path on both panic and error
   exit, and the default panic message is still present on stderr (the
   panic hook chains the previous hook).
4. Retention prunes to 20 files when 21 reports exist.
5. Crash dir falls back to `std::env::temp_dir().join("llm-wiki-crashes")`
   when `HOME` is unset.
6. Successful runs leave the crashes directory untouched.
7. Env redaction: a synthetic `LLM_WIKI_FAKE_TOKEN=abc` environment
   variable shows up as `<redacted>` in the report; a safe-list key like
   `RUST_LOG=info` shows its value.

## What Would Close This Proposal

Promotion to a `wiki/plans/crash-reports.plan.md` whose acceptance criteria
are:

1. Panic hook installed before any command runs, chaining the previous
   hook so default panic stderr wording is preserved.
2. `fn main() -> ExitCode` + `run(&cli) -> Result<()>` split, with an
   error-return wrapper that produces a crash report and a single stderr
   pointer.
3. Reports written under `~/.llm_wiki/crashes/` with the documented
   filename pattern (nanosecond timestamp + pid + tid) and body format.
4. Panic-path backtraces captured unconditionally via
   `Backtrace::force_capture()` inside the hook, without `RUST_BACKTRACE`
   or any flag. Error-return reports carry the `anyhow` error chain;
   origin-frame backtrace for returned errors is a future improvement.
5. Env section is redacted by the documented allowlist + redaction rules.
6. Retention prunes to a fixed cap.
7. Integration tests cover panic, error, retention, fallback-to-temp, env
   redaction, and chained panic stderr wording.
8. README documents the crash-report location and retention.

## Future Improvements

The first cut deliberately keeps each surface as small as defensibly
useful. The following items are deferred and can be reopened once the
basic crash-report path is in place:

1. **Origin-aware backtraces for returned errors.** Either a crate-local
   `Error` / `Result` wrapper, or a `try_or_report!` macro at command
   dispatch arms in `run`, that calls `Backtrace::force_capture()` before
   `?` propagates the error. Until this lands, error-return reports carry
   the `anyhow` error chain but not the originating `bail!` frame.
2. **CLI parse-error reports.** Switch `Cli::parse()` to
   `Cli::try_parse()` and write a report for `ErrorKind` values other
   than `DisplayHelp` / `DisplayVersion`. Skipped now because clap
   already prints actionable parse errors and `parse()` keeps `main`
   small.
3. **Crash-file collision retry.** If the nanosecond + pid + tid filename
   ever collides under real load, fall back to
   `OpenOptions::new().create_new(true).open(...)` with a numeric
   suffix.
4. **Embedding the verbose diagnostic tail in the report when `-v` is
   active.** Requires buffering stderr; see Open Question 1.
5. **`llm-wiki doctor` listing recent crash reports.** See Open Question
   2.
6. **Env-configurable retention cap** (`LLM_WIKI_CRASH_KEEP`). See Open
   Question 3.

## Open Questions

1. Should the crash report include the *last N lines* of the in-process
   verbose diagnostic buffer when `-v` is active? It would make verbose runs
   self-debugging but requires buffering stderr.
2. Should `llm-wiki doctor` learn to list and `cat` recent crash reports? Adds
   discoverability but expands `doctor`'s scope.
3. Should the retention cap (20) be configurable via env (e.g.
   `LLM_WIKI_CRASH_KEEP`)? Probably not in the first cut.
