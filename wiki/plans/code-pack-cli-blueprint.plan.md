# Plan: Code Pack And CLI Tool Blueprint

- Document Class: Plan
- Status: Planned
- Date: 2026-05-11
- Category: Init UX, project scaffolding, blueprint/pack catalog
- Scope: Implement the accepted code-pack and `cli-tool` blueprint decision without changing the framework's universal wiki spine.
- Sources: wiki/decisions/code-pack-cli-blueprint.decision.md, wiki/proposals/code-folders-opt-in.proposal.md, wiki/decisions/composable-project-init.decision.md, src/init/compose.rs, src/init/blueprints.rs, src/init/packs.rs, templates/base/project_guidelines.md, templates/packs/
- Related: wiki/decisions/code-pack-cli-blueprint.decision.md, wiki/proposals/code-folders-opt-in.proposal.md, wiki/decisions/composable-project-init.decision.md, wiki/specs/wiki-init-skill.spec.md

## Goal

`llm-wiki init` should stop creating `src/`, `tests/`, `scripts/`, and
`infra/` unless the resolved pack set contains `code`.

The named `cli-tool` blueprint should be available anywhere blueprints are
listed or parsed, default to the `code` pack, and be recorded in generated
`.llm_wiki/init.toml` manifests.

No implementation has landed as part of this planning step. This plan is the
implementation contract for the next coding pass.

## Implementation Steps

1. Add `Pack::Code` in `src/init/packs.rs`.
2. Add `code` to `Pack::ALL`, `Pack::name`, `Pack::description`, and
   `FromStr`/display coverage through the existing enum accessors.
3. Make `Pack::Code::folders()` return `["src", "tests", "scripts", "infra"]`.
4. Keep `Pack::Code::doc_types()` and `status_vocab()` empty.
5. Add `templates/packs/code/agents.md` and
   `templates/packs/code/project_guidelines.md`.
6. Wire `Pack::Code` fragment rendering through explicit Askama template
   matches.
7. Remove `CODE_FOLDERS` from `src/init/compose.rs`.
8. Remove the unconditional `if !plan.is_existing` folder-extension block from
   `compose`.
9. Remove the `{% if !is_existing %}` code-folder section from
   `templates/base/project_guidelines.md`.
10. Add `Blueprint::CliTool` in `src/init/blueprints.rs`.
11. Add `cli-tool` to `Blueprint::ALL`, `Blueprint::name`,
    `Blueprint::description`, and parse/display coverage through the existing
    enum accessors.
12. Update `Blueprint::default_packs()` so:
    - `web-product`, `library-sdk`, `cli-tool`, `ml-research`, `ops-infra`, and
      `security` include `Pack::Code`.
    - `research`, `generic`, and `custom` do not include `Pack::Code`.
13. Confirm the interactive blueprint and pack prompts pick up `cli-tool` and
    `code` through `Blueprint::ALL` and `Pack::ALL`.
14. Confirm the non-interactive path accepts `--blueprint cli-tool` and
    `--pack code` through existing enum parsing.

## Tests

Add or update tests so the behavior is locked at the product boundary:

1. Unit tests still prove every blueprint and pack round-trips through its name.
2. Unit tests allow `code` to have folders but no doc types or status vocab.
3. Non-interactive `--blueprint research` produces no `src/`, `tests/`,
   `scripts/`, or `infra/`.
4. Non-interactive `--blueprint web-product` produces the four code folders.
5. Non-interactive `--blueprint cli-tool` produces the four code folders.
6. Non-interactive `--blueprint generic --pack code` produces the four code
   folders and records `code` in `.llm_wiki/init.toml`.
7. Golden snapshots cover at least `research`, `web-product`, and `cli-tool`.
8. Existing `ml-research` and `ops-infra` snapshots are updated intentionally
   because their default pack sets now include `code`.

## Documentation Updates

1. Update `wiki/specs/wiki-init-skill.spec.md` if it enumerates blueprints or
   pack behavior.
2. Update `wiki/specs/documentation-model.spec.md` if it still describes
   unconditional root code folders in generated projects.
3. Add a completion note to
   `wiki/decisions/composable-project-init.decision.md` only if the
   implementation changes the accepted D10 composition model beyond this
   dogfooding revision.
4. Update `wiki/index.md` and append to `wiki/log.md`.

## Verification

Run these before marking the plan completed:

```bash
cargo fmt
cargo test --workspace
git diff --check
```

If snapshot tests change, review the rendered project structures before
accepting the snapshots.

## Acceptance Criteria

This plan is complete when:

1. `code` is a selectable pack whose folders are created only when selected or
   defaulted by a blueprint.
2. `cli-tool` is a selectable blueprint whose default packs include `code`.
3. Research and generic projects no longer receive root code folders by default.
4. Software-shaped blueprints keep code folders through explicit `code`
   defaults.
5. `.llm_wiki/init.toml` records `code` whenever it is resolved.
6. Tests and snapshots cover the no-code and code-default cases.
7. The proposal, index, and log reflect the implemented state.
