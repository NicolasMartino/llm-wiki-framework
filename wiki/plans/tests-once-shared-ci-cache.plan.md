# Plan: Run The Tests Once And Share One CI Cache

- Document Class: Plan
- Status: Completed (develop)
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

- `just verify` is `fmt`, `snapshots` and `checks` (`test-tools`,
  clippy-strict, audit-legacy, branch-status-test): every workspace test runs
  once, in `cargo insta test --workspace --check`, which fails on a failing
  test or on a snapshot that does not match; `test-tools` runs the
  `tools/release-e2e` tests, a workspace of their own. `just test` stays a plain
  `cargo test` run. Warm, locally: 26 s before, 14 s after.
- `just verify-coverage` is `fmt`, `coverage` and the same `checks`:
  `coverage` runs that one test run under coverage instrumentation
  (`cargo llvm-cov show-env`, built into `target/llvm-cov-target`), then
  `cargo llvm-cov report --workspace --fail-under-lines 80` over it, with the
  same totals `cargo llvm-cov --workspace` gave (82.33% lines, 80.61% regions
  on this branch). The full CI's `test` job runs it as one step, and
  `just verify-full` is it plus `udeps`.
- `just fast-check` runs the quick tests once, as before, and now also
  `branch-status-test`, the check `just verify` runs on the `branch-status`
  recipe.
- The cache: the `Swatinem/rust-cache` steps in `fast-check.yml`, `ci.yml` and
  `post-install.yml` save only on a push to `develop` or master (`save-if`).
  A PR into `develop` restores the cache `develop`'s last push saved for the
  same job id (a branch may read its base's and the default branch's caches),
  so its first run starts as warm as `develop`. The key also hashes every
  installed toolchain, so each job in `fast-check.yml` and `ci.yml` removes the
  runner image's toolchains before installing the pinned ones (and the dated
  nightly, which the key then holds): a runner image update no longer changes
  the key. The two `strict` jobs share one key, so `develop`'s fast check warms
  both. The full CI's `test` and `udeps` jobs save on master, and the PR from
  `develop` into master reads them; a manual full CI run on a work branch finds
  no cache for those two jobs and starts cold.
- `post-install.yml` runs only on tags, and a tag's cache can be read by that
  tag alone, so it used to save caches nothing read; it now saves none.
- `release-e2e-linux-amd64.yml` keeps saving: it runs only on PRs into master
  and by hand, never on a push, so a `save-if` would leave it cold on every
  run, and its saves warm the reruns of the PR that made them.

## Done When

- The `just verify` log shows one test run, and it exits 0 with nothing skipped.
- The fast check passes on the PR, and a manual full CI run passes on its head.
- The durations before and after: locally warm (above), and on CI cold against
  cold on the same jobs, because a work branch never saves a full CI cache. The
  last cold run before (37499356182, post-PM1) took 8 min 57 s for
  `just verify` and 4 min 35 s for coverage on `ubuntu-latest`; the PR's full
  gate run gives the run after. The warm full CI time after is read from the
  next PR from `develop` into master, once this merges.
