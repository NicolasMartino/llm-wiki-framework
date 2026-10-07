# Composable Project Init: Blueprints and Packs

- Document Class: Decision
- Status: Accepted
- Date: 2026-05-07
- Category: Tooling, project scaffolding, init UX
- Scope: `llm-wiki init` becomes a composable generator. A chosen blueprint plus a selected set of opt-in packs renders a tailored canonical `AGENTS.md` and `project_guidelines.md` through a compile-time template engine; a per-project `.llm_wiki/` folder records the choices.
- Sources: wiki/proposals/blueprint-pack-init.proposal.md, templates/base/project_guidelines.md, templates/base/agents.md, templates/packs/, src/init/{blueprints,packs,compose,manifest,answers,template,scaffold,command,managed_block,collision}.rs, wiki/plans/operations-setup-in-llm-wiki.plan.md (phase 1), the owner's decision of 2026-10-06 on the block init owns, issue #53
- Related: wiki/proposals/blueprint-pack-init.proposal.md, wiki/proposals/skills-template-engine.proposal.md, wiki/specs/documentation-model.spec.md, wiki/specs/wiki-init-skill.spec.md, wiki/decisions/llm-wiki-binary-distribution.decision.md

## Choice

Replace the static project-guidelines template with a composable generator
built on a compile-time template engine (`askama`).

Vocabulary is fixed:

1. **Blueprint** — a named project archetype with a default pack selection.
2. **Pack** — a unit of capability the user opts into; contributes folders, doc types, status-vocab additions, and template fragments.
3. **Template** — markdown source files compiled at build time against typed Rust structs.
4. **Render** — resolving a blueprint + pack selection into project files.

Packs and blueprints are Rust enums with accessor methods, not TOML manifests. The pack catalog is a Rust API surface.

Fresh `init` is a scaffold operation. Rerunning `init` on a project with
`.llm_wiki/init.toml` is an edit operation over the recorded setup answers: the
interactive flow preselects current values, can create newly selected pack
folders, refreshes only the marked block init owns in each root schema file
(`AGENTS.md`, `CLAUDE.md`, `project_guidelines.md`) and leaves the text outside
it to the project, and preserves live wiki bookkeeping (`wiki/index.md` and
`wiki/log.md`). A future `upgrade` command is
still anticipated for richer migrations; the per-project `.llm_wiki/init.toml`
is the durable answer record that keeps that future command buildable without
archaeology.

Generated projects use `AGENTS.md` as the canonical agent schema file. `init`
also writes a tiny `CLAUDE.md` compatibility shim pointing at `AGENTS.md`, so
Claude-oriented tooling still has an entry point while the framework has one
canonical schema document to render and compose.

Template fragments are compile-time Askama templates selected by exhaustive
Rust matches. Packs do not return arbitrary runtime template paths.

## Why

1. The current template claims to be "the documentation and execution model" for any project but every project gets the same nine-section skeleton with two flags. Composable init makes the actual range of supported shapes explicit.
2. Compile-time templates turn rendering errors into build errors and remove the runtime template-loading step. Every template ships with the binary, so runtime loading buys nothing.
3. The `src/init/` skeleton (`profile.rs`, `answers.rs`, `template.rs`, `scaffold.rs`) already supports this shape — the change is an evolution, not a rewrite.
4. Self-replication (D7) gets sharper proof: bootstrapping two genuinely different projects (e.g. an ML-research wiki and an ops-infra wiki) means more than bootstrapping two instances of the same template.
5. A per-project `.llm_wiki/` folder is needed regardless — it is the right home for project-specific config and the init manifest.

## Alternatives Considered

### Static template plus more conditional sections

Cheap, but each new project type adds a flag and the template gets harder to read. Linear cost, sub-linear value.

### Pure à-la-carte packs, no blueprints

Flexible but high-friction — every new user has to know the full pack catalog before they can init. Blueprints are a UX shortcut, not a separate mechanism.

### Per-archetype templates with no shared spine

Maximum tailoring but breaks the universal lint/ingest/query contract. The epistemic spine (specs, decisions, proposals, index, log) must be non-negotiable.

### Runtime template engine (`minijinja`, `tera`, `handlebars`)

Rejected: every template ships with the binary, so runtime loading offers no benefit and forfeits the compile-time check.

### `rinja` (the actively maintained askama fork)

Considered. `rinja` and `askama` are functionally equivalent for this use case. Choosing `askama` over `rinja` because it has the larger ecosystem and prior in-house experience. The decision is reversible if `askama` proves under-maintained — both crates accept the same template syntax, so swapping is a Cargo-dependency change and a re-test, not a rewrite.

### TOML pack manifests

Rejected: packs ship with the binary. Encoding metadata as Rust enums plus accessor methods is type-checked, IDE-completable, and avoids the key-typo failure mode TOML schemas invite.

## Consequences and Tradeoffs

- The static `assets/templates/project_guidelines.md` is retired. The existing init template migrates onto the same engine in the same change set so the codebase has one rendering path.
- Adding a pack or blueprint requires a framework release. This is the explicit cost of going compile-time and is acceptable: every other framework asset (skills, base template) already ships embedded.
- The blueprint list becomes a user-facing surface. Adding a blueprint is cheap; removing or renaming one is a breaking change for any project whose `init.toml` references it.
- Rerun init gives existing projects an explicit way to review current
  blueprint/pack answers and opt into newly added packs or fields, but it is
  still not a full migration engine. `.llm_wiki/init.toml` remains the
  breadcrumb that keeps a future `upgrade` command buildable.
- Pack authoring discipline becomes a first-class concern; golden-file tests are the enforcement mechanism.

## Dogfooding Revision

On 2026-05-11, the first post-D10 catalog revision was accepted in
`wiki/decisions/code-pack-cli-blueprint.decision.md`: root code/deploy folders
move behind an explicit `code` pack, and `cli-tool` becomes a blueprint for
command-line products. This does not change the D10 composition model; it
applies it more strictly by removing hidden code scaffolding from the base
template.

On 2026-05-13, rerun init was accepted as a narrow edit surface for project
setup answers. The manifest now records project name and description in
addition to blueprint, resolved packs, and framework version. Interactive
reruns prefill from the manifest, while older manifests fall back to generated
project files when name or description can be recovered. Rerun writes avoid
clobbering existing `wiki/index.md` and `wiki/log.md`. When a rerun changes
blueprint, the pack prompt follows the new blueprint defaults instead of
pinning the old pack set. Auto-registration updates the existing same-root
registry entry and keeps the project id stable.

On 2026-05-14, rerun init gained a schema-drift audit without becoming a full
migration engine. New manifests record resolved folders; reruns compare the
previous manifest's pack set and resolved folder composition against the
current composition before overwrite. When drift exists, init preserves orphan
content on disk, appends structured evidence to `wiki/log.md`, and refreshes a
minimal generated `## Schema Drift` section in `wiki/index.md` while preserving
existing catalog entries.

On 2026-10-07, the rerun rule narrowed to a block init owns in each root
schema file, as the owner decided on 2026-10-06 ("a needle, a part of the
agents.md file that is dedicated to llm wiki", for `AGENTS.md`, `CLAUDE.md` and
`project_guidelines.md`) and phase 1 of
`wiki/plans/operations-setup-in-llm-wiki.plan.md` built:

- Each file holds one block between `<!-- llm-wiki:managed:start -->` and
  `<!-- llm-wiki:managed:end -->`; everything init renders for the file,
  pack fragments included, goes inside it, after a first line saying so. A
  rerun replaces the text between the markers and leaves every byte before
  and after them as it was.
- Init reads the three files, checks their markers and renders each block
  before it writes anything; broken markers (a begin without an end, an end
  before a begin, two blocks) refuse the run, naming the file and the line,
  and leave the project as it was.
- `.llm_wiki/init.toml` records the SHA-256 of each block under
  `[managed_blocks]` (`agents`, `claude`, `guidelines`); a manifest without
  the table still reads. A block that no longer matches its hash was edited:
  init saves it under `.llm_wiki/saved-blocks/`, named by the date and time
  with a counter when taken and never overwritten, replaces it, and warns
  with the edited lines the new block does not hold. A block that matches is
  refreshed without a word, even when the templates changed.
- A file without markers is migrated once: if it equals the render from the
  previously recorded answers (the guidelines' `- Date:` line left out on
  both sides) it becomes the block alone; otherwise init writes the block at
  the top, then a dated "Kept From Before The llm-wiki Block" heading, then
  the old file unchanged, and warns naming the file.
- Init writes into the AGENTS file that exists, `AGENTS.md` or `AGENTS.MD`,
  and `CLAUDE.md`'s block names it. The fresh-init collision guard also
  refuses a folder holding `AGENTS.MD`.

## What Would Cause This Decision To Be Revisited

- The blueprint catalog stops fitting real project shapes after dogfooding (the third or fourth project can't be expressed as a blueprint + pack combination).
- A second caller for the composition logic appears (e.g. a sibling generator) and justifies extracting `crates/llm-wiki-compose`.
- The compile-time engine choice fights the second adopter (skill projection — see `wiki/proposals/skills-template-engine.proposal.md`) badly enough that runtime templating becomes the lesser evil.
- `askama` becomes inactive or the project hits a bug `rinja` has already fixed; swap by changing the Cargo dependency.
