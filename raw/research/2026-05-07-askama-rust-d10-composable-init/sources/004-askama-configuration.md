# Source 004: Askama configuration, dirs, whitespace, escapers

- URL: https://askama.rs/en/stable/configuration.html
- Retrieved: 2026-05-07
- Mode: web
- Source Type: primary documentation

## Relevant Facts

- Askama reads optional `askama.toml` from the crate root when the default
  `config` feature is enabled.
- Default template search dirs are `["templates"]`, relative to the crate root.
- `dirs` supports glob syntax such as `templates/*` and `templates/**`.
- Default whitespace mode is `preserve`.
- Whitespace can be controlled locally with `-`, `+`, and `~` markers or
  globally with `whitespace = "suppress"` / `minimize`.
- Template derive attributes can override whitespace mode for a specific
  template.
- Default plain-text escapers include `md`, `yml`, `none`, `txt`, and the empty
  string. HTML escaping applies to extensions such as `html`, `htm`, `xml`,
  `j2`, `jinja`, and `jinja2`.

## D10 Relevance

D10 can avoid `askama.toml` initially because the root crate can place files
under root `templates/`. `.md` templates naturally use plain-text escaping, but
the plan's explicit `escape = "none"` remains useful as a visible contract.

Do not enable global whitespace suppression for Phase 1. Use preserve mode and
adjust local whitespace markers only where snapshots show drift.

## Future Skill Projection Caveat

If the schema crate later uses Askama, its crate root is
`crates/llm-wiki-schema`, so it will not automatically see the root binary
crate's `templates/` directory. That follow-on must choose a crate-local
template folder or a deliberate `askama.toml` strategy.
