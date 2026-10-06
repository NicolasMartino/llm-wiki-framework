# Plan: A Fast Check On Develop, The Full CI On The Way To Master

- Document Class: Plan
- Status: Active
- Branch: `NicolasMartino/ci-9`
- Date: 2026-10-06
- Category: CI, development process
- Scope: Split CI by target branch, so that work PRs into `develop` and their
  merges run one fast Linux check, and only the PR from `develop` into master
  and master itself run the full matrix; make both green; point
  `just branch-status` at `develop`.
- Sources:
  - The owner's decisions of 2026-10-06: "maybe the CI on macos, ubuntu etc
    should only run on master so we could have a lighter CI for develop
    branch? They take forever"; "I prefer the develop branch with the merge
    the pr to master triggering the longer CI"; "a merge into develop PR only
    triggers fast CI"; "do the develop/master CI now"
  - The CI runs of 2026-10-06 on master, read by the coordinator
- Related:
  - `wiki/roadmaps/framework-v1.roadmap.md`, P10 and P11
  - `.github/workflows/ci.yml`, `.github/workflows/release.yml`,
    `.github/workflows/release-e2e-linux-amd64.yml`
  - `justfile`, recipes `verify` and `branch-status`

## What This Proves

A work PR into `develop` gets CI's answer in minutes, and master only takes
`develop` through a PR whose full matrix passed.

## Where It Stands (2026-10-06)

- `develop` exists, cut from master at `dbe6dce`, and is GitHub's default
  branch.
- `ci.yml` runs on every `pull_request` and on pushes to master: a test job
  over `ubuntu-latest`, `ubuntu-24.04-arm`, `macos-13` and `macos-14`, and an
  "unused dependencies" job. `release.yml` (cargo-dist) runs its plan job on
  every `pull_request`; `release-e2e-linux-amd64.yml` on PRs that touch code.
- CI on master has been red since at least 2026-08-13 (`866a44f`), for three
  reasons:
  - `tests/post_install.rs:198`, `managed_binary_can_self_install`, times out
    on the Linux jobs ("timed out waiting for …/.llm_wiki/bin/llm-wiki install
    --skip-path-guidance --disable-llm-search"); `macos-14` fails too, cause
    not read yet;
  - the unused-dependencies job's `dtolnay/rust-toolchain@nightly-2026-05-01`
    does not resolve;
  - `macos-13` never gets a runner: it stays queued and is cancelled after a
    day, so every run shows "queued" for a day. While a run is queued, its
    jobs' logs are readable only through
    `gh api --allow-escape-sequences repos/NicolasMartino/llm-wiki-framework/actions/jobs/<id>/logs`.

## Target

- **PRs into `develop`, and pushes to `develop`:** one job on `ubuntu-latest`
  running the steps of `just verify` (fmt, test, clippy-strict, snapshots,
  audit-legacy), and nothing else. It is the gate for work PRs.
- **PRs into master, and pushes to master:** the full matrix
  (`ubuntu-latest`, `ubuntu-24.04-arm`, `macos-14`; `macos-13` dropped), the
  unused-dependencies job with a toolchain pin that resolves, and the release
  plan and release E2E jobs where they run today. A manual run
  (`workflow_dispatch`) of the full CI is possible on any branch.
- **cargo-dist:** if its PR runs are set in its own config, change them there
  and regenerate `release.yml` rather than editing the generated file by hand.
- **`just branch-status`:** `develop` is the base it reports against.
- **Green:** the self-install timeout and the `macos-14` failure are fixed at
  their cause. No test is skipped, ignored or loosened to get there.

## Phases

1. Read the failing jobs' logs and reproduce the self-install timeout
   locally.
2. Split the triggers as above.
3. Fix the three causes of red.
4. Point `just branch-status` at `develop`.
5. Prove it on the PR itself.

## Done When

- The PR into `develop` shows only the fast check, and it passes.
- A manual run of the full CI on the PR's head passes every job.
- `just verify` passes locally with nothing skipped.
- `just branch-status` names `develop` as the base.

## Out Of Scope

- Releases, tags, and the post-install workflow on tags.
- The pages and rules that still say "master" (P11).
