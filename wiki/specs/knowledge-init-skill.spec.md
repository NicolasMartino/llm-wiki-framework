# Knowledge Init Skill

- Document Class: Spec
- Status: Active
- Date: 2026-05-07
- Category: Tooling
- Scope: The `knowledge-init` agent skill as a thin conversational wrapper over the `llm-wiki init` binary command.
- Sources: assets/skills/knowledge-init/SKILL.md, src/init/mod.rs, src/init/blueprints.rs, src/init/packs.rs, wiki/decisions/llm-wiki-binary-distribution.decision.md, wiki/decisions/binary-path-bootstrap.decision.md, wiki/decisions/composable-project-init.decision.md
- Related: wiki/specs/documentation-model.spec.md, wiki/specs/knowledge-ingest-skill.spec.md, wiki/decisions/composable-project-init.decision.md, wiki/plans/composable-project-init.plan.md

## Contract

`knowledge-init` collects setup answers and delegates deterministic project
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
4. blueprint: `generic`, `web-product`, `library-sdk`, `ml-research`,
   `ops-infra`, `security`, `research`, or `custom`
5. optional pack overrides: `api`, `frontend`, `library`, `ml`, `data`,
   `ops`, `ops-lite`, `security`, `research`, or `qmd-scale`
6. whether the target is an existing codebase
7. optional initial source paths

When no `--pack` flags are supplied, the binary uses the selected blueprint's
default pack selection. Supplying one or more `--pack` flags replaces the
blueprint defaults with that explicit pack set.

`--existing` is passed when adding the framework to existing code.
Each explicit pack becomes one `--pack <pack>` flag.
Each initial source becomes one `--initial-sources <path>` flag.

## Runtime Projection

Canonical source: `assets/skills/knowledge-init/SKILL.md`.
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

## Proven By

- `llm-wiki init` has golden tests for baseline, ML, QMD, combined ML+QMD,
  existing-code, `ml-research`, and `ops-infra` profiles.
- `llm-wiki init` refuses paths containing framework artifacts.
- Initial-source tests assert files are copied into `raw/initial/` and no ingest results appear in `wiki/`.
- The D10 manifest writer records blueprint, resolved packs, and framework
  version in `.llm_wiki/init.toml`.
- The `knowledge-init` canonical is embedded at compile time and projection snapshots lock both runtime variants.
- Post-install integration tests assert installed skills contain the managed binary path and that the managed binary executes under a sanitized `PATH`.
