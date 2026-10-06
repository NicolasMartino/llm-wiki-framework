# Plan: The Search Eval Test Runs Against A Frozen Wiki

- Document Class: Plan
- Status: Draft
- Date: 2026-10-06
- Category: Search, tests, CI
- Scope: Make the search quality test in `src/search/qmd_rs.rs` read a frozen
  copy of the wiki instead of the live `wiki/`, without weakening what it
  checks, so the fast check on `develop` is green again and stays green as the
  wiki grows.
- Sources:
  - The owner, 2026-10-06: open a PR from `develop` into master, "and if it
    [CI] fails, fix it"
  - The failed fast check on `develop` at `67c70ce`, run 37485986382, read by
    the coordinator
- Related:
  - `wiki/roadmaps/framework-v1.roadmap.md`, P14 and P9
  - `src/search/qmd_rs.rs`, `tests/fixtures/`

## What This Proves

A search quality check that fails only when search gets worse, not when
someone adds or edits a wiki page.

## Where It Stands (2026-10-06)

- `fixed_eval_queries_keep_expected_targets_in_top_two`
  (`src/search/qmd_rs.rs`, about line 1633) indexes
  `CARGO_MANIFEST_DIR/wiki`, the live wiki, and asserts that each query finds
  one of its expected pages in the top two.
- It passed on `develop` at `7940130` and failed at `67c70ce`, a commit that
  only added wiki text (two roadmap entries and a plan): "qmd-rs scale search
  llm-wiki binary: expected one of [framework-v1 roadmap, documentation-model
  spec] in top two, got [composable-project-init plan, …]". The roadmap grew,
  and its score fell.
- Every PR into `develop` now fails its fast check, and the full CI on the PR
  into master (#14) runs the same test.
- `tests/fixtures/` already holds `eval-corpora/`, `eval-testbed/` and
  `wikis/v1/`.

## Target

- The test indexes a frozen copy of the pages it needs, kept under
  `tests/fixtures/`, and keeps every query and expected target, or replaces a
  target only with the page that held that content when the copy was taken.
- Nothing in the test reads the live `wiki/`.
- No test is skipped, ignored or loosened.

## Done When

- The test passes on the PR, and still fails if search is made worse (show it
  once with a deliberately broken ranking, not committed).
- The fast check passes on the PR into `develop`.
- `just verify` passes locally with nothing skipped.
