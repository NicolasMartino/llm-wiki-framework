# Source 003: Askama `Template` derive attributes

- URL: https://askama.rs/en/stable/doc/askama/derive.Template.html
- Retrieved: 2026-05-07
- Mode: web
- Source Type: primary API documentation

## Relevant Facts

- `#[derive(Template)]` generates trait implementations for a struct decorated
  with Askama's `template()` attribute.
- `path = "..."` points to a template file relative to configured template
  directories; default directory is `templates` next to `Cargo.toml`.
- `source = "..."` can embed short template source in an attribute, but then
  `ext` is required and the generated path is undefined, which limits template
  references.
- `ext = "..."` sets content type/extension when using `source`.
- `print = "code"` or related values can print generated code/debug output at
  compile time.
- `block` and `blocks` can render named template blocks or generate
  sub-templates for block fragments.
- `escape = "none"` overrides extension-derived escaping.

## D10 Relevance

Use file-backed `path` templates for all real D10 templates. `source` is useful
only for the Phase 0 throwaway smoke proof. Use explicit `escape = "none"` for
Markdown and TOML outputs. `print = "code"` can be temporarily useful while
debugging derive errors, but should not be committed.

## Suggested D10 Shape

```rust
#[derive(Template)]
#[template(path = "base/project_guidelines.md", escape = "none")]
struct ProjectGuidelinesTemplate<'a> {
    project_name: &'a str,
    project_description: &'a str,
    date: &'a str,
    include_ml_ai: bool,
    include_qmd: bool,
}
```

For pack fragments, prefer separate template structs over `block` extraction
unless multiple fragments truly belong in one file.
