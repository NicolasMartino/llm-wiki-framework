# Source 006: Local D10 context

- Paths Reviewed:
  - wiki/plans/composable-project-init.plan.md
  - wiki/decisions/composable-project-init.decision.md
  - wiki/index.md
- Retrieved: 2026-05-07
- Mode: local context
- Source Type: project wiki

## Relevant D10 Constraints

- `llm-wiki init` must render canonical `AGENTS.md` and
  `project_guidelines.md`; `CLAUDE.md` becomes a small compatibility shim.
- Template files use `.md` under `templates/`, not `.md.jinja`.
- Askama should be invoked with `escape = "none"` for Markdown output.
- Pack fragments must be compile-time templates selected by exhaustive Rust
  matches, not arbitrary runtime template path lookup.
- Old `ProjectProfile` booleans are allowed only as a Phase 1 compatibility
  bridge.
- Final non-interactive init surface is `--blueprint <name>` plus repeatable
  `--pack <name>`; old `--type` and `--scale` are retired.
- Phase 1 should preserve current output bytes except for intentional
  `AGENTS.md` / `CLAUDE.md` transition.
- D10 closes only after tests and two contrasting blueprint bootstraps prove
  the system works beyond snapshots.

## D10 Askama Design Implications

- Phase 1 should be a refactor plus file-target transition, not pack work.
- Phase 2/3 should introduce Rust-side catalog/composition before writing all
  pack text.
- Fragment render methods should likely return `anyhow::Result<Option<String>>`
  rather than plain `Option<String>` so Askama render errors are not hidden.
- The base render context should contain final display-ready lists and
  fragments; Rust owns dedupe/order/policy.
