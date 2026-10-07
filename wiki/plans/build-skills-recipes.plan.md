# Plan: Working build-skills Recipes

- Document Class: Plan
- Status: Draft
- Date: 2026-10-07
- Category: Tooling
- Scope: Carry out P8 of the framework roadmap: `just build-skills` and `just
  build-skills-to <out>`, which fail because the subcommand they call is
  gone, are removed, and no page tells anyone to run them.
- Sources:
  - Issue #5, "Tooling: Fix the build-skills recipes"
  - The `justfile`, `src/cli.rs` and `wiki/specs/documentation-model.spec.md`
    at `cad8988`, read for this plan
  - Commit `866a44f` (2026-08-07), which moved llm-wiki to MCP first and took
    the `build` subcommand out of the CLI
- Related:
  - `wiki/roadmaps/framework-v1.roadmap.md`, P8 (this plan) and P4 (which
    found the failure)
  - `wiki/plans/development-workflow-setup.plan.md` (where the failure was
    first written down)
  - `wiki/decisions/skill-projection-template-engine.decision.md`
    (Superseded: the skill rendering these recipes ran)

## What This Proves

Every recipe in the `justfile` runs; none points at a command the binary no
longer has.

## Where It Stands (2026-10-07, at `cad8988`)

- The `justfile` has `build-skills`, which runs `cargo run --bin llm-wiki --
  build --out .`, and `build-skills-to out`, which runs the same with `--out
  "{{out}}"`.
- `llm-wiki` has no `build` subcommand (`src/cli.rs` lists install, init,
  headroom, mcp, read, eval, register, forget, projects, index, index-all,
  search, search-all, path, status, doctor and uninstall), so both recipes
  stop with "unrecognized subcommand 'build'".
- The subcommand rendered skills for each runtime. The distribution is now MCP
  first: the binary "no longer renders or installs generated runtime
  skills", and the repository's own skills are authored by hand
  (`wiki/specs/documentation-model.spec.md`); the decision behind the
  rendering is Superseded.
- Pages that name the recipes, outside the frozen copies of the wiki under
  `tests/fixtures/`: `wiki/plans/development-workflow-setup.plan.md`, which
  records finding the failure, and the framework roadmap's P4 and P8. None
  tells anyone to run them. No other file names them: no README, CI workflow
  or script.

## Target

- **Both recipes removed from the `justfile`.** There is nothing for them to
  build: skill rendering is gone on purpose (the owner's choice 1).
- **The pages stay as they are**: the setup plan and the roadmap record the
  failure and this fix, and tell no one to run the recipes; the frozen wiki
  copies under `tests/fixtures/` are never edited. The issue's "no page names
  it" is read as "no page tells anyone to run it" (the owner's choice 2).

## Done When

- `just --list` shows neither recipe, and no file outside `wiki/` and
  `tests/fixtures/` names them (searched on the PR's head).
- `just verify` passes locally and the fast check passes on the PR into
  `develop`.

## Open For The Owner

1. **Remove the recipes, not repair them.** Repairing them would mean
   bringing back a skill renderer the MCP-first move retired; nothing in this
   repository or its CI calls them. Not chosen: pointing them at another
   command (`llm-wiki install`, or copying the authored skills somewhere),
   which would give an old name a new meaning.
2. **Leave the pages that record the failure as they are.** They are history,
   and none tells anyone to run the recipes. Not chosen: rewording the
   finished setup plan and the roadmap's P4 to drop the names, which loses
   where the failure was found.

## Out Of Scope

- How skills are written and shipped, beyond removing the two recipes.
- The frozen wiki copies under `tests/fixtures/`.
