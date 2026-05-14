# Wiki Init Skill

- Document Class: Spec
- Status: Active
- Date: 2026-05-08
- Category: Tooling
- Scope: The `wiki-init` agent skill as a thin conversational wrapper over the `llm-wiki init` binary command.
- Sources: assets/skills/wiki-init/SKILL.md, src/init/mod.rs, src/init/blueprints.rs, src/init/packs.rs, wiki/decisions/llm-wiki-binary-distribution.decision.md, wiki/decisions/binary-path-bootstrap.decision.md, wiki/decisions/composable-project-init.decision.md, wiki/decisions/code-pack-cli-blueprint.decision.md
- Related: wiki/specs/documentation-model.spec.md, wiki/specs/wiki-ingest-skill.spec.md, wiki/decisions/composable-project-init.decision.md, wiki/decisions/code-pack-cli-blueprint.decision.md, wiki/plans/composable-project-init.plan.md

## Contract

`wiki-init` collects setup answers and delegates deterministic project
scaffolding to the binary:

```bash
llm-wiki init <path> --non-interactive --name <name> --description <description> --blueprint <blueprint>
```

The agent owns conversation, validation, and the optional handoff to ingest.
The binary owns filesystem writes, blueprint defaulting, pack composition,
template rendering, framework-artifact collision checks, and initial-source
copying.

## Required Answers

The wrapper gathers the binary's flag set:

1. target path
2. project name
3. one-sentence description
4. blueprint: `generic`, `web-product`, `library-sdk`, `cli-tool`,
   `ml-research`, `ops-infra`, `security`, `research`, or `custom`
5. optional pack overrides: `api`, `frontend`, `library`, `ml`, `data`,
   `ops`, `ops-lite`, `security`, `research`, `qmd-rs-scale`, or `code`
6. optional initial source paths

When no `--pack` flags are supplied, the binary uses the selected blueprint's
default pack selection. Supplying one or more `--pack` flags replaces the
blueprint defaults with that explicit pack set.

Each explicit pack becomes one `--pack <pack>` flag.
Each initial source becomes one `--initial-sources <path>` flag.

## Runtime Projection

Canonical source: `assets/skills/wiki-init/SKILL.md`.
Rendered runtime outputs are produced by:

```bash
llm-wiki build --out .
```

Global installation is produced by:

```bash
llm-wiki install
```

Installed global skills render the binary invocation through the managed
runtime path (`~/.llm_wiki/bin/llm-wiki` on Unix-like systems) so the wrapper
does not require `llm-wiki` to be discoverable on shell `PATH`.

## Blueprint and Pack Flow

Interactive init uses the same model as non-interactive init:

1. select one blueprint
2. review the pack multiselect with the blueprint defaults preselected
3. accept defaults or choose an explicit pack set

The `custom` blueprint starts with no pack defaults.

## Rerun Behavior

When `llm-wiki init <path>` targets a project with `.llm_wiki/init.toml`,
interactive prompts are prefilled from the current project manifest. The
manifest records project name, project description, blueprint, resolved packs,
and framework version. Projects initialized before name and description were
recorded fall back to generated `wiki/index.md`, `AGENTS.md`, or
`project_guidelines.md` when those values can be recovered.

The pack multiselect uses the manifest's saved pack set when the selected
blueprint still matches the manifest blueprint. If the user switches blueprint
during rerun, pack defaults follow the newly selected blueprint so changing
project shape exposes the expected pack set. Saved packs from the previous
blueprint are not preselected after a blueprint switch.

Rerun init refreshes framework-owned root schema files and creates folders for
the selected pack set. It does not overwrite existing `wiki/index.md` or
`wiki/log.md`, because those files are live project knowledge after bootstrap.
Fresh init still refuses paths containing framework artifacts unless a project
manifest is present. When registration is enabled, rerun init updates the
existing registry entry matched by canonical project root rather than creating
a duplicate; the existing project id remains stable.

## Proven By

- `llm-wiki init` has golden tests for baseline, ML, qmd-rs, combined ML+qmd-rs,
  `research`, `web-product`, `cli-tool`, `ml-research`, and `ops-infra`
  profiles.
- Code-pack tests assert `research` has no root code folders by default, while
  `web-product`, `cli-tool`, and explicit `--pack code` do and record `code` in
  `.llm_wiki/init.toml`.
- `llm-wiki init` refuses paths containing framework artifacts.
- Initial-source tests assert files are copied into `raw/initial/` and no ingest results appear in `wiki/`.
- The init manifest records project name, project description, blueprint,
  resolved packs, and framework version in `.llm_wiki/init.toml`.
- Rerun tests assert existing manifests prefill current answers, selected
  packs can be changed, new pack folders are created, and existing
  `wiki/index.md` / `wiki/log.md` content is preserved.
- Registry rerun tests assert project rename updates the existing same-root
  registry entry without changing its id or creating a duplicate.
- The `wiki-init` canonical is embedded at compile time and projection snapshots lock both runtime variants.
- Post-install integration tests assert installed skills contain the managed binary path and that the managed binary executes under a sanitized `PATH`.
