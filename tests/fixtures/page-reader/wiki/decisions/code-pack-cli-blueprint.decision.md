# Code Pack And CLI Tool Blueprint

- Document Class: Decision
- Status: Accepted
- Date: 2026-05-11
- Category: Init UX, project scaffolding, blueprint/pack catalog
- Scope: `llm-wiki init` moves code/deploy folders behind an explicit `code` pack and adds `cli-tool` as a first-class blueprint for command-line products.
- Sources: wiki/proposals/code-folders-opt-in.proposal.md, wiki/decisions/composable-project-init.decision.md, src/init/compose.rs, src/init/blueprints.rs, src/init/packs.rs, templates/base/project_guidelines.md, user discussion 2026-05-11
- Related: wiki/plans/code-pack-cli-blueprint.plan.md, wiki/decisions/composable-project-init.decision.md, wiki/specs/documentation-model.spec.md, wiki/specs/wiki-init-skill.spec.md

## Choice

Accept the code-scaffolding proposal.

`src/`, `tests/`, `scripts/`, and `infra/` are no longer unconditional output
for new projects. They become the contents of a new opt-in `code` pack.

Add `cli-tool` as a new blueprint for command-line tools and developer
utilities with commands, flags, local state, install/release behavior, and
stdout/stderr contracts. Its default pack selection includes `code`.

Software-shaped blueprints default to `code`:

| Blueprint | Code default |
| --- | --- |
| `web-product` | yes |
| `library-sdk` | yes |
| `cli-tool` | yes |
| `ml-research` | yes |
| `ops-infra` | yes |
| `security` | yes |

Knowledge-only blueprints do not default to `code`:

| Blueprint | Code default |
| --- | --- |
| `research` | no |
| `generic` | no |
| `custom` | no |

The user can still add or remove `code` through the interactive pack checklist
or non-interactive `--pack code` flag path.

## Why

The D10 composable-init decision says blueprints are default pack selections
and packs are the unit of optional capability. Unconditional code folders were
a hidden pack: they appeared for new projects even when the selected blueprint
was research-only and the user had no way to decline them.

Making code folders a pack restores the D10 model. It keeps the epistemic spine
universal while making non-wiki project structure explicit and recorded in
`.llm_wiki/init.toml`.

`cli-tool` is accepted because this framework is itself a command-line product.
It is not a web product, not only a library/SDK, and not just ops
infrastructure. The blueprint catalog should expose that shape directly rather
than forcing CLI projects through a nearby but inaccurate archetype.

## Boundaries

The `code` pack contributes folders and small template fragments describing
those folders. It does not add wiki document types, status vocabulary, or
wiki subdirectories.

`cli-tool` is a blueprint, not a pack. CLI is a project archetype; `code` is a
folder bundle.

A separate `cli` pack is deferred. It may become useful if multiple CLI
projects need repeatable documentation conventions such as command-contract
documents, shell-completion tracking, environment-variable inventories,
exit-code tables, install-path records, or stdout/stderr fixture rules. Those
conventions are broader than this folder-scaffolding fix and should not be
smuggled into `Pack::Code`.

Splitting `code` further, such as a separate `infra` pack, is also deferred
until a real project needs that shape.

## Consequences

- The pack catalog grows by one entry: `code`.
- The blueprint catalog grows by one entry: `cli-tool`.
- Existing projects are not modified on disk.
- Existing project manifests do not retroactively gain `code`; a future
  `upgrade` command must treat legacy absence of `code` as "do not delete
  existing code folders."
- New research/docs-only projects stop receiving empty code folders by default.
- Software-shaped projects keep the old practical scaffold through explicit
  default pack selection.
- Golden-file tests become the guardrail for research no-code behavior and
  software/CLI code-folder behavior.

## Implementation Authority

The tactical implementation lives in
`wiki/plans/code-pack-cli-blueprint.plan.md`.
