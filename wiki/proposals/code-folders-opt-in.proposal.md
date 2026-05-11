# Code Scaffolding As An Opt-In Pack

- Document Class: Proposal
- Status: Proposed
- Date: 2026-05-10
- Category: Init UX, project scaffolding, blueprint/pack catalog
- Scope: Stop creating `src/`, `tests/`, `scripts/`, `infra/` unconditionally during `llm-wiki init`. Move them behind an opt-in `code` pack so research-only and documentation-only projects do not start with empty code folders.
- Sources: src/init/compose.rs (CODE_FOLDERS, lines 25, 73-75), templates/base/project_guidelines.md (lines 264-268), src/init/blueprints.rs, src/init/packs.rs, dogfooding session 2026-05-10 against `/Users/nicolasmartino/Documents/car/electric` (research blueprint)
- Related: wiki/proposals/blueprint-pack-init.proposal.md, wiki/decisions/composable-project-init.decision.md, wiki/specs/documentation-model.spec.md

## Question

Should the four code/deploy folders (`src/`, `tests/`, `scripts/`, `infra/`) currently created on every new init move behind an opt-in pack, so research and knowledge-only projects do not get them by default?

## Proposal

Yes. Today `compose.rs:73-75` adds `CODE_FOLDERS` for any project where `is_existing == false`, regardless of blueprint. This contradicts the spirit of the D10 composable-init decision: the spine is the wiki spine, and everything else is supposed to be a pack the user opts into. Code scaffolding is currently a hidden, unstoppable pack with no way to decline it.

Concretely:

1. Introduce a new `Pack::Code` variant whose `folders()` returns `["src", "tests", "scripts", "infra"]` and whose `agents_fragment()` / `guidelines_fragment()` contribute the existing "Application code / Automated tests / Utilities / Infrastructure" prose currently inlined in `templates/base/project_guidelines.md:264-268`.
2. Remove the `CODE_FOLDERS` constant and the unconditional `if !plan.is_existing` block from `compose.rs`. Remove the `{% if !is_existing %}` block from `templates/base/project_guidelines.md`.
3. Update blueprint default-pack selections in `blueprints.rs:62-72`:
   - `web-product`, `library-sdk`, `ml-research`, `ops-infra`, `security` → add `Code` to defaults.
   - `research`, `generic`, `custom` → no `Code` by default.
4. The non-interactive flag path (`--pack code`) and the interactive `MultiSelect` give the user the final say either direction.

### Why a pack and not a blueprint flag

The framework already has the vocabulary for "thing the user opts into": it is a pack. The blueprint-pack proposal (D10) explicitly says blueprints are "shortcuts to a default pack selection; nothing more." Treating code-folders as a blueprint-conditional special case re-introduces the kind of one-off conditional logic the D10 decision retired.

A pack also gives us free benefits we would otherwise have to invent:

- It appears in `.llm_wiki/init.toml`, so a future `upgrade` knows whether the project asked for code scaffolding.
- It is visible in the interactive `MultiSelect` step, so the user *sees* the choice instead of having it made for them.
- If we later want to split the four folders (e.g. `tests/` shipped separately, `infra/` as its own pack), we are extending an existing surface rather than carving up `compose.rs` again.

### Pack composition

The `code` pack contributes only folders and a small `project_guidelines.md` fragment describing what each folder is for. It contributes no doc types, no status vocabulary, no `wiki/` subdirectories — the wiki spine is unaffected. This keeps it cleanly orthogonal to the existing nine packs, which contribute primarily *wiki* additions.

### Migration

This change affects new inits only. Existing projects keep their code folders (they exist on disk; nothing reads `init.toml` to delete folders). For projects initialized before this change, `init.toml` will not list `code` as a resolved pack; if a future `upgrade` command exists, it should treat absence-of-`code` in legacy manifests as "do not touch existing code folders."

## Why

1. **Dogfooding pain.** A `research` blueprint at `/Users/nicolasmartino/Documents/car/electric` was just initialized to track an electric-cars research project. It has no code, will never have code, and now ships with four empty top-level folders that have to be either explained away or manually deleted. This is the exact "first two real bootstrapped projects" cohort the D10 proposal flagged as the moment to revisit the catalog.
2. **The current behavior is invisible to the user.** The `inquire` `MultiSelect` shows nine packs the user can tick or untick. The `CODE_FOLDERS` block is a tenth, hidden pack the user cannot see and cannot decline. That is exactly what the D10 decision said we would stop doing.
3. **No one-size template fits research and software both.** A docs-only or research project does not want `src/`; a software project does want it. A blueprint is a *recommendation* of pack defaults, not a forced bundle. Today, the `is_existing` flag is being asked to do work the pack system should be doing.
4. **Cheap change.** One enum variant, one fragment file under `templates/packs/code/`, three default-pack tweaks in `blueprints.rs`, and the deletion of `CODE_FOLDERS`. Golden-file tests for `research` (no code) and `web-product` (with code) lock the behavior in.

## Alternatives Considered

1. **Make code folders blueprint-conditional inside `compose.rs`.** Cheapest possible fix — a `match plan.blueprint` branch around the existing `extend(CODE_FOLDERS)`. Rejected: it puts blueprint-specific behavior back into `compose.rs`, which D10 deliberately moved out into the pack catalog.
2. **Add a `--no-code` flag.** Lightweight but solves the wrong problem: it makes code-folders the default forever, when they should be one option among many.
3. **Delete code folders entirely; make every project wiki-only.** Too aggressive — the framework legitimately scaffolds software projects too, and `templates/base/project_guidelines.md` already documents `src/` etc. as part of the "documentation and execution model." Removing them would break the software use case.
4. **Defer until someone actually complains in production.** They have, just now. Defer is no longer free.

## Consequences and Tradeoffs

- The pack catalog grows by one entry (10 → 11 packs in the `MultiSelect`), all of them now genuinely opt-in.
- Five blueprints (`web-product`, `library-sdk`, `ml-research`, `ops-infra`, `security`) need their `default_packs()` updated to include `Code`. Forgetting one means a software project that used to have `src/` no longer does — golden-file tests catch this.
- `templates/base/project_guidelines.md` loses its `{% if !is_existing %}` branch. The same prose moves to `templates/packs/code/project_guidelines.md` and is composed in like any other pack fragment.
- The `is_existing` profile flag still has a job (suppressing first-run language elsewhere in the template), but it stops gating folder creation. One responsibility, not two.
- Existing projects are unaffected on disk; their `init.toml` simply will not record `code` as a resolved pack. A future `upgrade` command needs to handle this absence gracefully.

## What Would Close This Proposal

Acceptance, promotion to a decision and a small implementation plan, then:

1. New `Pack::Code` variant with folders, agents fragment, guidelines fragment.
2. `templates/packs/code/{agents.md,project_guidelines.md}` fragment files.
3. Removal of `CODE_FOLDERS` from `compose.rs` and the corresponding template branch.
4. Updated `Blueprint::default_packs()` for the five software-shaped blueprints.
5. Golden-file tests asserting that `research` produces no `src/` and `web-product` does.
6. A note in the D10 decision's "first dogfooding revisions" follow-up.

## Remaining Question

Whether to split `code` further (e.g. a separate `infra` pack for projects that ship deploy manifests but no application source). Defer until a project actually asks for that shape; same rule the D10 catalog used.
