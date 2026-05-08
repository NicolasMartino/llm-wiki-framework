# Source 002: Askama introduction and quick start

- URL: https://askama.rs/en/stable/
- Retrieved: 2026-05-07
- Mode: web
- Source Type: primary documentation

## Relevant Facts

- Askama is based on Jinja-like templates and generates Rust code at compile
  time from a user-defined context struct.
- Feature highlights include Rust type-system safety, compiled template code,
  UTF-8 templates/output, stable Rust support, template inheritance, loops,
  if/else, include support, macros, filters, whitespace suppression markers,
  opt-out HTML escaping, and syntax customization.
- Quick start pattern:
  1. add Askama to Cargo dependencies
  2. create `templates/` in the crate root
  3. create a template file
  4. derive `Template` on a Rust struct with fields matching template variables
  5. call `.render()`

## D10 Relevance

D10's `templates/base/...` layout matches Askama's default crate-root
`templates/` convention for the root binary crate. Context structs should be
small and typed: one for project guidelines, one for agent schema, then one per
pack fragment where needed.

## Implementation Note

Bring `askama::Template` into scope in `src/init/template.rs` or a sibling
module, derive template structs there, and keep `scaffold.rs` unaware of the
template engine beyond calling render functions.
