# Plan: Project and Skill Rename

- Document Class: Plan
- Status: Completed
- Date: 2026-05-08
- Category: Branding, naming, migration
- Scope: Execute D11: the rename from `llm-wiki-framework` to `llm-wiki-rs` and from `knowledge*` skills to `wiki-*`, while handling the small set of legacy symlinked repos through a one-off migration that is explicitly outside the framework's long-term architecture.
- Sources: wiki/proposals/project-and-skill-rename.proposal.md, wiki/decisions/llm-wiki-binary-distribution.decision.md, wiki/decisions/binary-path-bootstrap.decision.md, wiki/decisions/composable-project-init.decision.md, live local skill-symlink inspection under `~/.claude/skills/` and `~/.codex/skills/` on 2026-05-08
- Related: wiki/roadmaps/framework-v1.roadmap.md, wiki/specs/wiki-init-skill.spec.md, wiki/specs/wiki-query-skill.spec.md, wiki/specs/wiki-ingest-skill.spec.md, wiki/specs/wiki-research-skill.spec.md, wiki/specs/wiki-lint-skill.spec.md

## Deliverable

D11: land the rename as a clean product change:

- Cargo package: `llm-wiki-framework` -> `llm-wiki-rs`
- Skill namespace: `knowledge*` -> `wiki-*`
- Dispatcher: `knowledge` -> `wiki`
- Runtime mirrors regenerated under `.claude/` and `.codex/`

The framework does not grow a legacy compatibility layer. Old `knowledge*`
skill paths survive only through a one-off migration in the few local repos
that still depend on symlinked in-repo mirrors. That migration renames those
repos' in-repo mirrors to `.claude.legacy/` and `.codex.legacy/` and repoints
the existing home-level skill symlinks under `~/.claude/skills/` and
`~/.codex/skills/` at the legacy trees.

## Current Legacy Inventory

The live state inspected on 2026-05-08 established the migration target, and
the final migrated state was verified on 2026-05-09:

1. `~/.claude` and `~/.codex` themselves are real directories, not symlinks.
   The legacy linkage lives at the skill-directory level.
2. `~/.claude/skills/` currently contains symlinks for:
   `init-project`, `knowledge-ingest`, `knowledge-lint`, `knowledge-query`,
   `knowledge-research`.
3. `~/.codex/skills/` currently contains symlinks for:
   `init-project`, `knowledge`, `knowledge-ingest`, `knowledge-lint`,
   `knowledge-query`, `knowledge-research`.
4. Before migration, those `knowledge*` symlinks pointed into this repo's
   current in-repo runtime mirrors under `.claude/skills/` and
   `.codex/skills/`.
5. `~/.codex/skills/knlg` separately points at
   `/Users/nicolasmartino/Documents/local_llm_wiki/es_llm_wiki/.codex/skills/knlg`.
   That link is not part of this rename and should be left alone.
6. Final migrated state: the known legacy symlinks now point at frozen
   `.claude.legacy/skills/...` and `.codex.legacy/skills/...` trees in this
   repo, while the active `wiki*` runtime mirrors remain under `.claude/` and
   `.codex/`.

## Runtime Proof Status

The framework-side rename proof landed first. The one-off legacy migration is
now complete.

### Verified

1. Filesystem separator pre-flight on this macOS host accepts both `wiki-*`
   and `wiki:*` directory names under `/private/tmp/llm-wiki-rename-proof/`.
   This keeps `wiki-*` as the preferred portable form but confirms `wiki:*`
   is not rejected immediately on the current machine.
2. Baseline `cargo build` and `cargo test --workspace` were green on the
   pre-rename branch.
3. Post-rename framework work landed: active `wiki-*` specs exist, the runtime
   mirrors were regenerated, and the rename is recorded in `wiki/log.md`.
4. Live home-level legacy symlink topology is documented precisely enough to
   execute the one-off migration without guessing.

5. The home-level legacy symlinks under `~/.claude/skills/` and
   `~/.codex/skills/` were repointed on 2026-05-09 to frozen
   `.claude.legacy/` / `.codex.legacy/` trees in this repo.
6. `~/.codex/skills/knlg` was left untouched.

## In Scope

- Cargo package rename to `llm-wiki-rs`
- Embedded canonical skill directory and self-reference rename to `wiki-*`
- Dispatcher rename from `knowledge` to `wiki`
- User-visible CLI help, manifest entries, install paths, status output, and
  tests updated to the new names
- Runtime mirror regeneration via `llm-wiki build --out .`
- Active wiki/spec/decision/plan/roadmap prose sweep
- One-off migration instructions for the few local legacy repos

## Out Of Scope

- Product-level aliases for `knowledge*`
- A manifest schema or install-mode branch dedicated to legacy support
- Automatic legacy migration in `llm-wiki install`
- Changes to `~/.llm_wiki/` storage paths or per-project `.llm_wiki/init.toml`
- Changes to `knlg` or the `es_llm_wiki` repo

## Phases

### 0. Baseline and proof notes

1. Record the live symlink inventory and separator pre-flight result in this
   plan.
2. Run a baseline `cargo build` and `cargo test --workspace` on the pre-rename
   branch so rename regressions can be distinguished from pre-existing breakage.
3. Treat full renamed-surface runtime proof as blocked until the code emits the
   new names.

### 1. Package and canonical skill rename

1. Rename Cargo package metadata to `llm-wiki-rs`; leave the binary name
   `llm-wiki`.
2. Rename canonical skill directories under `assets/skills/`:
   `knowledge` -> `wiki`,
   `knowledge-init` -> `wiki-init`,
   `knowledge-query` -> `wiki-query`,
   `knowledge-ingest` -> `wiki-ingest`,
   `knowledge-research` -> `wiki-research`,
   `knowledge-lint` -> `wiki-lint`.
3. Update self-references inside each canonical `SKILL.md`.

### 2. Projector and install-surface rename

1. Update embedded asset registration in `src/embed.rs`.
2. Update any user-visible skill-name strings in install, build, doctor,
   manifest, status, and test code.
3. Regenerate `.claude/` and `.codex/` with `llm-wiki build --out .` rather
   than hand-editing projected files.

### 3. Active-doc sweep

1. Rename active skill spec filenames under `wiki/specs/` from
   `knowledge-*-skill.spec.md` to `wiki-*-skill.spec.md`.
2. Replace active prose references to `knowledge-*`, `$knowledge`, and
   `llm-wiki-framework` with the new names where they describe current truth.
3. Archive or supersede the namespace decision so the accepted active surface
   is unambiguous.
4. Update `wiki/index.md` and `wiki/log.md`.

### 4. Renamed-surface runtime proof

1. Run `cargo build` and `cargo test --workspace` after the rename lands.
2. Run `llm-wiki install` and verify installed directories are `wiki*`, not
   `knowledge*`.
3. Check Claude and Codex discovery under the new names.
4. Invoke `wiki-init`, `wiki-query`, and `wiki-lint`.
5. Run `llm-wiki uninstall` and confirm no renamed-surface orphans remain.

### 5. One-off legacy migration

This phase is deliberately not product behavior.

1. In each legacy repo that still needs the old symlinked runtime mirrors,
   rename `.claude/` to `.claude.legacy/` and `.codex/` to `.codex.legacy/`.
2. Repoint the existing home-level symlinks under `~/.claude/skills/` and
   `~/.codex/skills/` so they target the new legacy paths.
3. Leave `knlg` and unrelated skill symlinks untouched.
4. Do not add any framework code whose purpose is to automate or preserve this
   state.

### 6. Closeout

1. Update this plan to `Completed`.
2. Update the proposal to Accepted/Promoted if that promotion has landed by
   then.
3. Record the final migration result in `wiki/log.md`.

## Verification Gates

1. Baseline `cargo build` and `cargo test --workspace` are recorded before the
   rename sweep.
2. Post-rename `cargo build` and `cargo test --workspace` are green.
3. `llm-wiki build --out .` regenerates the committed runtime mirrors with only
   intended rename diffs.
4. `llm-wiki install` writes only `wiki*` skill names.
5. Claude and Codex expose the new names and the core renamed commands invoke
   correctly.
6. `llm-wiki uninstall` leaves no renamed-surface orphans.
7. The one-off legacy migration rewrites only the known `knowledge*` /
   `init-project` symlinks and leaves `knlg` untouched.
8. No active wiki page describes legacy symlink preservation as part of the
   framework architecture.

## Pages To Update On Completion

- `wiki/proposals/project-and-skill-rename.proposal.md`
- `wiki/archive/knowledge-command-namespace.decision.md`
- `wiki/specs/wiki-init-skill.spec.md` and the active `wiki-*-skill`
  spec family
- `wiki/decisions/llm-wiki-binary-distribution.decision.md` if wording still
  names the old surface as current truth
- `wiki/index.md`
- `wiki/log.md`

## What Closes The Plan

The plan closes when the product emits only the renamed `wiki-*` surface, the
active docs agree, the new runtime proof passes, and the known home-level
legacy symlinks have been moved onto frozen `.claude.legacy/` /
`.codex.legacy/` trees without teaching the framework to care about them.
