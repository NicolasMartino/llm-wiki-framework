# Plan: A Fast Check On Develop, The Full CI On The Way To Master

- Document Class: Plan
- Status: Completed (develop)
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

### Before Issue #9

The state the coordinator read, before this plan's PR changed it:

- `ci.yml` ran on every `pull_request` and on pushes to master: a test job
  over `ubuntu-latest`, `ubuntu-24.04-arm`, `macos-13` and `macos-14`, and an
  "unused dependencies" job. `release.yml` (cargo-dist) ran its plan job on
  every `pull_request`; `release-e2e-linux-amd64.yml` on PRs that touch code.
- CI on master had been red since at least 2026-08-13 (`866a44f`), for three
  reasons:
  - `tests/post_install.rs:198`, `managed_binary_can_self_install`, timed out
    on the Linux jobs ("timed out waiting for …/.llm_wiki/bin/llm-wiki install
    --skip-path-guidance --disable-llm-search"); `macos-14` failed too (its
    cause is below);
  - the unused-dependencies job's `dtolnay/rust-toolchain@nightly-2026-05-01`
    did not resolve: a dated nightly is not an action ref, and the job now
    names it through `dtolnay/rust-toolchain@master` with `toolchain:`;
  - `macos-13` never got a runner: it stayed queued and was cancelled after a
    day, so every run showed "queued" for a day. It is dropped. While a run is queued, its
    jobs' logs are readable only through
    `gh api --allow-escape-sequences repos/NicolasMartino/llm-wiki-framework/actions/jobs/<id>/logs`.

### What The Logs Showed (2026-10-06, issue #9)

- **The self-install timeout:** `llm-wiki install` sha256-hashed the running
  binary up to seven times in one self-install (three or four in a first
  install), and `sha2` was unoptimized in debug builds, where the binary is
  about 170 MB: about 3.4 s a hash on a laptop, more on a runner, against the
  test's 30 s bound. Install now hashes the running binary once and the
  managed binary once (again only after copying over it), and `sha2` builds at
  `opt-level = 3` in the dev profile; a self-install under a temporary HOME
  went from 24 s to under 1 s. The 30 s bound is unchanged.
- **`macos-14`:** CI never installed `cargo-insta`, so `cargo insta test`
  failed with "no such command: insta". The Linux jobs never reached that step.
- **A gate that passed without running:** CI did not install `ripgrep`, and
  `! rg …` in `just audit-legacy` turned "command not found" into a pass; two
  of its paths (`assets`, `crates`) do not exist, and rg's error on them
  passed the same way. `audit-legacy` now fails, naming the tool, when rg is
  missing, and passes only when rg finds nothing; `just snapshots` names
  `cargo-insta` and how to install it when it is missing.
- **The release plan job** runs on `ubuntu-20.04` (cargo-dist 0.28's default),
  a retired image: on every PR it stayed queued and was cancelled. cargo-dist
  has no setting for which PRs it runs on, so `pr-run-mode` is now `"skip"`
  and `release.yml` was regenerated (its `pull_request:` trigger is the only
  change). The release plan job no longer runs on PRs into master either, a
  departure from the Target below that awaits the owner's go. The alternative
  is to keep `pr-run-mode = "plan"` and give the plan job a current runner
  through cargo-dist's `[workspace.metadata.dist.github-custom-runners]`
  `global` setting, which would also unstall a tag release; releases are the
  owner's, so that is left to them.
- **`just branch-status`** names `develop` as the base, and accepts
  `Completed (develop)`, the status a plan takes when its PR merges into
  `develop`.
- **Two tests stay ignored, as they were:** `tests/gguf_cpu_smoke.rs` and
  `tests/natural_language_search_eval.rs` need managed model artifacts under
  `~/.llm_wiki`, and are run by hand.

### How To Run Each Check

The owner, 2026-10-06: "the longer CI different env (ubuntu, macos, windows
etc) tests on PR branch master. PR branch dev is fast checks, cargo clippy,
maybe some llm wiki quick tests and such."

- **The fast check** (PRs into `develop`, pushes to `develop`):
  `.github/workflows/fast-check.yml`, one `ubuntu-latest` job running
  `just fast-check`: fmt, clippy-strict, audit-legacy, the unit tests and the
  quick integration test files with the snapshot check in one
  `cargo insta test --check` run, and the `tools/release-e2e` tests. It needs
  `just`, `cargo-insta` and `ripgrep`, and says which is missing.
- **Left to `just verify` and the full CI:** the integration test files named
  in the justfile's `slow_tests`: `install`, `mcp_install` and `post_install`
  (each installs the binary under a temporary HOME), `properties` and
  `search_commands`. A new test file is in the fast check until it is named
  there.
- **`just verify`** stays the whole local gate: every test, run once by
  `cargo test` and again by `cargo insta test --check`.
- **The full CI** (PRs into master, pushes to master): `.github/workflows/ci.yml`
  (`ubuntu-latest`, `ubuntu-24.04-arm`, `macos-14`, each `just verify` and
  `just coverage`; the unused-dependencies job on `nightly-2026-05-01`) and the
  release E2E on PRs into master that touch code. On any other branch:
  `gh workflow run CI --ref <branch>`. Windows is not in the matrix: llm-wiki
  does not support it yet, and the cross-platform roadmap owns that.

Measured on `ubuntu-latest` with a warm cargo cache, `just verify` as the fast
check (`e32a7b3`, 2 min 53 s for the whole job):

| Step | Time | In `just fast-check` |
| --- | --- | --- |
| Setup (checkout, toolchain, cache, tools) | 29 s | yes |
| fmt | under 1 s | yes |
| `cargo test --workspace` (13 s of it compiling) | 67 s | quick files only |
| of which `search_commands` | 21 s | no |
| of which `install` | 13 s | no |
| of which `properties` | 7 s | no |
| of which `mcp_install`, `post_install` | 4 s | no |
| `tools/release-e2e` tests | 9 s | yes |
| clippy-strict | 5 s | yes |
| `cargo insta test --check` (the whole suite again) | 55 s | quick files only, once |
| audit-legacy | under 1 s | yes |

`just fast-check` as the fast check (`7ad359d`), warm cache: 1 min 17 s for the
whole job, of which setup 26 s, fmt under 1 s, clippy-strict 5 s, the quick
tests with the snapshot check 37 s (11 s of it compiling), the
`tools/release-e2e` tests 9 s. With a cold cache (the first run after the job
was renamed) it took 8 min 15 s, nearly all compiling: a work PR into
`develop` reads the cache `develop`'s own runs save.
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
