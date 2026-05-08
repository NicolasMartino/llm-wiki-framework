# Source 005: Askama template syntax for D10 composition

- URL: https://askama.rs/en/latest/template_syntax.html
- Retrieved: 2026-05-07
- Mode: web
- Source Type: primary documentation

## Relevant Facts

- Askama's HTML escaping can be disabled by extension or with
  `escape = "none"`. In HTML contexts, `safe` can prevent escaping for a single
  expression.
- `for` loops iterate over Rust iterators and support optional `{% else %}`
  blocks for empty loops. Loop metadata includes `loop.index`, `loop.index0`,
  `loop.first`, and `loop.last`.
- `if` mirrors Rust-style conditional logic, including `else if` / `elif`.
- `if let` is supported for matching options and similar patterns.
- Methods can be called on variables in scope, including `self`.
- Askama supports rendering one template inside another by placing a template
  value in context. For HTML inner templates, docs warn that `safe` may be
  needed to avoid literal escaped HTML.
- Macros exist and can be imported from external files, with typed or defaulted
  arguments.
- Block fragments can render a named block by itself through the derive
  `block` parameter.

## D10 Relevance

Base templates can loop over fragment strings:

```askama
{% for fragment in agents_fragments %}
{{ fragment }}
{% endfor %}
```

Conditional sections should map directly:

```askama
{% if include_ml_ai %}
...
{% endif %}
```

For D10's runtime-selected pack list, pre-rendered `Vec<String>` fragments are
simpler than Askama inheritance. Inheritance and block fragments are better for
static template relationships, not CLI-selected pack composition.

## Implementation Note

Keep policy decisions in Rust. Templates should display already-composed
fields, booleans, and fragments rather than sorting, deduplicating, or
validating pack combinations.
