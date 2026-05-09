# Plan: `llm-wiki` Product Layout Addendum

- Document Class: Plan
- Status: Completed
- Date: 2026-05-06
- Category: Tooling, repository layout
- Scope: Completed post-D8 repository layout correction that makes the `llm-wiki` binary the root product crate and moves embedded product assets under `assets/`.
- Sources: wiki/plans/llm-wiki-binary.plan.md, wiki/decisions/llm-wiki-binary-distribution.decision.md, README.md, Cargo.toml, src/, assets/, tests/
- Related: wiki/specs/documentation-model.spec.md, wiki/specs/wiki-init-skill.spec.md, wiki/specs/wiki-query-skill.spec.md, wiki/specs/wiki-ingest-skill.spec.md, wiki/specs/wiki-research-skill.spec.md, wiki/specs/wiki-lint-skill.spec.md

## Purpose

D8 made `llm-wiki` the product. This addendum corrected the repository shape
so the product crate is at the repository root and embedded product assets live
under `assets/`.

The change did not alter CLI behavior, manifest schema, installed paths, or
the D8 binary-distribution decision. Later D10/D11 follow-ons changed template
locations and package/skill names; this plan's current layout section reflects
those follow-ons.

## Final Layout

```text
Cargo.toml                  # root package + workspace
build.rs                    # compile-time embedded asset validation
src/                        # llm-wiki binary crate
assets/
  skills/                   # canonical embedded skill assets
templates/                  # Askama scaffold and skill-projection templates
crates/
  llm-wiki-schema/          # pure parser/projector library
tests/                      # binary integration tests and fixtures
wiki/                       # framework knowledge
raw/                        # framework source material
.claude/skills/             # generated convenience outputs
.codex/skills/              # generated convenience outputs
```

The root package is now `llm-wiki-rs`; the installed binary remains
`llm-wiki`. D10 later moved scaffold templates from `assets/templates/` to
`templates/`, and the skill-projection follow-on added
`templates/skills/` for runtime skill output.

## Completed Changes

1. Converted root `Cargo.toml` into both workspace root and
   `llm-wiki-framework` package.
2. Moved binary source to root `src/`.
3. Moved binary `build.rs` to the repository root.
4. Moved binary integration tests and fixtures to root `tests/`.
5. Moved canonical skill assets to `assets/skills/`.
6. Moved scaffold templates to `assets/templates/`.
7. Removed the old nested binary crate directory and old root canonical asset
   locations.
8. Updated `include_str!` paths, build-time validation paths, tests, snapshots,
   README-adjacent developer recipes, workflows, specs, index, and log.
9. Regenerated committed runtime outputs from the moved canonical assets.
10. Regenerated the cargo-dist release workflow for the root package layout.
11. Split nontrivial implementation out of `mod.rs` files into named modules
    (`answers`, `command`, `scaffold`, `schema`, `format`, and `types`).

## Invariants

- `cargo install --path .` produces the `llm-wiki` executable.
- `cargo run -- build --out .` regenerates committed runtime outputs.
- `.claude/skills/` and `.codex/skills/` remain generated convenience outputs.
- `assets/skills/` is canonical product content.
- `templates/base/` and `templates/packs/` are canonical scaffold template
  content.
- `templates/skills/` is canonical skill-projection structure.
- Subcommands, manifest schema, installed file paths, and generated runtime
  skill contents remain unchanged except for removing stale references to the
  retired template filename from ingest/lint skill text.

## Verification

Completed locally:

1. `cargo fmt --all --check`
2. `cargo test --workspace`
3. `cargo clippy --workspace --all-targets --all-features -- -D warnings -D dead_code`
4. `cargo insta test --workspace --accept` for deliberate skill text snapshot updates
5. `cargo run -- build --out .`
6. `cargo llvm-cov --workspace --fail-under-lines 80`
7. `cargo +nightly udeps --workspace`
8. `cargo install --path . --force`
9. `dist plan` using pinned cargo-dist 0.28.0

## Closure

Closed on 2026-05-06 when the target layout landed, runtime outputs were
regenerated from `assets/skills/`, and wiki/index/log bookkeeping recorded the
layout correction.
