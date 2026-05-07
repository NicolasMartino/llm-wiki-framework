# Research Summary: Askama Rust for D10 Composable Init

- Date: 2026-05-07
- Question: How should Askama be used to achieve the D10 composable init plan?
- Scope: Askama dependency/version, template layout, Markdown escaping,
  template context design, fragment composition, whitespace control, and
  implementation sequencing.

## Key Findings

Askama is a good fit for D10's core requirement: template code is compiled into
the crate from a Rust context type, which turns missing fields and invalid
syntax into compile-time failures. The current docs.rs package page lists
Askama 0.16.0 as the latest release, published 2026-04-29. Use `askama =
"0.16"` unless local build or lockfile constraints reveal a reason to pin
0.15.x.

The D10 cleanup decision to use natural `.md` filenames is supported by the
stable configuration docs: Askama's default plain-text escapers include `md`,
`yml`, `none`, `txt`, and the empty extension. Still, using
`#[template(path = "...", escape = "none")]` is a good explicit contract for
generated Markdown and TOML, especially because the D10 plan wants stable bytes
and future contributors should not need to remember extension inference rules.

Templates are found relative to configured template directories; by default
that is `templates` next to the package `Cargo.toml`. For the root binary crate,
D10's `templates/base/...` and `templates/packs/...` layout should work without
`askama.toml`. If skill projection later moves into `crates/llm-wiki-schema`,
that crate will need either its own `templates/` directory or an explicit
`askama.toml` strategy, because Askama reads config from the crate root.

For Phase 1, define typed context structs and derive templates directly:

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

For Phase 3 and pack composition, prefer pre-rendered fragment strings passed
into the base template context over dynamic Askama inheritance. Askama supports
block fragments and render-in-place composition, but D10's pack set is runtime
selected by CLI answers. The compile-time-safe pattern is: render each selected
pack through an exhaustive Rust `match`, sort/deduplicate in Rust, then pass
`Vec<String>` or `&[String]` into the base template and loop over it.

Example base insertion point:

```askama
{% for fragment in agents_fragments %}
{{ fragment }}
{% endfor %}
```

If the template uses `.md` plus `escape = "none"`, the fragment output will not
be HTML-escaped. Keep fragments trusted because they ship with the binary.

Askama's whitespace default is preserve. That is the safest starting point for
snapshot migration because the current renderer preserves most whitespace and
then compacts blank lines. Use localized `{%- ... -%}` only where snapshots show
extra blank lines. Avoid global `whitespace = "suppress"` at first; it may
silently alter markdown spacing across all templates.

## D10 Implementation Guidance

1. Add `askama = "0.16"` to workspace dependencies and root dependencies.
2. Run `cargo build` immediately after adding the dependency to catch version or
   MSRV issues. Local toolchain is `rustc 1.92.0`, so current local development
   should not be blocked by a modern Askama MSRV.
3. Move templates to `templates/base/project_guidelines.md` and
   `templates/base/agents.md`.
4. Convert placeholders to Askama syntax:
   `{{ project_name }}`, `{{ project_description }}`, `{{ date }}`.
5. Convert conditional sections to `{% if include_ml_ai %}` and
   `{% if include_qmd %}`.
6. Preserve Phase 1's compatibility bridge by keeping `ProjectProfile` booleans
   until the pack composer replaces them.
7. Introduce a narrow `render_*` wrapper returning `anyhow::Result<String>`;
   Askama render errors should not be unwrapped in production code.
8. Add snapshot coverage for `AGENTS.md` separately from `CLAUDE.md` because
   D10 intentionally changes the canonical schema file.
9. For pack fragments, define one `Template` struct per fragment file, even if
   the first versions have empty context. Select fragments by exhaustive
   `match Pack` arms to keep compiler enforcement when new packs are added.
10. Defer Askama inheritance unless a second, static inheritance use case
    appears. Runtime-selected packs are better modeled as rendered fragments
    passed through context.

## Caveats

- Official docs and docs.rs package metadata can lag or disagree on example
  dependency versions. Use current package metadata for the version and stable
  docs for API behavior, then let local `cargo build` be the source of truth.
- Askama can render nested templates by placing template values in context, but
  this is less convenient for a heterogeneous runtime-selected list of packs.
  `Vec<String>` fragment output is simpler and matches D10's accepted design.
- Avoid custom syntax and custom escapers unless necessary. Defaults are enough
  for Markdown and keep the migration smaller.

## Open Questions For Implementation

- Should D10 keep a tiny `compact_blank_lines` post-process for Phase 1
  byte-stability, or should whitespace be encoded entirely in templates? Start
  with the existing compaction behavior until snapshots are stable.
- Should generated fragment strings include their own leading/trailing
  newlines, or should the base template be responsible? Pick one convention
  before writing the first pack to avoid snapshot churn.
- Should `Pack::agents_fragment()` return `anyhow::Result<Option<String>>`
  instead of `Option<String>` so template render failures propagate cleanly?
  The research suggests yes.

## Ready For Ingest

This bundle is ready to ingest as a reference page supporting D10
implementation. It should cite the Askama package metadata, stable docs, and
the local D10 plan/decision.
