# Plan: Install Model Download Progress

- Document Class: Plan
- Status: Completed (develop)
- Date: 2026-08-14
- Category: Install UX, model materialization, operational diagnostics
- Scope: Replace the three silent long-running stalls in enabled-LLM-search
  install (pre-download artifact hashing, model download streaming, and
  post-download hash verification) with bounded stderr progress reporting that
  carries transferred bytes, rate, percentage, and estimated time remaining,
  degrades to periodic line output on non-TTY stderr, and leaves stdout and
  JSON output contracts byte-identical.
- Sources: src/search_models.rs, src/install.rs, src/cli.rs, wiki/checklists/observability-contract.checklist.md, wiki/plans/noninteractive-llm-search-install.plan.md, wiki/plans/idempotent-search-model-install.plan.md
- Related: wiki/plans/cli-observability.plan.md, wiki/proposals/cli-observability.proposal.md, wiki/plans/semantic-hybrid-search.plan.md, wiki/references/llm-search-model-licensing.reference.md
- Promotion Target: wiki/checklists/observability-contract.checklist.md (add a
  long-running-operation clause once the progress contract is validated)

## Problem

An enabled-LLM-search install transfers and hashes roughly 1.5 GiB while
printing nothing. The install is not hung, but it is indistinguishable from
hung. Observed during a real 2026-08-14 install attempt on Ubuntu 26.04.

Balanced profile payload, from the catalog constants in
[src/search_models.rs:178-222](../../src/search_models.rs#L178-L222):

| Model | Role | `expected_size_bytes` | Approx |
| --- | --- | --- | --- |
| `embeddinggemma-300m-q8_0` | embedding | 333,590,944 | 318 MiB |
| `qmd-query-expansion-1.7b-q4_k_m` | query-expansion | 1,282,438,912 | 1.19 GiB |
| **Total (balanced)** | | **1,616,029,856** | **1.51 GiB** |

`qwen3-reranker-0.6b-q8_0` (639,000,000 B) is in `MODEL_CATALOG` but
`BALANCED_PROFILE.reranker_model` is `None`, so it is not part of this payload.

There are three distinct silent stalls, not one:

1. **Pre-download hashing.** `classify_model_artifact`
   ([src/search_models.rs:514-535](../../src/search_models.rs#L514-L535)) calls
   `sha256_file` on every artifact that already exists on disk. On a re-run or
   `--force` install this reads up to 1.51 GiB from disk before any network
   activity, silently. This stall hits even when the outcome is `Reused` and
   nothing is downloaded at all.
2. **Download streaming.** `download_model`
   ([src/search_models.rs:617-660](../../src/search_models.rs#L617-L660)) loops over
   64 KiB chunks writing to a temp file with no output of any kind. The
   `reqwest` client timeout is `Duration::from_secs(3600)`
   ([src/search_models.rs:628](../../src/search_models.rs#L628)), so a stalled
   connection can sit for a full hour before erroring.
3. **Post-download verification.** `download_and_verify_model`
   ([src/search_models.rs:565-585](../../src/search_models.rs#L565-L585)) calls
   `sha256_file` again on the file it just wrote — a second full-size pass,
   immediately after the download appears to finish.

The only existing observability is two `context.diagnostic` calls that bracket
the whole operation ([src/install.rs:634-648](../../src/install.rs#L634-L648)). They
are `-v`-only and emit *before* and *after*, never during, so even a verbose
install shows nothing for the entire stall.

## Deliverable

**Promise:** a user running an enabled-LLM-search install sees continuous
evidence of forward progress, with a defensible time estimate, and can tell a
slow network from a hung process without adding `-v` or opening another
terminal.

Target rendering on a TTY (single line, rewritten in place, on stderr):

```text
[1/2] embeddinggemma-300m-q8_0   download   61%  195.2 MiB / 318.1 MiB   12.4 MiB/s   eta 00:09
[1/2] embeddinggemma-300m-q8_0   verify     88%  280.1 MiB / 318.1 MiB   1.1 GiB/s    eta 00:00
```

Target rendering on non-TTY stderr (append-only, bounded line count):

```text
[1/2] embeddinggemma-300m-q8_0 download start 318.1 MiB
[1/2] embeddinggemma-300m-q8_0 download 25% 79.5 MiB / 318.1 MiB 12.4 MiB/s eta 00:19
[1/2] embeddinggemma-300m-q8_0 download 50% 159.0 MiB / 318.1 MiB 12.6 MiB/s eta 00:12
[1/2] embeddinggemma-300m-q8_0 download 75% 238.6 MiB / 318.1 MiB 12.5 MiB/s eta 00:06
[1/2] embeddinggemma-300m-q8_0 download done 318.1 MiB in 00:25 (12.5 MiB/s)
```

## In Scope

- `download_model` streaming loop in `src/search_models.rs`
- `sha256_file` in `src/search_models.rs`, both call sites (classify and verify)
- `materialize_model` / `materialize_model_with_downloader` signatures, as
  needed to thread a progress sink without breaking the existing test seam
- the install call site in `src/install.rs`
- one new progress module, `src/progress.rs`
- tests in `tests/install.rs` plus unit tests beside the new module

## Out Of Scope

- Resumable / range-request downloads. Worth doing, but it is a separate
  deliverable with its own failure modes (partial-file trust, `ETag` handling)
  and must not be smuggled in behind a UX change.
- Parallel model downloads. Sequential ordering is what makes `[n/total]`
  honest, and concurrency would reshape the whole reporting model.
- Reducing the 3600 s client timeout, or adding a stall/idle timeout. Recorded
  as a follow-up below — it is a behavior change, not a reporting change.
- Progress for `index` / `index-all`. Same underlying need, different command
  path; do not widen this deliverable.
- Any change to stdout, JSON output, or exit codes.

## Depends On

Nothing. The catalog already carries `expected_size_bytes` for every model, so
totals are known before the first byte moves and no new metadata is required.

## Non-Negotiable Constraints

1. **Progress writes to stderr only.** Never stdout. This preserves the JSON
   stdout contract in the observability checklist without needing a special
   case per command.
2. **No ANSI escapes when stderr is not a TTY.** Carriage-return rewriting and
   any styling are conditional on an explicit `IsTerminal` check against
   stderr, not inferred from stdout and not left to a library default.
3. **Bounded output when not a TTY.** At most one line per 25% step plus a
   start and a done line per model per phase. A 1.51 GiB install must not
   produce megabytes of CI log.
4. **`expected_size_bytes` is the authority for the total**, not the HTTP
   `Content-Length` header. The catalog value is already covered by the hash
   check. If `Content-Length` disagrees with the catalog, report progress
   against the catalog value and record the disagreement as a `-v` diagnostic —
   it is a real signal that an upstream artifact moved.
5. **The existing test seam survives.** `materialize_model_with_downloader`
   currently accepts an injected `downloader` and is exercised by unit tests at
   [src/search_models.rs:789-829](../../src/search_models.rs#L789-L829). Those tests
   must keep passing with a no-op progress sink and must not gain timing
   dependence.
6. **`--verbose` does not change success, failure, or exit-code behavior**, and
   progress rendering is independent of `-v`. Progress is normal operation
   feedback; `-v` remains the diagnostics channel. They must not duplicate each
   other line for line.
7. **No new required interactivity.** The `--non-interactive` scripted install
   path must work unchanged, emitting the bounded non-TTY form.

## Design

### Progress sink

Add `src/progress.rs` exposing a small reporter facade over two modes: an
`indicatif` terminal renderer and a bounded line renderer. Select the mode once
at the install call site using `IsTerminal` on stderr. Tests can select a
no-output mode without adding timing dependence to model-materialization tests.

```text
ProgressReporter::begin(phase, label, index, total_models, total_bytes)
ProgressOperation::advance(bytes_delta)
ProgressOperation::finish()
```

`advance` is called from inside the existing 64 KiB loops, so the change to
`download_model` and `sha256_file` is one line each in the loop body.

### Rate and ETA

Delegate TTY rate and ETA calculation to `indicatif`; do not maintain a second
custom smoothing implementation. The bounded non-TTY reporter uses aggregate
rate only at its four milestone lines, where stability matters more than
frame-to-frame responsiveness.

ETA is suppressed and rendered as `eta --:--` until at least 2 seconds and 1%
have elapsed, so the first frame does not show an absurd number.

### Redraw throttling

Redraw at most every 100 ms on a TTY. The download loop runs at 64 KiB per
iteration; on a fast link that is thousands of iterations per second and
redrawing per chunk would itself become a measurable cost.

### Dependency choice

Use `indicatif` for the terminal renderer, pinned in `[workspace.dependencies]`
alongside the existing entries.

Rationale: terminal width handling, ANSI correctness, and redraw throttling are
exactly the details that go subtly wrong when hand-rolled, and the repo already
carries an interactive dependency (`inquire`) so this is not a new class of
dependency.

Caveat that must be handled explicitly, not assumed: `indicatif` defaults to a
hidden draw target when it does not detect a terminal, which would produce
**no** output in CI — the opposite of constraint 3. The plan therefore requires
the non-TTY path to be our own line renderer, chosen by our own `IsTerminal`
check, with `indicatif` used only on the confirmed-TTY branch and pointed at
`ProgressDrawTarget::stderr()`.

## Steps

1. **Add `src/progress.rs`** with the reporter facade, `indicatif` terminal
   mode, bounded non-TTY mode, test-only no-output mode, formatting helpers,
   and unit tests. Gate: `cargo test` green, strict clippy clean.
2. **Thread progress through `search_models.rs`.** Keep the injected downloader
   test seam, share one network streaming implementation, and use small
   content-length and byte-progress observers so normal and progress-aware
   paths do not duplicate download logic.
3. **Add the two real renderers** and perform `IsTerminal` selection before
   enabled-search preflight in `src/install.rs`,
   including the `[n/total]` counter derived from `install_plan.actions`.
   Gate: manual run of both TTY and piped-stderr forms.
4. **Cover the reuse path.** Ensure the pre-download `classify_model_artifact`
   hashing reports under a `verify` phase label, so a no-download re-install
   also shows progress. This is the stall most likely to be missed, because it
   does not look like a download.
5. **Record the `Content-Length` disagreement diagnostic** as a
   `context.diagnostic` line per constraint 4.
6. **Tests** per the gates below.
7. **Docs.** Add a short "what you will see during install" note to
   [README.md](../../README.md) next to the enabled-search command, showing both
   renderings.

## Implementation Status

Implemented on the branch `impl/install-download-progress` (2026-10-06) and
landed on `develop` through PR #66, with the real-install evidence below. The
TTY renderer uses `indicatif`, with the ETA floor below applied through a custom
template key; only the bounded non-TTY milestone policy is otherwise
project-owned. Progress selection is created before enabled-search preflight,
so reuse hashing is visible. Download and replacement actions consume the
preflight decision directly rather than classifying and hashing the same
existing artifact again.

The injected-downloader seam changed shape, against constraint 5's letter but
not its purpose: `materialize_model_with_downloader` had become a test-only
copy of decisions `plan_enabled_search_install` now makes (and its own tests
cover), so it is gone, with the reuse and hash-mismatch tests that drove it.
Production's `download_and_verify_model` takes the downloader instead, so the
wrong-bytes test, a replace test and a retry test now drive the same path
install runs, with a recorded line output and no timing dependence. A
debug-build hook, `LLM_WIKI_TEST_MODEL_SOURCE`, streams a local file in place
of the network, so an install test covers the download and verify progress and
the verbose gate (the `search model materialization` diagnostic once, the
`Content-Length` disagreement once). The success-path `materialization
outcome` diagnostic needs real model bytes, so only a real install reaches it.

## Outcome

A real enabled-search install, balanced profile, on this Linux host
(2026-10-07), under a temporary `HOME` deleted afterwards. Exit 0 every run;
stdout carried no progress line, and stderr no ANSI escape or carriage return
in the piped form.

Phase durations (piped run, fresh `HOME`, so no pre-download hashing):

| Model | Download | Verify |
| --- | --- | --- |
| `embeddinggemma-300m-q8_0` (318.1 MiB) | 01:31 (3.5 MiB/s) | under 1 s (1.1 GiB/s) |
| `qmd-query-expansion-1.7b-q4_k_m` (1.2 GiB) | 05:37 (3.6 MiB/s) | under 1 s (1.9 GiB/s) |

Pre-download hashing, from a re-install with both models present: under 1 s
per model (1.6 and 1.9 GiB/s), reported as `verify`, with nothing downloaded.
Peak rate seen at a milestone: 3.7 MiB/s.

ETA against actual over the final 50 % of each download:

| Model | At | ETA | Actual remaining | Error |
| --- | --- | --- | --- | --- |
| embedding | 50 % | 43 s | 48.5 s | -11 % |
| embedding | 75 % | 22 s | 26.7 s | -18 % |
| query expansion | 50 % | 166 s | 172.2 s | -4 % |
| query expansion | 75 % | 85 s | 84.7 s | 0 % |

All within ±20 %, so no eval page.

Piped stderr, first model (the second has the same shape):

```text
[1/2] embeddinggemma-300m-q8_0 download start 318.1 MiB
[1/2] embeddinggemma-300m-q8_0 download 25% 79.5 MiB / 318.1 MiB 3.7 MiB/s eta 01:05
[1/2] embeddinggemma-300m-q8_0 download 50% 159.1 MiB / 318.1 MiB 3.7 MiB/s eta 00:43
[1/2] embeddinggemma-300m-q8_0 download 75% 238.6 MiB / 318.1 MiB 3.7 MiB/s eta 00:22
[1/2] embeddinggemma-300m-q8_0 download 100% 318.1 MiB / 318.1 MiB 3.5 MiB/s eta 00:00
[1/2] embeddinggemma-300m-q8_0 download done 318.1 MiB in 01:31 (3.5 MiB/s)
[1/2] embeddinggemma-300m-q8_0 verify start 318.1 MiB
[1/2] embeddinggemma-300m-q8_0 verify 25% 79.6 MiB / 318.1 MiB 1.0 GiB/s eta --:--
[1/2] embeddinggemma-300m-q8_0 verify 50% 159.1 MiB / 318.1 MiB 1.0 GiB/s eta --:--
[1/2] embeddinggemma-300m-q8_0 verify 75% 238.6 MiB / 318.1 MiB 1.1 GiB/s eta --:--
[1/2] embeddinggemma-300m-q8_0 verify 100% 318.1 MiB / 318.1 MiB 1.1 GiB/s eta --:--
[1/2] embeddinggemma-300m-q8_0 verify done 318.1 MiB in 00:00 (1.1 GiB/s)
```

Terminal form, captured on a pseudo-terminal 140 columns wide: one line per
step, redrawn in place; frames of a reuse hash, then a download and its verify:

```text
[2/2] qmd-query-expansion-1.7b-q4_k_m verify     0%  64.00 KiB / 1.19 GiB  811.91 MiB/s  eta --:--
[2/2] qmd-query-expansion-1.7b-q4_k_m verify    48%  586.94 MiB / 1.19 GiB  1.97 GiB/s  eta --:--
[2/2] qmd-query-expansion-1.7b-q4_k_m verify   100%  1.19 GiB / 1.19 GiB  1.83 GiB/s  eta --:--
[1/2] embeddinggemma-300m-q8_0 download 100%  318.14 MiB / 318.14 MiB  1.88 MiB/s  eta 00:00
[1/2] embeddinggemma-300m-q8_0 verify   100%  318.14 MiB / 318.14 MiB  1.92 GiB/s  eta --:--
```

The first terminal run showed a first frame without its step name and an ETA
of `04:37:25` in the first second; both were fixed in this PR (the step is set
before the bar draws, and the ETA shows `--:--` below 2 s or 1 %), and the
rerun above is after the fix.

## Verification Gates

- **Unit:** project-owned non-TTY rate/ETA/format helpers are tested with
  injected elapsed values — byte formatting boundaries (999 B, 1 KiB,
  1023 MiB) and ETA suppression below the 2 s / 1% floor. TTY rate/ETA behavior
  is delegated to `indicatif`.
- **Non-TTY bound:** a line-renderer test asserts the emitted progress line
  count per model per phase is `<= 6`, and that no line contains
  `\x1b[` or `\r`.
- **Stdout purity:** existing JSON-output assertions in `tests/search_commands.rs`
  and `tests/install.rs` still pass; progress code has no stdout write path.
- **Seam preservation:** `materialize_reuses_verified_model_without_downloader`,
  `materialize_rejects_downloader_that_writes_wrong_bytes`, and the
  hash-mismatch test continue to pass through the injected downloader seam.
- **Verbose interaction:** a `-v` install asserts the existing
  `search model materialization` diagnostics still appear and are not duplicated
  by progress output, per the observability checklist.
- **Repo gates:** `just verify` (fmt, test, clippy-strict, snapshots,
  audit-legacy) green.

## Evidence To Record

A real enabled-search install on this Linux host, capturing:

1. wall-clock duration of each of the three phases per model
2. the captured stderr for both TTY and piped forms
3. peak observed rate and whether the ETA stayed within ±20% of actual for the
   final 50% of each transfer

Record as an eval page under `wiki/evals/` only if the ETA accuracy check
fails and needs iteration; otherwise the numbers belong in this plan's outcome
section.

## Observability Contract Compliance

Checked against
[wiki/checklists/observability-contract.checklist.md](../checklists/observability-contract.checklist.md):

- **User-facing output change is explicitly called out.** Yes — this plan's
  deliverable *is* new stderr output during `install`. The checklist requires
  the plan to say so rather than letting it arrive silently. Stdout is
  unchanged.
- **Diagnostics through `CliContext`.** The new `Content-Length` disagreement
  fact goes through `context.diagnostic`. Progress itself is deliberately *not*
  a `CliContext` diagnostic — it is unconditional operational feedback, not
  `-v` diagnostics, and routing it through `CliContext` would wrongly make it
  verbose-gated.
- **What `-v` adds that normal output does not:** resolved artifact paths, the
  reuse-versus-download decision per model, and any `Content-Length`
  disagreement. Progress answers "is it moving"; `-v` answers "what was
  decided".
- **No re-resolved paths for formatting.** The sink receives already-resolved
  model id and path values from the call site.
- **JSON stdout parseable, no ANSI.** Enforced by the non-TTY bound test.

## Follow-Ups (Not This Deliverable)

1. **Stall detection.** The 3600 s timeout should be paired with a much shorter
   idle timeout — no bytes for N seconds is a failure worth surfacing. Needs
   its own proposal because it changes failure behavior.
2. **Resumable downloads.** A 1.19 GiB transfer that fails at 95% currently
   restarts from zero.
3. **Progress for `index` / `index-all`**, which has the same shape.

## Definition Of Done

1. All three silent stalls report progress.
2. Verification gates above pass.
3. Evidence recorded in this plan's outcome section.
4. README documents the expected install output.
5. Status moves `Draft` → `Active` → `Completed`; `wiki/index.md` and
   `wiki/log.md` updated.
6. The observability checklist gains a long-running-operation clause per the
   promotion target.
