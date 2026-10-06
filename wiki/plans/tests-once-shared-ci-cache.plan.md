# Plan: Run The Tests Once And Share One CI Cache

- Document Class: Plan
- Status: Active
- Branch: `NicolasMartino/tests-once-15`
- Date: 2026-10-06
- Category: CI, development process
- Scope: Make `just verify`, and the full CI jobs that run it, run the test
  suite once instead of twice, and save the CI cache only from `develop` and
  master, so every PR starts from `develop`'s cache.
- Sources:
  - The owner's decisions of 2026-10-06: "I think it should get all cargo
    crates once, then run all tests", and the follow-up to the develop and
    master CI (#12), approved the same day
  - The runs of PR #12, read by the coordinator
- Related:
  - `wiki/roadmaps/framework-v1.roadmap.md`, P13
  - `wiki/plans/develop-and-master-ci.plan.md`
  - `justfile`, recipes `verify`, `test`, `snapshots`, `coverage`, `fast-check`
  - `.github/workflows/ci.yml`, `.github/workflows/fast-check.yml`

## What This Proves

Each run compiles once and runs every test once, and a PR's first CI run is
as fast as its later ones.

## Where It Stands (2026-10-06, rechecked on `develop` at 81ffab3)

- `just verify` runs `fmt`, `test` (`cargo test --workspace` and the
  `tools/release-e2e` tests), `clippy-strict`, `snapshots`
  (`cargo insta test --workspace --check`, which runs every workspace test
  again) and `audit-legacy`: 26 s warm locally, each test binary run twice.
- The full CI's `test` job (`ubuntu-latest`, `ubuntu-24.04-arm`, `macos-14`)
  runs `just verify` and then `just coverage` (`cargo llvm-cov --workspace
  --fail-under-lines 80`), which runs the workspace tests a third time, under
  coverage. Its last warm run (on `ci-9`, before PM1) took 3 min 30 s for
  `just verify` and 1 min 37 s for coverage on `ubuntu-latest`.
- Since PM1 (#28): `rust-toolchain.toml` pins 1.99.0, `tools/udeps-nightly`
  names the dated nightly, and both workflows have a `strict` job
  (`just strict`, `tools/strict-gates.sh`) that runs its own tests and coverage
  over the two strict crates. `just strict` is not part of this plan.
- `just fast-check` already runs its tests once.
- `Swatinem/rust-cache` saves a cache from every run, PR runs included, keyed
  by job id, OS and toolchain (so the two `strict` jobs share one). A PR's
  first run restores `develop`'s cache only if `develop` saved one. Every full
  CI run since PM1 started cold: the toolchain pin changed every key.
- Windows is not in the full CI: it joins once Windows support passes its gates
  (`wiki/roadmaps/cross-platform-release-e2e.roadmap.md`, rule 4).

## Target

- `just verify` runs the suite once, with `cargo insta test --workspace --check`
  as its test step, plus the `tools/release-e2e` tests; no test or check is
  dropped. `just test` may stay as a plain run for people.
- The full CI runs the tests once per job where coverage allows. If coverage
  needs its own instrumented run, keep it and say so; coverage is not dropped.
- The cache is saved only on pushes to `develop` and master (`save-if`); PR runs
  only restore it. Job names stay as they are.

## How It Runs

- `just verify` (fmt, `snapshots`, `test-tools`, clippy-strict, audit-legacy,
  branch-status-test): every workspace test runs once, in
  `cargo insta test --workspace --check`, which fails on a test or on a
  snapshot that does not match; `test-tools` runs the `tools/release-e2e`
  tests, a workspace of their own. `just test` stays a plain `cargo test` run.
- `just verify-coverage` is `just verify` with `coverage` in place of
  `snapshots`: the same one run, under coverage instrumentation
  (`cargo llvm-cov show-env`, built into `target/llvm-cov-target`), then
  `cargo llvm-cov report --fail-under-lines 80` over it. The full CI's `test`
  job runs it as one step, and `just verify-full` is it plus `udeps`. The
  coverage figure is the same as `cargo llvm-cov --workspace` gave.
- `just fast-check` runs the quick tests once, as before, and now also
  `branch-status-test`, the check `just verify` runs on the `branch-status`
  recipe.
- The cache: every `Swatinem/rust-cache` step saves only on a push to
  `develop` or master (`save-if`). A PR into `develop` restores the cache
  `develop`'s last push saved for the same job id (GitHub lets a branch read
  its base's and the default branch's caches), so its first run starts as warm
  as `develop`. The full CI's `test` and `udeps` jobs save on master, and the
  PR from `develop` into master reads them; a manual full CI run on a work
  branch finds no cache for those two jobs and starts cold. The two `strict`
  jobs share one key, so `develop`'s fast check warms both.

## Done When

- The `just verify` log shows one test run, and it exits 0 with nothing skipped.
- The fast check passes on the PR, and a manual full CI run passes on its head.
- The PR gives the warm-cache durations before and after, locally and on CI.
