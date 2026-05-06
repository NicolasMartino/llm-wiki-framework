# Knowledge Init Skill

- Document Class: Spec
- Status: Active
- Date: 2026-05-06
- Category: Tooling
- Scope: The `knowledge-init` agent skill as a thin conversational wrapper over the `llm-wiki init` binary command.
- Sources: assets/skills/knowledge-init/SKILL.md, src/init/mod.rs, wiki/decisions/llm-wiki-binary-distribution.decision.md, wiki/decisions/binary-path-bootstrap.decision.md
- Related: wiki/specs/documentation-model.spec.md, wiki/specs/knowledge-ingest-skill.spec.md

## Contract

`knowledge-init` collects setup answers and delegates deterministic project
scaffolding to the binary:

```bash
llm-wiki init <path> --non-interactive --name <name> --description <description> --type <type> --scale <scale>
```

The agent owns conversation, validation, and the optional handoff to ingest.
The binary owns filesystem writes, profile resolution, template rendering,
framework-artifact collision checks, and initial-source copying.

## Required Answers

The wrapper gathers the binary's flag set:

1. target path
2. project name
3. one-sentence description
4. project type: `web`, `api`, `cli`, `ml`, `data`, `lib`, or `other`
5. scale: `small`, `medium`, or `large`
6. whether the target is an existing codebase
7. optional initial source paths

`--existing` is passed when adding the framework to existing code.
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

## Proven By

- `llm-wiki init` has golden tests for baseline, ML/AI, QMD, combined ML/AI+QMD, and existing-code profiles.
- `llm-wiki init` refuses paths containing framework artifacts.
- Initial-source tests assert files are copied into `raw/initial/` and no ingest results appear in `wiki/`.
- The `knowledge-init` canonical is embedded at compile time and projection snapshots lock both runtime variants.
- Post-install integration tests assert installed skills contain the managed binary path and that the managed binary executes under a sanitized `PATH`.
