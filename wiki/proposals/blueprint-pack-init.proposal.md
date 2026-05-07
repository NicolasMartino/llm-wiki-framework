# Composable Project Init: Blueprints and Packs

- Document Class: Proposal
- Status: Accepted
- Date: 2026-05-07
- Promoted To: wiki/decisions/composable-project-init.decision.md, wiki/plans/composable-project-init.plan.md, wiki/roadmaps/framework-v1.roadmap.md (D10)
- Category: Tooling, project scaffolding, init UX
- Scope: Evolve `llm-wiki init` from one static template gated by two conditional flags (`SECTION:ML_AI`, `SECTION:QMD`) into a composable generator that produces a tailored `AGENTS.md` and `project_guidelines.md` from a chosen blueprint plus a set of opt-in packs, and add a per-project `.llm_wiki/` folder for project-specific config and an init manifest.
- Sources: assets/templates/project_guidelines.md, assets/templates/CLAUDE.md, src/init/{profile,answers,template,scaffold,command}.rs
- Related: wiki/specs/documentation-model.spec.md, wiki/specs/knowledge-init-skill.spec.md, wiki/decisions/llm-wiki-binary-distribution.decision.md, wiki/decisions/binary-path-bootstrap.decision.md, wiki/proposals/skills-template-engine.proposal.md

## Question

Should `llm-wiki init` become a composable generator — picking a blueprint and a set of packs and rendering the result — instead of shipping one static template with two conditional sections?

## Proposal

Yes. The framework already aspires to be a "machine that builds the machine": one binary that scaffolds many kinds of projects. Today's template is too generic for any given project (every wiki gets the same skeleton) and too rigid for cross-domain use (only ML/AI and QMD scale are toggleable). Replace it with a small composition system.

### Vocabulary

Four words, used consistently:

1. **Blueprint** — a named project archetype (e.g. `web-product`, `ml-research`, `library-sdk`, `ops-infra`, `security`, `generic`, `custom`). A blueprint is a *shortcut to a default pack selection*; nothing more. It carries no behavior the user cannot reproduce by ticking packs by hand.
2. **Pack** — a unit of capability the user opts into. A pack contributes some subset of: doc types, folder additions, status-vocabulary entries, operations, init questions, and content fragments for `AGENTS.md` / `project_guidelines.md`. Packs are defined in Rust and shipped with the binary.
3. **Template** — the markdown source files under `templates/` that the chosen template engine compiles into typed Rust render functions. Templates are checked at build time against the structs that drive them.
4. **Render** — the act of resolving a blueprint + pack selection into concrete files written into the project, by invoking the compiled templates.

The composer is the binary. There is no "module," "addon," or "plugin" vocabulary — those terms are reserved for future extensibility work and not used inside this proposal.

### Init Flow

Two-step interactive flow:

1. **Step 1 — Blueprint.** The user picks one blueprint from a short list, or `custom`.
2. **Step 2 — Packs.** A checklist of every available pack is shown. If a blueprint was chosen, its packs are pre-ticked; if `custom`, all are unticked. The user can tick or untick anything before confirming.

The non-interactive path (`--non-interactive`, used by `knowledge-init`) accepts `--blueprint <name>` and `--pack <name>` flags, with the same defaulting rules.

### Spine vs. Packs

Every project, regardless of blueprint or pack selection, gets the **epistemic spine**: `specs/`, `decisions/`, `proposals/`, `index.md`, `log.md`, plus the ingest/query/lint operations and the metadata block. The spine is non-negotiable and lives in the base template. Packs only *add* on top of it.

This guarantees that every wiki produced by the framework is recognizable and lintable by the same skills, regardless of which packs were chosen.

### Per-project `.llm_wiki/`

`init` writes a `.llm_wiki/` folder into the new project alongside the wiki, mirroring the global runtime home. It contains:

1. `init.toml` — records the chosen blueprint, the resolved pack list, and the framework binary version that ran the init. This is the breadcrumb a future `upgrade` command needs.
2. Optional project-specific overrides (e.g. custom status vocabulary, additional doc types, project-local skill config) layered on top of the global config.

Today, only `init.toml` is written. The folder is established now so future overrides have a home, but no override mechanism is in scope for this proposal.

### Blueprint Catalog (initial)

Eight entries (seven named blueprints plus `custom`). Each blueprint is a name, a one-line description, and a default pack selection — nothing else.

| Blueprint | One-line description | Default packs |
| --- | --- | --- |
| `generic` | Spine-only knowledge base with no domain assumptions. Default when unsure. | (none) |
| `web-product` | Frontend, backend, or full-stack product with users and a release cadence. | `api`, `frontend`, `ops-lite` |
| `library-sdk` | Public API surface with semver discipline and external consumers. | `api`, `library` |
| `ml-research` | Pre-production model work: experiments, datasets, evaluation. | `ml`, `data`, `research` |
| `ops-infra` | Infrastructure or platform work without a product surface: runbooks, on-call, postmortems. | `ops` |
| `security` | Security review, threat modeling, audit, or finding tracking. | `security`, `ops-lite` |
| `research` | Non-ML research project: literature notes, hypotheses, citation graph. | `research`, `qmd-scale` |
| `custom` | Empty default; user ticks packs by hand. | (none) |

A productionized ML system (sometimes called "ml-product") is intentionally not a separate blueprint: it is `ml-research` plus the `ops` pack ticked in step 2. If that combination becomes the most common ML use case in practice, it can be promoted to its own blueprint later.

### Pack Catalog (initial)

| Pack | Adds |
| --- | --- |
| `api` | API spec doc type, stability annotations, deprecation tracking, OpenAPI/schema source convention |
| `frontend` | Design-doc convention, component inventory, screenshot conventions for raw/ |
| `library` | Changelog discipline, public-surface tracking, compatibility notes, examples folder |
| `ml` | `experiments/`, `evals/`, model-card and dataset-card doc types, eval status vocab (`Planned`, `Baseline`, `Candidate`, `Accepted`, `Rejected`, `Superseded`), model lineage notes |
| `data` | `data/` folder, dataset schema docs, lineage tracking, transformation specs |
| `ops` | `runbooks/`, `postmortems/`, SLO doc type, on-call rotation, incident archive conventions |
| `ops-lite` | Smaller `ops`: runbooks only, no SLO/on-call/postmortem machinery |
| `security` | `threat-models/`, findings doc type, audit notes, controls register |
| `research` | Literature-note doc type, hypothesis doc, citation conventions, lab-notebook style |
| `qmd-scale` | The QMD search section currently gated behind `<!-- SECTION:QMD -->` |

The pack list is the actual extensibility surface. Adding a project shape over time is "add a pack" or "add a blueprint that picks an existing combination," not "add another flag to a monolithic template."

### Generator Implementation

The current dependency set is intentionally lean (`anyhow`, `chrono`, `clap`, `serde`/`serde_json`/`serde_yaml`, `sha2`). Two crate additions plus one already-present crate (`toml` via cargo features, used only for the runtime breadcrumb file) cover composable init:

1. **`askama`** for compile-time template rendering. The template files live under `templates/` and are compiled into typed Rust render functions; field references that don't exist on the driving struct are *build errors*, not runtime surprises. This same engine takes over the existing init template (replacing the ad-hoc `{{PROJECT_NAME}}`/`{{DATE}}` substitution in `src/init/template.rs`) so we have one rendering path, not two. `rinja` (the actively maintained fork) is functionally equivalent and is a drop-in fallback if `askama` ever stalls — same template syntax, swap is a Cargo dependency change. `minijinja`/`tera`/`handlebars` were considered and rejected: the runtime-loading they enable buys nothing here (every template ships with the binary) and forfeits the compile-time check.
2. **`inquire`** for interactive prompts. Step 1 is a `Select` over blueprints; step 2 is a `MultiSelect` over packs with the blueprint's default packs pre-checked — `inquire`'s `MultiSelect::with_default` does exactly this. `dialoguer` works too but its multi-select API is clunkier. The non-interactive path bypasses the crate entirely and reads from `clap`-parsed flags.
3. **`toml`** for serializing `.llm_wiki/init.toml` only. There is no pack-manifest TOML or blueprint TOML — packs and blueprints are Rust definitions (see below). `toml` shows up purely to write and later read the runtime breadcrumb that records the user's choices.

Packs and blueprints are expressed as Rust:

```rust
pub enum Pack { Api, Frontend, Library, Ml, Data, Ops, OpsLite, Security, Research, QmdScale }

impl Pack {
    pub fn name(&self) -> &'static str { ... }
    pub fn folders(&self) -> &'static [&'static str] { ... }
    pub fn doc_types(&self) -> &'static [DocType] { ... }
    pub fn status_vocab(&self) -> &'static [StatusEntry] { ... }
    pub fn agents_fragment(&self) -> Option<&'static str> { ... }      // path key into compiled templates
    pub fn guidelines_fragment(&self) -> Option<&'static str> { ... }
}

pub enum Blueprint { Generic, WebProduct, LibrarySdk, MlResearch, OpsInfra, Security, Research, Custom }

impl Blueprint {
    pub fn default_packs(&self) -> &'static [Pack] { ... }
}
```

This means: adding a pack is "add an enum variant + implement its accessors + add its template fragment file." No TOML schema, no key-typo failure mode, IDE autocomplete, exhaustive `match` enforcement. The pack catalog is a Rust API surface, not a configuration format.

The composition logic itself stays in `src/init/`, not a new workspace crate. Pack composition and render orchestration are project-internal; if the same shape ever finds a second user (a sibling project generator), that is the moment to extract a `crates/llm-wiki-compose` crate.

A separate, larger refactor — moving `src/skill_render.rs` onto the same template engine so the framework has exactly one text-rendering pipeline — is **out of scope** for this proposal but flagged here so the engine choice anticipates it. `askama` handles both cases.

### Asset Layout

```
templates/
  base/                     The spine (always rendered)
    agents.md.jinja
    project_guidelines.md.jinja
  packs/                    Per-pack template fragments
    api/
      agents.md.jinja
      project_guidelines.md.jinja
    frontend/
      ...
    library/
    ml/
    data/
    ops/
    ops-lite/
    security/
    research/
    qmd-scale/
```

Each pack directory holds only the template fragments it contributes. There is no `pack.toml`, no blueprint TOML, no manifest. The pack's *metadata* (name, folders, doc types, status vocab, default-blueprint membership, which fragments it owns) lives in Rust, alongside the `Pack` enum. The pack's *content* lives in the template files. The two are tied together by compile-time references: the Rust accessor returns a path key, the engine resolves that key to a compiled template.

Blueprints have no on-disk presence — they are pure Rust enums whose only behavior is `default_packs()`.

### One-shot, with upgrade as a path

`init` is one-shot: it generates files and exits. There is no `upgrade` command in this proposal. But the manifest at `.llm_wiki/init.toml` is written precisely so that a future `upgrade` command is buildable without archaeology. We are paying a tiny cost now to keep that door open.

## Why

1. **The current template is dishonest about its variability.** It claims to be a "documentation and execution model" for any project, but every project gets the same nine-section skeleton plus two flags. The proposal makes the actual range of supported shapes explicit.
2. **The `src/init/` skeleton already supports this.** `profile.rs`, `answers.rs`, `template.rs`, `scaffold.rs` exist. Pushing them from "render one template with flags" to "compose packs into a render plan" is an evolution of the existing structure, not a rewrite.
3. **Self-replication gets sharper.** D7's proof — bootstrap two distinct projects with a single binary — is more meaningful when the two projects differ in shape (e.g. an ML-research wiki and an ops-infra wiki) rather than being two instances of the same template.
4. **The `.llm_wiki/` folder is needed regardless.** Even without composable init, projects need a place for project-specific config and an init manifest. Establishing it now avoids retrofitting later.

## Alternatives Considered

1. **Keep the static template, add more conditional sections.** Cheap, but each new project type adds a flag and the template gets harder to read. Linear cost, sub-linear value.
2. **Pure à-la-carte packs, no blueprints.** Flexible but high-friction — every new user has to know the full pack catalog before they can init. Blueprints are a UX shortcut, not a separate mechanism.
3. **Per-archetype templates with no shared spine.** Maximum tailoring, but breaks the universal lint/ingest/query story. Rejected: the spine must stay non-negotiable.
4. **Defer until D9 or later.** Possible, but the static template is already actively misleading new users about what the framework can do, and the `.llm_wiki/` folder lands cleanest before the next round of skill changes.

## Consequences and Tradeoffs

- The static `assets/templates/project_guidelines.md` is retired in favor of a base spine plus pack fragments rendered through `askama`. The existing init template moves onto the same engine in the same change set so the codebase ends up with one rendering path, not two.
- Pack authoring discipline is now a first-class concern: a pack must be coherent on its own and compose cleanly with other packs. Golden-file tests are the enforcement mechanism, the same way they are for skill projection.
- Adding a pack or blueprint requires a framework release (recompile and ship the binary). This is the explicit cost of going compile-time and is acceptable: every other framework asset (skills, base template) already ships embedded in the binary.
- The blueprint list becomes a user-facing surface area we will need to evolve carefully. Adding a blueprint is cheap (an enum variant); removing or renaming one is a breaking change for anyone whose `init.toml` references it.
- One-shot init means today's projects do not benefit from future pack additions. This is acceptable for now and is the explicit motivation for keeping `.llm_wiki/init.toml`.

## What Closes This Proposal

Promotion to a decision plus a roadmap entry (likely D9) and an execution plan covering:

1. Asset layout under `templates/{base,packs/<name>}/`.
2. `Pack` and `Blueprint` enums plus their accessor traits in `src/init/`.
3. Migration of the existing init template onto `askama` in the same change set.
4. Composition logic in `src/init/` (default-pack resolution, fragment ordering, idempotent folder creation).
5. Two-step interactive flow with `inquire`, plus the non-interactive `--blueprint` / `--pack` flag mapping.
6. Initial blueprint and pack catalog.
7. `.llm_wiki/init.toml` schema and writer.
8. Golden-file tests for at least two contrasting blueprints (e.g. `ml-research` vs. `ops-infra`).

## Open Questions

1. Whether `custom` should let the user *define* a new pack inline, or only select existing ones. Lean toward the latter for now (defining a pack means writing Rust, which is a framework-release activity).
2. Whether project-local pack overrides under `.llm_wiki/` are in scope for the first cut. Lean no.
3. Whether the catalog above survives contact with the first two real bootstrapped projects. Treat the catalog as first-cut; the plan revises after dogfooding.

(The earlier `rinja` vs. `askama` open question is decided: `askama`, on ecosystem size and prior in-house experience. `rinja` remains a drop-in fallback if `askama` stalls.)

(The earlier open question about pack-conflict rules — two packs contributing the same doc type or folder — is resolved by the compile-time pack model: overlaps are visible in the Rust definitions and either deduplicated explicitly or rejected by a `match` on a shared enum.)
