# Skill Projection on the Composable-Init Template Engine

- Document Class: Proposal
- Status: Proposed
- Date: 2026-05-07
- Category: Tooling, skill projection, internal architecture
- Scope: Once the composable-init proposal lands its compile-time template engine (`askama`), migrate skill projection in `crates/llm-wiki-schema/src/projector/` onto the same engine so the framework has a single text-rendering pipeline.
- Sources: crates/llm-wiki-schema/src/projector/{claude,codex,format,idiom,types}.rs, src/skill_render.rs
- Related: wiki/proposals/blueprint-pack-init.proposal.md, wiki/decisions/llm-wiki-binary-distribution.decision.md, wiki/decisions/binary-path-bootstrap.decision.md
- Depends On: wiki/proposals/blueprint-pack-init.proposal.md (must be accepted and the engine adopted before this proposal lands)

## Question

Once the composable-init proposal introduces a compile-time template engine, should skill projection move onto the same engine?

## Proposal

Yes. The current projector renders per-runtime skill markdown by Rust-side string concatenation of section headers and bodies, with helper functions for runtime-specific transformations. That is rendering — done a different way than the init template will be done. Once the framework has a chosen, working template engine, keeping a second hand-rolled renderer is a smell: it means future skill changes touch Rust, future init changes touch templates, and contributors have to learn both pipelines.

Migrate `crates/llm-wiki-schema/src/projector/{claude,codex}.rs` to render through `askama` templates. The typed `SkillDoc` stays; the projector traits stay; only the rendering tail changes from string concatenation to template invocation.

### Shape

The per-runtime variants are textbook template inheritance, and skill templates share a root with the project-guidelines fragments introduced by the init proposal:

```
templates/
  base/                       # init-proposal: project-guidelines spine
  packs/                      # init-proposal: per-pack fragments
  skills/
    base.md.jinja             # frontmatter, section scaffold, default block bodies
    claude.md.jinja           # extends base, overrides what differs
    codex.md.jinja            # extends base, overrides what differs
    codex_runtime_config.toml.jinja
```

`askama` bind one struct to one template, so each runtime gets a thin newtype wrapper around `SkillDoc`:

```rust
#[derive(Template)]
#[template(path = "skills/claude.md.jinja")]
pub struct ClaudeSkill<'a>(pub &'a SkillDoc);

#[derive(Template)]
#[template(path = "skills/codex.md.jinja")]
pub struct CodexSkill<'a>(pub &'a SkillDoc);
```

Three lines per runtime, no extra "context" types. Templates reference fields directly (`self.0.frontmatter.name`, `self.0.body.purpose`) and call methods on `SkillDoc` for derived values (`{{ self.0.rewritten_invocation(...) }}`). All field references are checked at compile time against the schema's existing types.

Imperative bits that fit awkwardly in templates — `supports_runtime` gating, runtime-specific description shaping, optional-section presence — stay in Rust. `supports_runtime` is a precondition that runs before `.render()` (outside the template either way). `rewrite_invocation` and `description_for` move onto `SkillDoc` as methods so templates can call them. The template never branches on policy; it reads fields and method results.

### What stays the same

1. The canonical `SkillDoc` type, its frontmatter schema, and the parser.
2. The `Projector` trait surface (`project(&self, doc: &SkillDoc) -> Result<RenderedSkill, _>`).
3. Golden-file outputs for every existing skill. The migration is a refactor; output bytes must match (modulo whitespace policy, which the templates encode explicitly).
4. The `BINARY_MARKER` substitution in `src/skill_render.rs`. That logic moves into the context-struct preparation.

### What changes

1. The body of `ClaudeProjector::project` / `CodexProjector::project` becomes "wrap `SkillDoc` in the per-runtime newtype, call `.render()`" instead of "concat strings."
2. `format::render_frontmatter` is replaced by a frontmatter block in the base template.
3. `idiom::rewrite_invocation` and `format::description_for` move from free functions into methods on `SkillDoc` so templates can call them by name.
4. `crates/llm-wiki-schema/Cargo.toml` gains the chosen template-engine dependency. The schema crate's contract expands from "data and parsing" to "data, parsing, and canonical projection to per-runtime markdown." This is deliberate: projecting `SkillDoc` to runtime markdown *is* the schema crate's reason for existing, so making rendering part of its public surface is honest about what the crate does.

## Why

1. **One rendering pipeline.** After this lands, every text the framework produces — `AGENTS.md`, `project_guidelines.md`, every projected skill variant — flows through the same engine. New contributors learn one mental model.
2. **Template inheritance is the right abstraction for per-runtime variants.** The current Rust projectors duplicate section ordering and frontmatter shape between Claude and Codex. A base template with overridable blocks expresses that relationship directly.
3. **Skill changes review better as template diffs.** Adding a new section, reordering, or tweaking the frontmatter shape is one template diff instead of two parallel Rust edits.
4. **Validates the engine choice from the init proposal.** The init migration is the pilot. If `askama` works for both init and skills, it has earned its place. If it fights skills, that is information we want before committing harder.

## Alternatives Considered

1. **Bundle into the init proposal.** Rejected: doubles the surface area of an already-large change, and forfeits the value of having two independent landing points to learn from.
2. **Leave skill projection as Rust string concatenation indefinitely.** Cheap but locks in two rendering pipelines forever. The cost compounds slowly: every contributor learns two patterns, every refactor risks drifting them apart.
3. **A different engine for skills than for init.** Maximum flexibility, zero coherence benefit, double the dependency footprint. Rejected unconditionally.

## Consequences and Tradeoffs

- The migration must produce byte-identical output for every existing skill (the golden files are the contract). This is enforced by the existing snapshot test suite.
- Skill authoring workflow changes: skill changes that previously meant editing Rust now mean editing templates. This is the explicit goal but worth flagging — a contributor who has only seen Rust skill projection will need to learn the template syntax.
- The schema crate (`crates/llm-wiki-schema`) picks up a template-engine dependency and stops being a pure data-and-parsing library. This is accepted, not regretted: the crate's job already includes projecting `SkillDoc` to per-runtime markdown, so rendering is a fair part of its contract. The dependency wraps rendering complexity; it does not represent architectural drift.
- Section order and frontmatter shape become *template authority* rather than Rust authority. Reading the base template should be the canonical answer to "what does a projected skill look like."

## What Closes This Proposal

Promotion to a decision plus an execution plan covering:

1. Template layout under the shared `templates/skills/` directory (sibling to the init proposal's `templates/base/` and `templates/packs/`).
2. `ClaudeSkill<'a>` / `CodexSkill<'a>` newtype wrappers and their `#[derive(Template)]` annotations in the schema crate.
3. Methods on `SkillDoc` that templates call (`rewritten_invocation`, `description_for`, etc.), migrated from `crates/llm-wiki-schema/src/projector/{format,idiom}.rs`.
4. Codex runtime-config rendered through `templates/skills/codex_runtime_config.toml.jinja`, replacing the ad-hoc `with_runtime_config_template(String)` mechanism with a typed template driven by a newtype around `SkillDoc` plus runtime context.
5. Migration order (one runtime first, golden-file diff zero, then the second; runtime-config follows skill markdown).
6. Removal of the now-redundant string-concat projector code paths and the ad-hoc `with_runtime_config_template` API.
7. Documentation updates: any agent-facing notes that describe skill projection as a Rust path get updated to describe templates.

## Open Questions

(None remaining. All earlier open questions are now decided:)

1. **Template-engine dependency location**: in the schema crate, as part of its expanded "data + canonical projection" contract.
2. **Templates root**: a single shared `templates/` root holds both project-guidelines fragments (from the init proposal) and skill templates. One tool, one purpose, one place to look.
3. **Runtime-config emission**: goes through the same template engine. `CodexProjector::with_runtime_config_template` already takes a template string today — formalizing it as a `askama` template (driven by its own newtype wrapper around `SkillDoc` plus runtime context) collapses two ad-hoc template mechanisms into one engine. Structured-data "templates" rendered as text is exactly what the engine is for.
