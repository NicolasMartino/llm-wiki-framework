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

## Where It Stands (2026-10-06)

- `just verify` runs `test` (`cargo test --workspace` and the
  `tools/release-e2e` tests) and then `snapshots`
  (`cargo insta test --workspace --check`), which runs every test again: about
  35 s locally. The full CI's job runs `just verify` and then `just coverage`,
  which runs the tests a third time under coverage.
- `just fast-check` already runs its tests once.
- `Swatinem/rust-cache` saves a cache from every run, PR runs included. A PR's
  first run restores `develop`'s cache only if `develop` saved one.
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

## Done When

- The `just verify` log shows one test run, and it exits 0 with nothing skipped.
- The fast check passes on the PR, and a manual full CI run passes on its head.
- The PR gives the warm-cache durations before and after, locally and on CI.
