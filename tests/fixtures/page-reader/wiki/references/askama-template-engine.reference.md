# Askama Template Engine for D10 Composable Init

- Document Class: Reference
- Status: Sourced
- Date: 2026-05-07
- Category: Rust template engine, D10 implementation
- Scope: Askama usage guidance for implementing D10 composable project init with compile-time Markdown templates and runtime-selected packs.
- Sources: raw/research/2026-05-07-askama-rust-d10-composable-init/manifest.md, raw/research/2026-05-07-askama-rust-d10-composable-init/research-summary.md, raw/research/2026-05-07-askama-rust-d10-composable-init/sources/001-askama-crate-latest.md, raw/research/2026-05-07-askama-rust-d10-composable-init/sources/002-askama-introduction.md, raw/research/2026-05-07-askama-rust-d10-composable-init/sources/003-askama-template-derive.md, raw/research/2026-05-07-askama-rust-d10-composable-init/sources/004-askama-configuration.md, raw/research/2026-05-07-askama-rust-d10-composable-init/sources/005-askama-template-syntax.md, raw/research/2026-05-07-askama-rust-d10-composable-init/sources/006-d10-local-context.md
- Related: wiki/plans/composable-project-init.plan.md, wiki/decisions/composable-project-init.decision.md, wiki/proposals/skills-template-engine.proposal.md

## Source Set

The Askama research bundle reviewed primary Askama package and documentation
sources plus local D10 context:

1. docs.rs package metadata for Askama.
2. Askama stable introduction and quick start.
3. Askama `Template` derive attributes.
4. Askama configuration, template directories, whitespace, and escapers.
5. Askama template syntax for loops, conditionals, block fragments, macros, and
   render-in-place composition.
6. The local D10 plan and accepted composable-init decision.

## Facts

As of 2026-05-07, docs.rs lists Askama `0.16.0` as the current package page,
published 2026-04-29. The implementation should start with `askama = "0.16"`
and verify through local Cargo resolution and build.

Askama compiles templates into Rust code from typed context structs. The core
D10 value is that invalid template syntax and references to missing context
fields fail at compile time instead of during `llm-wiki init`.

Askama's default template directory is `templates` relative to the crate root.
For the root binary crate, D10's planned `templates/base/` and
`templates/packs/<name>/` layout can work without `askama.toml`.

Askama's default plain-text escapers include Markdown (`md`) and TOML-adjacent
plain-text extensions. Even so, D10 should keep explicit
`#[template(..., escape = "none")]` attributes for generated Markdown and TOML
so the no-HTML-escaping contract is visible in Rust.

Askama's default whitespace behavior is preserve. This matches D10's
snapshot-first migration better than global suppression. Whitespace should be
controlled locally with Askama's whitespace markers only where snapshots show
drift.

## D10 Implementation Guidance

Phase 0 should add Askama as:

```toml
askama = "0.16"
```

Then run `cargo build` immediately to validate dependency resolution and macro
setup. The local research environment used `rustc 1.92.0` and `cargo 1.92.0`,
so local development is not expected to be blocked by a modern Askama MSRV.

Phase 1 should define narrow typed templates:

```rust
use askama::Template;

#[derive(Template)]
#[template(path = "base/project_guidelines.md", escape = "none")]
struct ProjectGuidelinesTemplate<'a> {
    project_name: &'a str,
    project_description: &'a str,
    date: &'a str,
    include_ml_ai: bool,
    include_qmd: bool,
}

#[derive(Template)]
#[template(path = "base/agents.md", escape = "none")]
struct AgentsTemplate<'a> {
    project_name: &'a str,
    project_description: &'a str,
    ml_ai_types: &'a str,
}
```

The Phase 1 compatibility bridge can keep `ProjectProfile` booleans until the
pack composer replaces them. Render wrappers should return
`anyhow::Result<String>` rather than unwrap Askama render errors.

Pack fragments should be rendered through exhaustive Rust matches and passed
to the base templates as already-rendered strings. This fits D10 because pack
selection is runtime data from CLI answers, while template definitions remain
compile-time assets.

Example base insertion point:

```askama
{% for fragment in agents_fragments %}
{{ fragment }}
{% endfor %}
```

This approach is preferable to Askama inheritance for D10 pack composition.
Inheritance and block fragments are useful for static template relationships,
but selected packs are dynamic.

`Pack` fragment accessors should return a fallible type, for example:

```rust
fn agents_fragment(&self) -> anyhow::Result<Option<String>>;
fn guidelines_fragment(&self) -> anyhow::Result<Option<String>>;
```

This keeps template render failures visible to `init` instead of forcing
unwraps or hiding failures behind `Option`.

## Skill Projection Caveat

The follow-on skill-projection proposal plans to use the same template engine
from `crates/llm-wiki-schema`. Askama resolves template paths relative to the
crate doing the derive unless configured otherwise. That means the follow-on
must explicitly decide whether skill templates live under the schema crate,
whether `askama.toml` points at a shared root, or whether derives stay in the
root binary crate. D10 init itself is not blocked by this because its templates
belong to the root binary crate.

## Open Implementation Questions

1. Should Phase 1 keep the current blank-line compaction post-process for
   byte-stability, then remove it once template whitespace is encoded? The
   research leans yes.
2. Should pack fragments own leading/trailing newlines, or should base
   templates own all separators? Pick one convention before writing the first
   pack to avoid snapshot churn.
3. Should D10 add an `askama.toml` at all? The research leans no for init; the
   default root `templates/` directory is sufficient.
