# Plan: Composable Project Init

- Document Class: Plan
- Status: Completed
- Date: 2026-05-08
- Category: Tooling, project scaffolding, template engine adoption
- Scope: Implement D10 composable init: introduce a compile-time template engine, migrate the existing init template onto it, replace generated `CLAUDE.md` output with `AGENTS.md`, retire the static template assets under `assets/templates/`, and ship the blueprint + pack composition system from `wiki/decisions/composable-project-init.decision.md`.
- Sources: wiki/proposals/blueprint-pack-init.proposal.md, wiki/decisions/composable-project-init.decision.md, wiki/references/askama-template-engine.reference.md, templates/base/project_guidelines.md, templates/base/agents.md, templates/packs/, src/init/{blueprints,packs,compose,manifest,answers,template,scaffold,command}.rs
- Related: wiki/roadmaps/framework-v1.roadmap.md (D10), wiki/proposals/skills-template-engine.proposal.md, wiki/specs/wiki-init-skill.spec.md, wiki/references/askama-template-engine.reference.md

Post-D11 note: this plan was written before the skill family was renamed from
`knowledge*` to `wiki-*`. Historical phase text below may still name
`knowledge-init`; the current active initialization spec is
`wiki/specs/wiki-init-skill.spec.md`.

Post-D10 rerun note: on 2026-05-13, `init` gained a narrow rerun mode for
projects with `.llm_wiki/init.toml`. The manifest now stores project name and
description as setup answers, interactive reruns prefill from the manifest, and
rerun writes preserve existing `wiki/index.md` and `wiki/log.md`.

## Deliverable

D10: `llm-wiki init` produces a tailored `AGENTS.md` and `project_guidelines.md` from a chosen blueprint plus a selected set of packs, rendered through a compile-time template engine. For Claude compatibility, init also writes a small `CLAUDE.md` shim that points at `AGENTS.md`; `AGENTS.md` is the canonical generated schema file. A per-project `.llm_wiki/` folder records setup choices in `init.toml` for reruns and a future `upgrade` command. The existing static template is retired in the same change set.

Two contrasting blueprints (`ml-research` and `ops-infra`) bootstrap green wikis end-to-end, including all expected folders, doc types, and status vocabulary, with golden-file coverage proving the rendered output is byte-stable.

## Existing Implementation Touchpoints

Inspect these sites before changing code:

- `src/init/template.rs` — current ad-hoc `{{PROJECT_NAME}}`/`{{DATE}}` substitution. Becomes a thin wrapper around the engine.
- `src/init/scaffold.rs` — folder and file creation. Extends to per-pack folder additions and switches generated schema output from `CLAUDE.md` to `AGENTS.md`.
- `src/init/profile.rs`, `src/init/answers.rs` — current question flow. Step-1 (blueprint) and step-2 (pack multiselect) hook in here.
- `src/init/command.rs` and `src/cli.rs` — the CLI entry point and flag surface; gain `--blueprint <name>` and repeatable `--pack <name>` flags for the non-interactive path.
- `assets/templates/project_guidelines.md`, `assets/templates/CLAUDE.md` — the templates being migrated and split. The generated schema target changes from canonical `CLAUDE.md` to canonical `AGENTS.md` plus a `CLAUDE.md` compatibility shim.
- `Cargo.toml` — adds `askama`, `inquire`, and `toml` to `[workspace.dependencies]` and `[dependencies]`.
- `tests/init.rs` (and any existing `cargo insta` snapshots covering init) — the golden-file harness extends to cover the new blueprint × pack matrix.
- Active wiki pages that still describe generated `CLAUDE.md` output or the old static-template asset path. D10 must leave active documentation aligned with the shipped scaffold behavior.

## Implementation Clarifications

- Template files use `.md` filenames under `templates/`, not `.md.jinja`. Askama is invoked with `escape = "none"` for Markdown output. This avoids relying on an unknown `jinja` extension while keeping Jinja-like Askama syntax.
- Askama dependency starts at `askama = "0.16"` based on 2026-05-07 package metadata, then local `cargo build` validates resolution before template migration begins.
- Do not add `askama.toml` for init unless implementation proves it necessary; the root binary crate can use Askama's default root `templates/` directory.
- Pack fragments are compiled templates selected by exhaustive Rust `match` arms. No runtime template path lookup is introduced. `Pack` accessors expose rendered fragment content (or fragment enum values), not arbitrary string paths.
- Pack fragment accessors are fallible: prefer `anyhow::Result<Option<String>>` over plain `Option<String>` so Askama render errors propagate through `init`.
- Base templates expose fixed insertion points by rendering `agents_fragments` and `guidelines_fragments` from the typed render context. Do not use Askama inheritance blocks as dynamic pack anchors.
- Preserve Askama's default whitespace mode initially; use local whitespace markers only when snapshots show drift.
- The old `ProjectProfile` booleans remain only as a Phase 1 compatibility bridge to prove the engine migration is byte-stable. By the end of D10, `ml` and `qmd-rs-scale` packs replace `include_ml_ai` and `include_qmd`.
- The final CLI surface for non-interactive init is `--blueprint <name>` plus repeatable `--pack <name>`. The old `--type` and `--scale` flags are retired in the same change set, with `knowledge-init`, README, and specs updated accordingly.

## In Scope

- The `askama` compile-time template engine.
- A `templates/` root containing `templates/base/` (project-guidelines spine and agent schema) and `templates/packs/<name>/` (per-pack fragments).
- Rust enums `Pack` and `Blueprint` in `src/init/` with accessor methods (`folders`, `doc_types`, `status_vocab`, `agents_fragment`, `guidelines_fragment`, `default_packs`).
- The full pack catalog from the accepted proposal: `api`, `frontend`, `library`, `ml`, `data`, `ops`, `ops-lite`, `security`, `research`, `qmd-rs-scale`.
- The full blueprint catalog: `generic`, `web-product`, `library-sdk`, `ml-research`, `ops-infra`, `security`, `research`, `custom`.
- Two-step interactive flow with `inquire`: `Select` over blueprints, then `MultiSelect` over packs with the blueprint's defaults pre-checked.
- Non-interactive path: `--blueprint <name>` and repeatable `--pack <name>` flags, with the same defaulting rules.
- Per-project `.llm_wiki/` folder, with `init.toml` recording setup answers,
  including project name, project description, chosen blueprint, resolved pack
  list, and framework version.
- Migration of the existing init template onto the same engine in this change set.
- Migration of generated schema output from `CLAUDE.md` to `AGENTS.md` in the same change set.
- Retirement of `<!-- SECTION:ML_AI -->` / `<!-- SECTION:SEARCH -->` flags (replaced by the `ml` and `qmd-rs-scale` packs).
- Golden-file snapshots for `ml-research` and `ops-infra` rendering, plus at least one `custom`-with-no-packs control case.
- Documentation updates: the active init-skill spec reflects the new flow and
  flag surface, and active wiki pages stop describing generated `CLAUDE.md`
  output except where preserved as historical record in archive/log context.

## Out Of Scope

- Skill projection migration onto the engine (see `wiki/proposals/skills-template-engine.proposal.md` — sibling, follow-on).
- An `upgrade` command. `.llm_wiki/init.toml` is written for that future, not this one.
- Project-local overrides under `.llm_wiki/` beyond the init manifest.
- User-defined packs (defining a pack means writing Rust; that is a framework-release activity).
- Specs promotion before behavior is implemented and tested.

## Phases

### 0. Add dependencies

1. Add `askama = "0.16"`, `inquire`, and `toml` to `Cargo.toml` (`[workspace.dependencies]` and `[dependencies]`). Engine choice is fixed by the decision; no spike required.
2. Confirm `cargo build` and the existing test suite stay green with the new deps in place.
3. Build a 30-line throwaway proof rendering a hello-world `askama` template inside the binary, just to verify the macro derive and the `templates/` discovery are wired correctly before Phase 1 starts touching real templates. Delete it once Phase 1 lands the real migration. Completed by validating Askama discovery through the real base templates during the first implementation slice.

### 1. Migrate the existing init template

1. Move `assets/templates/project_guidelines.md` and `assets/templates/CLAUDE.md` into `templates/base/project_guidelines.md` and `templates/base/agents.md`.
2. Replace `{{PROJECT_NAME}}`, `{{PROJECT_DESCRIPTION}}`, `{{DATE}}` substitutions with engine syntax driven by a typed `BaseContext` struct.
3. Rewrite the schema-file references in the base templates from `CLAUDE.md` to `AGENTS.md`. The generated project shape after D10 is `raw/` + `wiki/` + `AGENTS.md` + `project_guidelines.md`; the old filename survives only in archived history and log provenance.
4. Translate the existing `<!-- SECTION:ML_AI -->` / `<!-- SECTION:SEARCH -->` blocks into engine `{% if %}` blocks driven by booleans on `BaseContext`. (These booleans become pack-derived in Phase 4 — for now they are explicit fields, used to keep golden-file output byte-identical to the current `init` behavior where the content is otherwise unchanged.)
5. Rewrite `src/init/template.rs` to populate `BaseContext` and call `.render()`.
6. Start writing `AGENTS.md` as the canonical rendered schema file and `CLAUDE.md` as `See @AGENTS.md.`.
7. Verify the migrated output is byte-stable relative to the intended post-D10 baseline, with the filename/schema-file rename called out as the deliberate diff from the pre-D10 scaffold.

### 2. Pack and Blueprint enums

1. Add `Pack` and `Blueprint` enums in `src/init/packs.rs` (new file) and `src/init/blueprints.rs` (new file). Wire from `src/init/mod.rs`.
2. Implement accessors on `Pack`: `name() -> &'static str`, `folders() -> &'static [&'static str]`, `doc_types() -> &'static [DocType]`, `status_vocab() -> &'static [StatusEntry]`, `agents_fragment() -> anyhow::Result<Option<String>>`, `guidelines_fragment() -> anyhow::Result<Option<String>>`. Stub the fragment methods to `Ok(None)` for now; folders/doc-types/status-vocab return the values from the accepted proposal's pack catalog.
3. Implement `Blueprint::default_packs() -> &'static [Pack]` per the catalog.
4. Unit tests: every blueprint's default pack list is a subset of the full pack catalog; every pack's accessors are non-empty where the catalog says they should be.

### 3. Render plan composer

1. In `src/init/compose.rs` (new file), define `RenderPlan { blueprint: Blueprint, packs: Vec<Pack> }` and a `compose(plan: &RenderPlan) -> InitOutput` that produces the full file write list (path → contents) without touching disk.
2. Compose logic: render each selected pack fragment through an exhaustive Rust `match`, pass the resulting fragment strings into the base template context, and let the base templates render them at fixed insertion points.
3. Folders: union of the spine folders and each pack's `folders()`. Idempotent — if two packs declare the same folder, it is created once. (No conflict-resolution logic needed; folders are de-duplicated by string.)
4. Status vocabulary: union of the spine's vocab and each pack's `status_vocab()`. Same de-duplication rule.
5. Snapshot tests for `compose(...)` output against `ml-research`-default and `ops-infra`-default plans.

### 4. First three packs end-to-end

Implement the smallest set of packs that exercises every code path: `ml`, `ops`, `qmd-rs-scale`. For each:

1. Create `templates/packs/<name>/agents.md` and `templates/packs/<name>/project_guidelines.md`.
2. Wire the pack's `agents_fragment` / `guidelines_fragment` accessors to render the new template files through explicit Rust matches.
3. Add the pack's folders, doc types, and status vocab from the accepted proposal.
4. Snapshot tests for each pack rendered standalone (against a `custom` blueprint with only that pack ticked).

### 5. Remaining packs and blueprints

1. Repeat Phase 4 for the remaining packs: `api`, `frontend`, `library`, `data`, `ops-lite`, `security`, `research`.
2. Snapshot tests for each remaining named blueprint at default pack selection.

### 6. Two-step interactive flow

1. Add `inquire`-driven `Select` over blueprints in `src/init/answers.rs`.
2. Add `MultiSelect` over packs with `with_default` set to `blueprint.default_packs()`.
3. The `custom` blueprint shows the multiselect with no defaults checked.
4. Manual smoke test: run `llm-wiki init` interactively and verify the prompt sequence.

### 7. Non-interactive flag mapping

1. Add `--blueprint <name>` and repeatable `--pack <name>` to `src/init/command.rs`. Validate against the enums.
2. The final `--non-interactive` mode requires `--blueprint`; packs default from the blueprint unless `--pack` overrides. `--type` and `--scale` are removed or rejected with guidance to use `--blueprint` / `--pack`.
3. Integration test through `assert_cmd`: run `llm-wiki init --non-interactive --blueprint ml-research <path>` and assert the resulting wiki has `experiments/`, `evals/`, and the ML status vocab present.

### 8. `.llm_wiki/init.toml` writer

1. Define `InitManifest { blueprint: Blueprint, packs: Vec<Pack>, framework_version: String }` with `serde::Serialize`.
2. Write `.llm_wiki/init.toml` as the last step of `init`, after all other files have been written successfully.
3. D10 originally only wrote the manifest. Post-D10 rerun init now reads it
   back for prompt defaults, while a full `upgrade` command remains out of
   scope.

### 9. Documentation and cleanup

1. Update the active init-skill spec to describe the new flag surface and the two-step flow.
2. Update `wiki/specs/documentation-model.spec.md`, `wiki/decisions/three-layer-architecture.decision.md`, `wiki/decisions/llm-wiki-binary-distribution.decision.md`, and any other active wiki page that still claims generated projects use `CLAUDE.md`, so active docs match the shipped D10 scaffold. Historical mentions remain only in `wiki/archive/` and `wiki/log.md`.
3. Update `README.md` if it documents the old static template behavior or the old generated schema filename.
4. Delete `assets/templates/project_guidelines.md` and `assets/templates/CLAUDE.md` (now superseded by `templates/base/`).
5. Update `wiki/index.md` to reflect any renamed or materially revised active pages.
6. Append to `wiki/log.md`.

## Verification Gates

1. `cargo test --workspace` is green.
2. `cargo build --release` produces a binary that runs `llm-wiki init` interactively to completion.
3. Two integration tests through `assert_cmd`: `--blueprint ml-research` and `--blueprint ops-infra` non-interactive runs each produce green wikis with expected files, folders, doc types, and status vocabulary. `llm-wiki doctor` remains an install/runtime diagnostic unless it gains a project-wiki validation mode during implementation.
4. Snapshot tests for the base template, every pack standalone, and every named blueprint at default selection. Byte-stable across re-runs.
5. The migrated existing init template's rendered content is byte-stable for at least one fixed input set aside from the intentional D10 diffs: `AGENTS.md` replaces `CLAUDE.md`, and pack-driven sections replace legacy conditional markers.
6. No active wiki page, README section, or generated scaffold output claims that `init` writes `CLAUDE.md`; that filename appears only in archive/log history or in code/docs that explicitly describe pre-D10 behavior.
7. `.llm_wiki/init.toml` round-trips: `init` writes it, a follow-up read parses it back into the same enum values.

## Pages To Update On Completion

- `wiki/plans/composable-project-init.plan.md` — Status → Completed.
- `wiki/decisions/composable-project-init.decision.md` — no status change; remains Accepted.
- `wiki/proposals/blueprint-pack-init.proposal.md` — Status already Accepted with Promoted To set; verify still current.
- `wiki/specs/wiki-init-skill.spec.md` — describes the new flow and flags.
- `wiki/specs/documentation-model.spec.md` — references the composable init flow and the post-D10 generated schema file shape.
- `wiki/decisions/three-layer-architecture.decision.md` — update if the project-level schema artifact described there changes from `CLAUDE.md` to `AGENTS.md` for generated projects.
- `wiki/decisions/llm-wiki-binary-distribution.decision.md` — update if it still describes `CLAUDE.md` as generated scaffold output rather than historical D8 behavior.
- `wiki/roadmaps/framework-v1.roadmap.md` — D10 status → Completed.
- `wiki/index.md` — entry status updates.
- `wiki/log.md` — completion entry.

## What Closes The Plan

D10 closed when `cargo insta test --workspace --accept` passed, the
`ml-research` and `ops-infra` non-interactive snapshots proved contrasting
blueprint output, and `.llm_wiki/init.toml` was generated from the resolved
pack set.
