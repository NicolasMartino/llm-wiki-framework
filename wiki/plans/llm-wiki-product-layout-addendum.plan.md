# Plan: `llm-wiki` Product Layout Addendum

- Document Class: Plan
- Status: Active
- Date: 2026-05-06
- Category: Tooling, repository layout
- Scope: Post-D8 repository layout correction that makes the `llm-wiki` binary the root product crate and moves embedded product assets out of legacy root locations.
- Sources: wiki/plans/llm-wiki-binary.plan.md, wiki/decisions/llm-wiki-binary-distribution.decision.md, README.md, Cargo.toml, tools/llm-wiki/
- Related: wiki/specs/documentation-model.spec.md, wiki/specs/init-project-skill.spec.md, wiki/specs/knowledge-query-skill.spec.md, wiki/specs/knowledge-ingest-skill.spec.md, wiki/specs/knowledge-research-skill.spec.md, wiki/specs/knowledge-lint-skill.spec.md

## Purpose

D8 made `llm-wiki` the product, but the repository still reflects the
implementation path that got it there: the binary crate lives under
`tools/llm-wiki/`, canonical skills live at root `skills/`, and the project
guidelines template lives at root `project_guidelines.template.md`.

This addendum corrects layout only. It does not change CLI behavior, manifest
schema, generated skill content, snapshot expectations, or the D8 distribution
decision.

## Target Layout

```text
Cargo.toml                  # root package + workspace
build.rs                    # validates embedded product assets at compile time
src/                        # llm-wiki binary crate
assets/
  skills/                   # canonical embedded skill assets
    init-project/SKILL.md
    init-project/codex/openai.yaml
    knowledge-query/SKILL.md
    ...
  templates/
    project_guidelines.md   # embedded project guidelines template
    CLAUDE.md               # embedded agent instruction template
crates/
  llm-wiki-schema/          # pure parser/projector library
wiki/                       # framework knowledge
raw/                        # framework source material
.claude/skills/             # generated convenience outputs
.codex/skills/              # generated convenience outputs
```

The root package is `llm-wiki-framework`; the installed binary remains
`llm-wiki`.

## Required Changes

1. Convert the repository root `Cargo.toml` into both the workspace root and
   the `llm-wiki-framework` package. The workspace members list shrinks to
   `["crates/llm-wiki-schema"]`; the root binary package is implicit.
2. Move binary crate source from `tools/llm-wiki/src/` to root `src/`.
3. Move binary `build.rs` from `tools/llm-wiki/build.rs` to root `build.rs`.
4. Move binary integration tests from `tools/llm-wiki/tests/` to root `tests/`.
5. Move canonical skill assets from root `skills/` to `assets/skills/`.
6. Move `project_guidelines.template.md` to
   `assets/templates/project_guidelines.md`.
7. Move `tools/llm-wiki/templates/CLAUDE.md` to
   `assets/templates/CLAUDE.md` as the single embedded agent instruction
   template.
8. Remove root `skills/`, root `project_guidelines.template.md`, and
   `tools/llm-wiki/`.
9. Update `include_str!` paths, build-time validation paths, tests, snapshots,
   README, specs, `.github/workflows/`, `justfile`, wiki plan/index/log
   entries, and audit recipes.
10. Let `Cargo.lock` refresh only as required by the root package move.
11. Check `.cargo-llvm-cov.toml` for path-sensitive exclusions and update only
   if needed.
12. Use `git mv` where practical so review history remains readable.

## Implementation Notes

The move should simplify path handling rather than preserve old path math.

`build.rs` root resolution:

```rust
// before, inside tools/llm-wiki/
let root = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?)
    .join("../..")
    .canonicalize()?;

// after, at repository root
let root = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);
```

Embedded skill paths:

```rust
// before
include_str!("../../../skills/init-project/SKILL.md")

// after
include_str!("../assets/skills/init-project/SKILL.md")
```

Template paths:

```rust
// before
include_str!("../../../project_guidelines.template.md")
include_str!("../templates/CLAUDE.md")

// after
include_str!("../assets/templates/project_guidelines.md")
include_str!("../assets/templates/CLAUDE.md")
```

`insta` snapshots may need re-acceptance because source headers move from
`tools/llm-wiki/tests/...` to `tests/...`. Re-acceptance is allowed only for
source-path metadata or deliberate path-reference updates; rendered skill
content must remain unchanged unless the diff is separately justified.

This layout correction should land as one commit or one PR-sized merge. Main
must not sit in a half-moved state containing both `tools/llm-wiki/` and root
`src/`, or both root `skills/` and `assets/skills/` as canonical sources.

## Invariants

- `cargo install --path .` produces the `llm-wiki` executable.
- `cargo run -- build --out .` regenerates the committed `.claude/skills/`
  and `.codex/skills/` outputs.
- `.claude/skills/` and `.codex/skills/` remain generated convenience outputs,
  not canonical source.
- `assets/skills/` is canonical product content.
- `assets/templates/` is canonical scaffold template content.
- No root `skills/` directory remains.
- No `tools/llm-wiki/` directory remains.
- No active docs refer to root `skills/<name>/SKILL.md` as canonical.
- No active docs refer to root `project_guidelines.template.md` as canonical.
- Subcommands, manifest schema, installed file paths, and generated runtime
  skill contents remain unchanged.

## Verification

1. `cargo fmt --all --check`
2. `cargo test --workspace`
3. `cargo clippy --workspace --all-targets --all-features -- -D warnings -D dead_code`
4. `cargo insta test --workspace --check`
5. `cargo llvm-cov --workspace --fail-under-lines 80`
6. `cargo +nightly udeps --workspace`
7. `cargo install --path . --force`
8. `cargo dist plan`
9. `llm-wiki build --out .` produces no uncommitted runtime-output diff.
10. `rg -n 'tools/llm-wiki|project_guidelines\.template\.md|skills/[a-z-]+/SKILL\.md|canonical source: `?skills/|Canonical source: `?skills/' wiki README.md justfile Cargo.toml .github AGENTS.MD CLAUDE.md .cargo-llvm-cov.toml src assets tests crates` returns no active-doc, workflow, or code references.

## Closure

This addendum closes when the target layout is in place, all verification
gates pass, and `wiki/index.md` and `wiki/log.md` record the layout correction.
