# Composable Project Init: Blueprints and Packs

- Document Class: Decision
- Status: Accepted
- Date: 2026-05-07
- Category: Tooling, project scaffolding, init UX
- Scope: `llm-wiki init` becomes a composable generator. A chosen blueprint plus a selected set of opt-in packs renders a tailored canonical `AGENTS.md` and `project_guidelines.md` through a compile-time template engine; a per-project `.llm_wiki/` folder records the choices.
- Sources: wiki/proposals/blueprint-pack-init.proposal.md, templates/base/project_guidelines.md, templates/base/agents.md, templates/packs/, src/init/{blueprints,packs,compose,manifest,answers,template,scaffold,command}.rs
- Related: wiki/proposals/blueprint-pack-init.proposal.md, wiki/proposals/skills-template-engine.proposal.md, wiki/specs/documentation-model.spec.md, wiki/specs/knowledge-init-skill.spec.md, wiki/decisions/llm-wiki-binary-distribution.decision.md

## Choice

Replace the static project-guidelines template (gated today by `<!-- SECTION:ML_AI -->` and `<!-- SECTION:QMD -->`) with a composable generator built on a compile-time template engine (`askama`).

Vocabulary is fixed:

1. **Blueprint** — a named project archetype with a default pack selection.
2. **Pack** — a unit of capability the user opts into; contributes folders, doc types, status-vocab additions, and template fragments.
3. **Template** — markdown source files compiled at build time against typed Rust structs.
4. **Render** — resolving a blueprint + pack selection into project files.

Packs and blueprints are Rust enums with accessor methods, not TOML manifests. The pack catalog is a Rust API surface.

`init` is one-shot. A future `upgrade` command is anticipated but out of scope; the per-project `.llm_wiki/init.toml` is written now precisely so that upgrade is buildable later without archaeology.

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
- One-shot init means existing projects do not pick up future pack additions automatically. `.llm_wiki/init.toml` is the breadcrumb that keeps a future `upgrade` command buildable.
- Pack authoring discipline becomes a first-class concern; golden-file tests are the enforcement mechanism.

## What Would Cause This Decision To Be Revisited

- The blueprint catalog stops fitting real project shapes after dogfooding (the third or fourth project can't be expressed as a blueprint + pack combination).
- A second caller for the composition logic appears (e.g. a sibling generator) and justifies extracting `crates/llm-wiki-compose`.
- The compile-time engine choice fights the second adopter (skill projection — see `wiki/proposals/skills-template-engine.proposal.md`) badly enough that runtime templating becomes the lesser evil.
- `askama` becomes inactive or the project hits a bug `rinja` has already fixed; swap by changing the Cargo dependency.
