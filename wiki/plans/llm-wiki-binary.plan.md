# Plan: `llm-wiki` Binary Implementation (D8)

- Document Class: Plan
- Status: Completed
- Date: 2026-05-06
- Category: Tooling, framework distribution
- Scope: Implement the `llm-wiki` Rust binary that owns global skill installation, project scaffolding, and skill projection per the accepted D8 deliverable.
- Sources: wiki/proposals/llm-wiki-binary.proposal.md, wiki/decisions/llm-wiki-binary-distribution.decision.md, wiki/roadmaps/framework-v1.roadmap.md (D8), wiki/decisions/single-source-skills.decision.md, wiki/decisions/framework-path-resolution.decision.md, wiki/specs/init-project-skill.spec.md
- Related: wiki/specs/documentation-model.spec.md, wiki/specs/knowledge-ingest-skill.spec.md, wiki/specs/knowledge-query-skill.spec.md, wiki/specs/knowledge-research-skill.spec.md, wiki/specs/knowledge-lint-skill.spec.md, skills/README.md

## 1. Deliverable

D8: `llm-wiki` Rust binary. Single-command install of framework skills globally for Claude Code and Codex; deterministic project scaffolding; canonical-markdown skill projection with golden-file tests; `cargo-dist` multi-arch distribution.

This plan executes the binary's V1 (D8 v0.1). It does not re-litigate design choices — those are in `wiki/decisions/llm-wiki-binary-distribution.decision.md` and the accepted proposal.

## Post-Completion Addendum

D8 is behaviorally completed. A follow-up layout correction is tracked in
`wiki/plans/llm-wiki-product-layout-addendum.plan.md`: move the binary from
`tools/llm-wiki/` to the root package layout and move embedded product assets
from root `skills/` and `project_guidelines.template.md` into `assets/`.
The addendum does not change D8 CLI behavior, manifest schema, or generated
runtime outputs.

## 2. In Scope

- New Rust workspace at the repo root with two crates: `crates/llm-wiki-schema/` (the parsing and projection library — pure, no I/O) and `tools/llm-wiki/` (the binary, depending on `llm-wiki-schema` as both a normal dep and a build-dep so the binary's `build.rs` can validate embedded canonicals at compile time).
- All six subcommands: `install`, `build`, `init`, `status`, `doctor`, `uninstall`.
- Embedded canonical skill content (`skills/<name>/SKILL.md`) via `include_str!` after migrating those files from `runtime-specific HTML marker` block markers to the typed schema.
- Embedded project templates (`project_guidelines.template.md`, embedded `CLAUDE.md` template).
- Manifest at `~/.local/share/llm-wiki/manifest.json` with the three-way hash policy from the proposal.
- Golden-file (`insta`) snapshot tests, integration tests with `tempfile::TempDir` and `HOME` redirection, property tests with `proptest`, post-install verification tests.
- Compat fixtures at `tools/llm-wiki/tests/fixtures/wikis/v1/` plus the agent-driven `wiki/checklists/v1-fixture-smoke.checklist.md`. (One canonical path; all references in this plan use it.)
- `cargo-dist` configuration for the four target platforms.
- CI: GitHub Actions running `cargo test`, `cargo clippy -- -D warnings`, `cargo fmt --check`, `cargo insta test --check`, `cargo-llvm-cov` with the 80% gate.
- Update of the `init-project` agent skill (markdown) to be a thin conversational wrapper that shells out to `llm-wiki init --non-interactive`.
- Cleanup pass: remove `pre-binary render script`, drop `legacy tag marker` markers from canonical skills, update specs.
- Wiki bookkeeping: flip predecessor decisions and the bash-renderer plan to `Superseded` after the binary is in operation; update `documentation-model.spec.md` to record the distribution model as proven; mark D8 `Completed` on the roadmap.

## 2a. No-Legacy Discipline (Load-Bearing)

D8 is a clean-slate replacement, not an additive evolution. The pre-binary renderer, the conditional-block canonical format, the project-local-skill model, and the markdown-driven scaffolding skill are all replaced — not preserved alongside the binary. This section is non-negotiable; it shapes stages 5.4, 5.11, 5.12, and 5.13.

**Principles:**

1. **Single source of truth at any moment.** During implementation, the pre-binary renderer remains the operating model. At the merge that ships D8, the pre-binary renderer is gone. There is no main-branch state in which both systems coexist.
2. **Dead code is a release blocker.** No code paths in the binary that parse `runtime-specific marker` / `runtime-specific marker` blocks, no fallback for "old canonical format," no compatibility shims for the pre-binary skill layout. Every line in the binary serves the current spec; CI enforces this.
3. **Old prose is deleted, not migrated.** The current `skills/init-project/SKILL.md` contains hundreds of lines describing the question flow, profile resolution, scaffolding behavior, etc. After stage 5.11, that prose is gone — not moved into a comment, not preserved in `archive/`. The wrapper is short and self-contained.
4. **Spec rewrites, not spec patches.** `wiki/specs/init-project-skill.spec.md` and the `knowledge-*-skill.spec.md` family are rewritten from scratch under the binary's authority. The old text is discarded, not edited.
5. **Superseded predecessors move to `wiki/archive/`.** Per `project_guidelines.template.md:327`, archived documents leave the active index. Predecessor decisions and the bash-renderer plan get archived at the D8 merge — not left in `wiki/decisions/` or `wiki/plans/` with a stale `Superseded` status accumulating.
6. **Open questions resolve before stage 5.1.** §12's four implementation questions are decided before the scaffold step. No "we'll figure it out during implementation" hedge — that's how dead code paths get committed.
7. **Backward compatibility is bounded.** The binary preserves compat for *external project wikis* (the `wiki/` shape promise from the proposal). It does **not** preserve compat for the *internal bash-renderer canonical format*. Those are different surfaces; the first is a feature, the second is dead code.

**Enforcement:** §6 adds a "No-Legacy Audit" gate. CI runs `cargo +nightly udeps` (unused dependencies), `cargo clippy --all-targets -- -D warnings -D dead_code` (dead code as a hard error), and a custom grep that fails the build if any of the legacy markers (`runtime-specific marker`, `runtime-specific marker`, `section-end marker`, `pre-binary render script`, references to "pre-binary renderer" outside `wiki/archive/`) appears anywhere in the working tree at v0.1.0.

## 3. Out Of Scope

- `init` Update mode — agent-owned (per accepted decision).
- Automated runtime skill-discovery tests — no stable API exists on either runtime.
- `llm-wiki self-update` — users update via package manager, GitHub release download, or `cargo install`.
- Per-project skill overrides.
- Runtimes beyond Claude and Codex (Cursor, Aider, Amp deferred to V2).
- MCP-based skill exposure.
- Network fetching of skills (all content embedded at compile time).
- Windows builds in V1 (`cargo install` available; `cargo-dist` matrix can add Windows later).
- Configuration files (`~/.config/llm-wiki/config.toml`) — no V1 use case.

## 4. Workspace Layout

The Rust workspace lives alongside the framework's existing files. Two crates, separated so the binary's `build.rs` can validate embedded canonicals at compile time (a build script cannot depend on the package being built, so the schema/projector logic must live in a separate crate that the binary uses both as a normal dep and a build-dep).

```text
Cargo.toml                          # workspace root
crates/
  llm-wiki-schema/                  # pure library: parsing + projection
    Cargo.toml                      # name = "llm-wiki-schema"
    src/
      lib.rs                        # re-exports
      frontmatter.rs                # YAML deserialization with serde
      body.rs                       # markdown section parsing
      validation.rs                 # field validation, required-section checks
      doc.rs                        # SkillDoc = Frontmatter + Body
      projector/
        mod.rs                      # trait Projector
        claude.rs                   # ClaudeProjector impl
        codex.rs                    # CodexProjector impl
        idiom.rs                    # invocation-syntax rewriting (slash vs $ namespace)
    tests/
      schema.rs                     # unit tests on tiny inline canonicals
      projector.rs                  # unit tests with hand-coded SkillDoc inputs
tools/
  llm-wiki/                         # the binary
    Cargo.toml                      # name = "llm-wiki-framework", [[bin]] name = "llm-wiki"
                                    # depends on llm-wiki-schema (normal + build-dep)
    build.rs                        # uses llm-wiki-schema to parse every embedded skill
                                    # at compile time; build fails on malformed canonical
    src/
      main.rs                       # clap entry point, subcommand dispatch
      cli.rs                        # clap derive structs
      embed.rs                      # include_str!-based access to skills/ content
      manifest/
        mod.rs                      # Manifest struct, atomic read/write
        hash.rs                     # SHA-256 helpers
        collision.rs                # three-way comparison logic
      install.rs                    # install subcommand
      build.rs                      # build subcommand (NB: this is src/build.rs, not the
                                    # crate's build.rs at the crate root)
      init/
        mod.rs                      # init subcommand
        profile.rs                  # ML_AI, QMD, IS_EXISTING resolution
        template.rs                 # conditional-section resolution
        collision.rs                # framework-artifact detection
        sources.rs                  # initial-source copying (binary owns the copy; ingest
                                    # stays with the agent)
      status.rs
      doctor.rs
      uninstall.rs
      paths.rs                      # platform-aware path resolution (HOME-redirectable)
    tests/
      install.rs                    # integration: HOME redirection, idempotency, --force
      init.rs                       # integration: profiles, framework-artifact collision
      uninstall.rs                  # integration: manifest-only deletion
      doctor.rs                     # integration: drift detection, broken-symlink reporting
      build.rs                      # integration: --out, --target
      properties.rs                 # proptest: idempotency, totality
      fixtures/
        wikis/v1/                   # full v1-shaped wiki
        wikis/v1-expected.rs        # hand-coded expected metadata structs
      snapshots/                    # insta snapshots (committed)
```

**Package vs binary names** are distinct on purpose: the package on crates.io is `llm-wiki-framework`, the installed binary is `llm-wiki`. So `cargo install llm-wiki-framework` produces an `llm-wiki` executable on `$PATH`. This is a standard Cargo pattern.

## 5. Steps

Stages run in order. Each stage produces a verifiable artifact and ends in a checkpoint commit.

### 5.1 Workspace scaffold and CI (foundation)

1. Add a workspace `Cargo.toml` at the repo root listing both crates as members.
2. `cargo new --lib crates/llm-wiki-schema`. Set `name = "llm-wiki-schema"`. Pure library.
3. `cargo new --bin tools/llm-wiki`. Set package `name = "llm-wiki-framework"` and `[[bin]] name = "llm-wiki"` so `cargo install llm-wiki-framework` produces an `llm-wiki` executable.
4. Reserve the `llm-wiki-framework` package name on crates.io.
5. `tools/llm-wiki/Cargo.toml` lists `llm-wiki-schema` as both a normal dependency and a `[build-dependencies]` entry — required for `build.rs` to validate embedded canonicals at compile time.
6. Schema crate dependencies: `serde`, `serde_yaml`, `serde_json`, `pulldown-cmark` (or hand-rolled section parser), `thiserror`. Binary crate dependencies (in addition to `llm-wiki-schema`): `clap` (derive), `sha2`, `chrono`, `anyhow`. Dev: `tempfile`, `insta`, `proptest`, `assert_cmd`, `predicates`.
7. Add `.github/workflows/ci.yml`: matrix over (macos-14 [arm64], macos-13 [x86_64], ubuntu-latest [x86_64], ubuntu-latest-arm). Steps: fmt check, clippy with `-D warnings`, `cargo test --workspace`, `cargo insta test --check`, `cargo-llvm-cov --workspace` with 80% gate.
8. Add `.cargo-llvm-cov.toml` with platform-specific exclusions placeholder.

**Verification:** CI passes on all four platforms with both crates empty (lib has `pub fn nothing() {}`, bin has `fn main() {}`).

### 5.2 Schema and parser (lives in `llm-wiki-schema`)

1. Define `SkillFrontmatter` struct mirroring the proposal's canonical schema (name, description, runtimes, operations, arguments, invocation_style, dispatcher_for). All required-vs-optional and field types match the table in `wiki/proposals/llm-wiki-binary.proposal.md`.
2. Define `SkillBody` with the fixed-shape sections (Title, Purpose, Behavior, Invocation, optional Notes).
3. Implement `parse(input: &str) -> Result<SkillDoc, ParseError>` splitting frontmatter (YAML) from body (markdown), validating required sections.
4. Unit tests covering: valid input, malformed YAML, missing required field, unknown runtime value, conflicting fields, missing required body section, schema-snapshot test (`crates/llm-wiki-schema/tests/snapshots/schema.snap` locks the accepted field set).

**Verification:** `cargo test -p llm-wiki-schema` green; schema snapshot committed.

### 5.3 Projector logic with unit-test fixtures (no real skills yet)

This stage proves the projector logic is correct on controlled inputs, before the real canonicals exist in their migrated form.

1. Define `trait Projector { fn project(&self, doc: &SkillDoc) -> Result<RenderedSkill, ProjectError>; }` in `llm-wiki-schema`.
2. Implement `ClaudeProjector`: rewrites `<skill-name>` invocation patterns to `/skill-name`, applies Claude-flavored frontmatter description, never emits `agents/openai.yaml`.
3. Implement `CodexProjector`: rewrites to `$skill-name` and `$knowledge <op>`, applies Codex-flavored description, emits `agents/openai.yaml` from per-skill template.
4. Unit tests use **hand-coded `SkillDoc` values** (or tiny inline canonical strings parsed via stage 5.2) as input — not the repo's real `skills/<name>/SKILL.md` files, which still have the legacy `legacy tag marker` markup at this point. Cover: every projection rule (slash vs `$`, frontmatter description templating, `agents/openai.yaml` emission), every runtime restriction (Claude-only, Codex-only, both), error cases (skill declares an unsupported runtime).
5. The "snapshot tests against real skills" gate moves to stage 5.4, where the migrated canonicals exist.

**Verification:** `cargo test -p llm-wiki-schema projector::` green; every projection rule exercised on a controlled input; no dependency on real skill content.

### 5.4 Migrate canonical skills to clean schema (single-pass; no legacy residue)

This is the content-completeness pass that the pre-binary renderer skipped (the bug we just hit), combined with the real-skill snapshot gate. Per the No-Legacy Discipline (§2a), this stage is single-pass: the migrated canonicals contain **zero** legacy markup. There is no follow-up cleanup stage that "removes the markers later."

1. For each `skills/<name>/SKILL.md`, diff today's rendered Claude and Codex outputs (`.claude/skills/<name>/SKILL.md` and `.codex/skills/<name>/SKILL.md`) against the current canonical with `legacy tag marker` blocks. Identify all content present in either runtime output but absent from canonical.
2. Rewrite each canonical as: typed YAML frontmatter (per stage 5.2 schema), fixed-shape body sections, **no `runtime-specific marker`, `runtime-specific marker`, `section-end marker`, or any other HTML-comment markers**. Runtime-specific deltas live in the projector, not in the canonical.
3. Per-skill `codex/openai.yaml` files stay where they are (no schema change needed).
4. Run the projector against each migrated canonical and diff against the current committed `.claude/skills/` and `.codex/skills/` outputs. **Every difference is a deliberate decision recorded in PR review** — this prevents recurrence of the silent-content-loss bug.
5. Add `insta` snapshot tests under `tools/llm-wiki/tests/snapshots/` (or in `crates/llm-wiki-schema/tests/snapshots/` if the snapshot is purely a function of canonical → projection): all 11 skill × runtime combinations.
6. Add a CI grep gate (a small shell script in `.github/workflows/`) asserting that no file under `skills/` contains the strings `runtime-specific marker`, `runtime-specific marker`, or `section-end marker`. This gate fires from this stage forward, so any accidental reintroduction breaks CI immediately.

**Verification:** `cargo insta test --check` green for every skill × runtime. The `knowledge` dispatcher snapshot exists only for Codex. Any intentional simplification is captured in PR commentary, not silent. The grep gate passes — no legacy markers anywhere under `skills/`.

### 5.5 Embed canonical content with compile-time validation

1. `tools/llm-wiki/src/embed.rs` uses `include_str!("../../../skills/<name>/SKILL.md")` for each skill (or a build-script-generated registry of `(name, content)` pairs).
2. `tools/llm-wiki/build.rs` imports `llm-wiki-schema` (declared in `[build-dependencies]` of `tools/llm-wiki/Cargo.toml`) and calls its `parse()` on every embedded skill. The build script fails (`cargo build` exits non-zero) if any skill is missing or malformed. This works because `llm-wiki-schema` is a separate crate, so the binary's `build.rs` is allowed to depend on it.
3. Embedded `project_guidelines.template.md` and a `CLAUDE.md` template via the same pattern, with a similar build-time validation pass on the conditional-section markers.
4. CI includes a **negative test**: a separate workflow run perturbs one canonical (e.g., removes a required field), runs `cargo build`, and asserts the build fails. Confirms the compile-time guarantee is real, not theoretical.

**Verification:** crate builds cleanly with all canonicals present and valid; introducing a malformed canonical breaks the build (negative-test workflow asserts this).

### 5.6 `build` subcommand (simplest, no manifest)

1. `clap` struct: `Build { target: Option<Target>, out: PathBuf }`. Default `target=Both`, `out=./build`.
2. Iterate embedded canonicals, project via the appropriate projector(s), write to `<out>/.claude/skills/<name>/SKILL.md` and `<out>/.codex/skills/<name>/SKILL.md` (and `agents/openai.yaml`).
3. Replace `bash pre-binary render script` with `llm-wiki build --out .` in the repo's self-dogfooding workflow. Document in `skills/README.md`.
4. Integration tests: `build --out <tempdir>` produces expected files; `--target claude` skips Codex output; `--out .` overwrites without manifest interference.

**Verification:** `bash pre-binary render script` and `llm-wiki build --out <tempdir>` produce byte-identical output for every skill (one-off cross-check before the pre-binary renderer is removed).

### 5.7 Manifest + `install` + `uninstall`

1. Implement `Manifest` struct: top-level `binary_version`, `installed_at`, `files: Vec<ManifestEntry>`. Atomic write via `tempfile::NamedTempFile::persist` (write to temp + rename).
2. Implement three-way hash comparison from the proposal's collision table. Each of the eight cases is a unit test.
3. Implement `<path>.bak.<UTC-ISO8601>` backup helper (`chrono` for the timestamp).
4. `install` subcommand: walk embedded skills, compute target paths under `~/.claude/skills/` and `~/.codex/skills/`, apply collision policy, write files, write manifest atomically.
5. `uninstall` subcommand: read manifest, delete entries in reverse order, delete manifest. Files not in manifest are never touched.
6. Integration tests:
   - Fresh install (no manifest, no files) → all expected files written; manifest matches.
   - Idempotent re-run (no flags) → second run is a no-op (verified by mtime / hash).
   - Upgrade (manifest has older bundled hashes) → files updated, manifest hashes refreshed.
   - User-edited framework file → default refuses; `--force` backs up with timestamped suffix and overwrites.
   - User content at framework path → default refuses; `--force` backs up and overwrites.
   - Pre-binary symlink (e.g., dangling `~/.codex/skills/init-project` from the legacy `software_project_management` repo) → default refuses with a "run `doctor`" message; `--force` replaces.
   - Repeated `--force` runs produce backups with distinct timestamps.
   - `uninstall` after install → only manifest-owned files removed; user content untouched.
7. Property tests: `install` is idempotent under any sequence of N runs (proptest generator emits valid manifest+filesystem states).

**Verification:** all integration and property tests green under `HOME` redirection; manifest is round-trippable (`serde_json` write → read produces identical struct).

### 5.8 `init` subcommand (Create mode only)

1. Embed `project_guidelines.template.md` and the `CLAUDE.md` template.
2. Conditional-section resolution: `<!-- SECTION:ML_AI -->` and `<!-- SECTION:QMD -->` markers in the embedded template are kept or stripped based on flags.
3. Framework-artifact collision check: if any of `<path>/wiki/`, `<path>/raw/`, `<path>/CLAUDE.md`, `<path>/project_guidelines.md` exists, refuse with a clear error naming which artifact was found.
4. Interactive mode: clap-driven prompts for the six questions from the existing skill spec.
5. `--non-interactive` mode: requires `--name`, `--type`, `--scale` (and optional `--existing`, `--initial-sources`); errors on missing required flags.
6. Writes: `raw/`, `wiki/index.md`, `wiki/log.md`, `wiki/specs/`, `wiki/decisions/`, …, `CLAUDE.md`, `project_guidelines.md` per profile.

**Initial-source handling — agent-vs-binary split.** The existing `init-project` skill spec couples two operations: (a) copy initial source files into `raw/`, (b) ingest them into `wiki/`. Operation (a) is deterministic file copying; operation (b) is LLM-driven judgment. The binary owns (a) only:

- `--initial-sources <paths>`: each path is copied into `<path>/raw/initial/<basename>` (or `raw/initial/<dated-subdir>/<basename>` when multiple sources are provided in one run, to avoid name collisions).
- The binary writes a small `<path>/raw/initial/manifest.md` listing what was copied, when, and from which absolute source path.
- The binary's stdout reports: "Sources copied to `raw/initial/`. Run `knowledge-ingest` to compile them into the wiki."
- The binary does **not** invoke any ingest logic. Ingestion remains the agent's `knowledge-ingest` skill.
- The `init-project` agent skill (the wrapper migrated in stage 5.11) is responsible for the conversational handoff: "Files copied. Should I run `knowledge-ingest` now?" and then dispatching to the ingest skill if the user agrees.

This preserves the principle: deterministic file ops in the binary, LLM-driven judgment in the agent.

7. Golden-file tests for all five profiles: baseline, ML_AI, QMD, ML_AI+QMD, IS_EXISTING (the IS_EXISTING profile asserts that pre-existing source code, configs, tests at `<path>` are untouched and only framework files are added).
8. Additional fixture for `--initial-sources`: a list of three sample files copied into `raw/initial/`; manifest is checked; `wiki/` content is asserted to contain no ingest results (the binary did not ingest).

**Verification:** `cargo insta test --check` green for all five `init` profiles plus the initial-source fixture. Framework-artifact collision is rejected even when the directory contains unrelated files. Initial-source copying never writes to `wiki/`.

### 5.9 `status` and `doctor`

1. `status`: read manifest, compare to filesystem, report per-skill `OK / Missing / Drifted / Unknown`. Includes binary version, install date, total file count.
2. `doctor`:
   - Detect dangling symlinks under `~/.codex/skills/` pointing at `software_project_management` (legacy from review.md §9.2).
   - Detect framework-shaped paths (`~/.claude/skills/<known-skill>/SKILL.md`, `~/.codex/skills/<known-skill>/SKILL.md`) the manifest does not own.
   - Detect manifest entries with no corresponding file (interrupted install, manual deletion).
   - Suggest concrete commands to fix each: `llm-wiki install --force` for drift, manual symlink removal for legacy residue, etc.

**Verification:** integration tests construct each broken-state scenario and assert `doctor` reports it accurately; review.md §9.2 explicitly closed.

### 5.10 Compat fixtures

1. Build `tools/llm-wiki/tests/fixtures/wikis/v1/` from a snapshot of this repo's current `wiki/` (every document type, every status, archive entries, log file).
2. Hand-code expected metadata structs in `tools/llm-wiki/tests/fixtures/wikis/v1-expected.rs`: one struct per fixture document with the fields the parser should produce.
3. Tests assert: every fixture document parses; metadata extraction matches expected; `init` invoked with the fixture's recorded answers regenerates a matching `project_guidelines.md`; every skill name referenced in fixture index/log/specs has a current canonical entry under `skills/`.
4. Create `wiki/checklists/v1-fixture-smoke.checklist.md` for the agent-driven operations-level smoke test (ingest a sample raw source against the fixture, query, lint).

**Verification:** all four fixture assertions green; checklist exists and is referenced from the V1 release-gate workflow.

### 5.11 Replace `init-project` skill with thin wrapper (delete old prose; must happen before release)

This stage runs **before** the cargo-dist release because skills are embedded at compile time (stage 5.5). If the release ships before the wrapper migration, the v0.1.0 binary embeds the old markdown-driven `init-project` skill and breaks the agent-vs-binary contract.

Per the No-Legacy Discipline (§2a), the old prose is **deleted**, not preserved alongside the wrapper. The current `skills/init-project/SKILL.md` describes question flow, profile resolution, conditional sections, scaffolding behavior, file structure, and a six-question table — most of that detail is now the binary's job. The wrapper retains only what the agent does (intake + handoff). Anything the binary owns is gone from the canonical.

1. **Delete** the current `skills/init-project/SKILL.md` body (the "Behavior", "Question Flow", "Project Profiles", "What Gets Generated" sections). They duplicate what the binary now owns; keeping them is dead documentation that will drift.
2. Write a fresh canonical: typed YAML frontmatter, short body. Behavior is: ask the six questions (one section enumerating them), validate, shell out to `llm-wiki init --non-interactive --name X --type Y --scale Z ...`, optionally chain to `knowledge-ingest` for any `--initial-sources`. Aim for under 60 lines total.
3. Re-run the projector; commit the regenerated runtime variants and updated golden-file snapshots. The Claude and Codex variants should each be similarly short.
4. Recompile so the new canonical is re-embedded in the binary.
5. Pre-release smoke check: in a fresh Claude Code session, invoke the `init-project` skill against a tempdir; verify the agent collects answers, calls `llm-wiki init`, and reports completion. Repeat in Codex via `$knowledge init`. Manual smoke tests, captured in the release-gate checklist.

**Verification:** the embedded `init-project` skill (visible via `cargo run -- build --out <tempdir>` and inspecting the rendered output) is the wrapper version, not the legacy markdown-driven version. The canonical is under 60 lines. `git log -p skills/init-project/SKILL.md` shows a clean delete-then-rewrite, not an accumulating patch. Manual smoke tests pass in both runtimes. Spec rewrite lands in stage 5.13.

### 5.12 `cargo-dist` and first release

1. `cargo dist init`. Configure for macos-aarch64, macos-x86_64, linux-x86_64, linux-aarch64. Set up the GitHub Releases workflow.
2. Tag `v0.1.0` once stages 5.1–5.11 have green CI **and** the smoke checks in stage 5.11 have passed in both runtimes.
3. Release-gate workflow runs the post-install verification tests (HOME redirection, manifest cross-check) on every tag.
4. Document install path in repo `README.md`: `curl -L .../llm-wiki-installer.sh | sh && llm-wiki install`.

**Verification:** `v0.1.0` produces signed prebuilt binaries for all four platforms; `cargo install llm-wiki-framework` works; `curl ... | sh` install path completes without manual intervention. The shipped binary embeds the wrapper version of `init-project`, not the legacy version.

### 5.13 Single-merge cleanup (delete legacy, archive superseded, rewrite specs)

Per the No-Legacy Discipline (§2a), this stage lands as **one merge** that contains every removal, every archive move, and every rewrite. There is no main-branch state in which the pre-binary renderer and the binary coexist. If this stage is split across multiple merges, the discipline fails — pick a tighter scope or revert.

**Deletions** (`git rm`, not just edits):

1. `pre-binary render script` — pre-binary renderer is gone.
2. `skills/README.md` — rewritten from scratch under the binary's authority (instructions become "edit `skills/<name>/SKILL.md`, run `cargo run -- build --out .` to refresh local renders, run `llm-wiki install` to deploy globally").
3. Any `runtime-specific marker`, `runtime-specific marker`, `section-end marker` markers anywhere in the working tree (verified by the CI grep gate added in stage 5.4 — this step is the final assert, not a hopeful sweep).
4. Any references to "pre-binary renderer" in active documents outside `wiki/archive/` (verified by a second grep gate).

**Archive moves** (legacy documents leave `wiki/decisions/` and `wiki/plans/`, move into `wiki/archive/`, and drop out of the active index per `project_guidelines.template.md:327`):

5. `wiki/decisions/single-source-skills.decision.md` → `wiki/archive/single-source-skills.decision.md`. Status flipped to `Superseded` with `Superseded By:` linking to the binary-distribution decision. Removed from active index.
6. `wiki/decisions/framework-path-resolution.decision.md` → `wiki/archive/framework-path-resolution.decision.md`. Same flip and archive treatment.
7. `wiki/decisions/project-local-codex-skills.decision.md` (already `Superseded` since 2026-05-06 acceptance pass) → `wiki/archive/project-local-codex-skills.decision.md`. The chain collapses forward: archive note records the path through `single-source-skills` → `llm-wiki-binary-distribution`.
8. `wiki/plans/single-source-skills.plan.md` → `wiki/archive/single-source-skills.plan.md`. Status flipped to `Superseded`. Removed from active index.

**Promotion**:

9. Update `wiki/decisions/llm-wiki-binary-distribution.decision.md`: change `deferred supersession metadata:` to `Supersedes:`; collapse the Consequences section's split (Immediately + On D8 completion) into a single now-effective list. Drop the deferred-supersession framing entirely; it's history.

**Spec rewrites** (delete-then-rewrite per §2a, not edit-in-place):

10. `wiki/specs/init-project-skill.spec.md` — old text deleted; new text describes the agent skill as a thin wrapper over `llm-wiki init`, with the binary as the authority for scaffolding behavior. Question flow lives in the spec only as a reference to the binary's flag set, not as duplicated prose.
11. `wiki/specs/knowledge-{ingest,query,research,lint}-skill.spec.md` — old text deleted; new text describes the canonical source location (`skills/<name>/SKILL.md`), the rendering path (binary), and the runtime invocation. Hardcoded path references and bash-renderer mentions are gone.
12. `wiki/specs/documentation-model.spec.md` — distribution model recorded as proven; the operations table no longer mentions skill rendering as a separate concern (it's now binary-owned implementation detail).

**Roadmap and bookkeeping**:

13. `wiki/roadmaps/framework-v1.roadmap.md`: D8 status `Active` → `Completed`. D8's "Closes" list verified against actual state (review.md §9.1 closed by construction, §9.2 closed by `doctor` reporting on legacy symlinks during install).
14. `wiki/index.md`: archived documents removed from active sections; new spec versions reflected; status lines for D8 updated.
15. `wiki/log.md`: single entry recording the D8 ship — what was deleted, what was archived, what was rewritten, and the No-Legacy Audit gate result from §6.

**Annotations cleanup**:

16. Remove `forward pointer metadata:` fields from any document — they were transitional. Once predecessors are archived and the new decision is `Supersedes:`, the forward-pointing annotations are dead metadata.

**Verification:** the No-Legacy Audit gate (§6.17) passes. `wiki/index.md` and `wiki/log.md` are coherent. No stale `forward pointer metadata:`, no stale `deferred supersession metadata:`, no stale `legacy tag marker` markers anywhere. `git diff main` for the merge that ships D8 shows: binary added, pre-binary renderer deleted, legacy markers removed, predecessor decisions moved to archive, specs rewritten — all in one commit range.

## 6. Verification Gates

Each gate maps to one or more acceptance criteria from the proposal (cited in parens).

**Functional gates:**

1. `llm-wiki install` against redirected `HOME` writes correct files with manifest entries; second run is a no-op. (AC #1)
2. Default `install` refuses user-authored collisions; `--force` backs up to `<path>.bak.<UTC-ISO8601>`; repeated `--force` runs produce distinct backups. (AC #2)
3. `llm-wiki init <path>` Create mode produces correct structure for all five profiles via golden-file fixtures. Update mode invocation against a path containing any framework artifact returns a clear error. (AC #3)
4. Canonical skill source contains no `runtime-specific marker` / `runtime-specific marker` blocks; per-runtime variation is fully in the projector. (AC #4)
5. `llm-wiki uninstall` removes only manifest-owned files; user content untouched. (AC #5)
6. `llm-wiki doctor` flags dangling pre-binary symlinks under `~/.codex/skills/` and reports manifest-vs-filesystem drift accurately. (AC #6, closes review.md §9.2)

**Test gates:**

7. `cargo insta test --check` passes for all 11 skill × runtime combinations and all 5 `init` profiles. (AC #7)
8. Unit tests cover schema parsing, projectors, embed loader, path resolution, profile resolution. (AC #8)
9. Integration tests cover all eight collision-policy cases, `--force` behavior, `uninstall` symmetry, `init` profile correctness, `doctor` diagnostics, `status` drift detection. (AC #9)
10. Property tests cover install idempotency and renderer totality. (AC #10)
11. Compat fixture v1: parseability, metadata extraction, template compat, skill availability all pass. (AC #11)
12. Line coverage ≥ 80% on `cargo-llvm-cov` default-feature run. (AC #12)

**Distribution gates:**

13. `v0.1.0` builds cleanly on all four target platforms via `cargo-dist`. (AC #13)
14. `cargo install llm-wiki-framework` works for users with a Rust toolchain. (AC #14)
15. End-user install path (`curl ... | sh && llm-wiki install`) verified by post-install file-and-manifest assertions in the release-gate workflow. (AC #15)

**Roadmap gate:**

16. D8 entry in `wiki/roadmaps/framework-v1.roadmap.md` is `Completed`; predecessor decisions and the bash-renderer plan are moved to `wiki/archive/` (per §2a No-Legacy Discipline) with `Superseded` status and `Superseded By` links. Active index no longer references them. (AC #16)

**No-Legacy Audit gate (per §2a):**

17. `cargo +nightly udeps` reports zero unused dependencies in either crate.
18. `cargo clippy --all-targets --all-features -- -D warnings -D dead_code` passes with no exceptions; any `#[allow(dead_code)]` in the codebase has a one-line comment justifying it (and any such allow is considered a smell to be removed before tag).
19. The CI grep gate added in stage 5.4 passes: no `runtime-specific marker`, `runtime-specific marker`, or `section-end marker` anywhere in the working tree.
20. A second grep gate passes: legacy shell rendering references do not appear in any active wiki document (`wiki/specs/`, `wiki/decisions/`, `wiki/plans/`, `wiki/roadmaps/`, `wiki/index.md`, top-level `CLAUDE.md`/`AGENTS.MD`/`README.md`). They may appear only in `wiki/archive/` (history) and `wiki/log.md` (chronological record).
21. No file in `tools/llm-wiki/src/` or `crates/llm-wiki-schema/src/` parses, accepts, or emits the legacy `legacy tag marker` canonical format. There is no `parse_legacy()` function, no `SchemaVersion` enum, no fallback path. The binary handles only the current schema.
22. No `forward pointer metadata:` field remains on any active wiki document; archived documents may carry `Superseded By` (current) but not `forward pointer metadata:` (transitional).

## 7. Evidence To Record

- `wiki/log.md` entries on stage start, stage milestones, and plan completion.
- Schema snapshot (`tests/snapshots/schema.snap`) committed.
- Per-skill golden-file snapshots (11 of them) committed.
- Per-profile `init` snapshots (5 of them) committed.
- Compat fixture (`tools/llm-wiki/tests/fixtures/wikis/v1/`) committed with hand-coded expected struct file.
- `wiki/checklists/v1-fixture-smoke.checklist.md` committed and referenced from the release-gate workflow.
- A short `review.md §11` entry recording the binary's V1 ship: which acceptance gates passed, which review.md items closed (§9.1, §9.2 by construction; pre-binary renderer's content-loss class by golden-file tests), and any deferred items.

## 8. Wiki Pages To Update When Done

- `wiki/decisions/llm-wiki-binary-distribution.decision.md` — collapse deferred-supersession wording to effective.
- `wiki/decisions/single-source-skills.decision.md` — `Accepted` → `Superseded`.
- `wiki/decisions/framework-path-resolution.decision.md` — `Accepted` → `Superseded`.
- `wiki/decisions/project-local-codex-skills.decision.md` — already `Superseded`; chain unchanged.
- `wiki/plans/single-source-skills.plan.md` — `Active` → `Superseded`.
- `wiki/plans/llm-wiki-binary.plan.md` (this file) — `Draft` → `Active` (on stage 5.1) → `Completed`.
- `wiki/roadmaps/framework-v1.roadmap.md` — D8 → `Completed`.
- `wiki/specs/init-project-skill.spec.md` — binary as scaffolding authority.
- `wiki/specs/knowledge-{ingest,query,research,lint}-skill.spec.md` — canonical source location and renderer updated.
- `wiki/specs/documentation-model.spec.md` — distribution model proven.
- `wiki/index.md` — every status flip reflected; new checklist listed.

## 9. What Closes The Plan

- All sixteen verification gates in §6 pass.
- Stages 5.1–5.13 complete.
- All wiki updates in §8 applied.
- §7 evidence recorded.
- Status of this plan moves `Draft` → `Active` (on stage 5.1) → `Completed` (after stage 5.13).
- A `wiki/log.md` entry confirms D8 is shipped.

## 10. Sequencing Rationale

Stages run in order with these dependencies:

```
5.1 (scaffold + workspace + CI)
    │
    └─→ 5.2 (schema, in llm-wiki-schema)
            │
            └─→ 5.3 (projector logic, unit-tested with inline fixtures)
                    │
                    └─→ 5.4 (canonical migration + golden-file snapshots)
                            │
                            └─→ 5.5 (embed canonical content with compile-time validation)
                                    │
                                    ├─→ 5.6 (build subcommand)
                                    │
                                    ├─→ 5.7 (manifest + install + uninstall)
                                    │
                                    ├─→ 5.8 (init subcommand, including --initial-sources copy)
                                    │
                                    └─→ 5.9 (status + doctor)

5.10 (compat fixtures) — can run in parallel with 5.5–5.9

After all functional stages pass:
    5.11 (init-project skill wrapper migration)
        │
        └─→ 5.12 (cargo-dist + v0.1.0 release) ──→ 5.13 (cleanup + supersession bookkeeping)
```

**Critical reordering note:** stage 5.11 (`init-project` skill wrapper migration) must precede stage 5.12 (release). Skills are embedded at compile time in stage 5.5, so the v0.1.0 binary cannot ship with the wrapper unless the canonical was migrated and the binary recompiled before tagging. Earlier drafts of this plan had the release before the wrapper migration; that ordering would have shipped the legacy markdown-driven skill in v0.1.0.

Tightest-first ordering inside the implementation phase:

- **5.2 → 5.3 → 5.4** is the bug-prevention spine: schema first so the parser is the contract, projector logic second with controlled-input unit tests, canonical migration last so content drift surfaces against committed golden-file snapshots. The pre-binary renderer's content-loss bug would have been caught at stage 5.4 by this order.
- **5.3 deliberately uses inline fixtures** rather than the real `skills/<name>/SKILL.md` files. The migrated canonicals don't exist yet at 5.3; using inline fixtures keeps every stage independently verifiable and keeps the projector logic tests decoupled from canonical-content review.
- **5.7 (manifest)** is the most complex piece; it gets the most test surface but happens after the renderer is solid so install and rendering bugs don't entangle.
- **5.8 (init)** reuses the embed pattern from 5.5, so it lands cleanly after.
- **5.9 (status/doctor)** are reads against stable manifest format; they come last among functional stages.

5.10 (fixtures) can happen in parallel with 5.5–5.9 if a second contributor is available; otherwise it lands before 5.11.

## 11. Implementation Risks

Design risks live in the proposal. These are implementation-specific.

| Risk | Likelihood | Impact | Mitigation |
| --- | --- | --- | --- |
| Stage 5.4 (canonical migration) loses content the same way the pre-binary renderer did | Medium | High | The pass is explicitly a *content-completeness audit*: every difference between today's runtime outputs and the new projector output is reviewed in PR commentary. Golden-file tests then lock the result |
| `HOME` redirection in tests proves portable on macOS but breaks on Linux (or vice versa) | Medium | Medium | CI matrix runs all four platforms from stage 5.1; redirection logic centralized in `paths.rs` so platform-specific behavior is one file, not scattered |
| `cargo-dist` setup is more involved than the proposal implied | Medium | Medium | Reserve a stage-5.12 spike day before tagging; if blocked, manual `cargo build --release` per platform is the V0.1 fallback |
| Manifest schema needs to evolve mid-implementation | Low | Medium | Manifest carries `binary_version`; reading an older manifest is a forward-compat case from day one. A simple `migrate_manifest()` per version transition keeps users from re-installing |
| `clap` derive macros and `serde_yaml` interact badly with `include_str!` content (build-time vs run-time validation boundary unclear) | Low | Medium | Resolve at stage 5.5: `build.rs` runs the parser at compile time; any error is a build failure; this is itself one of the test gates |
| Smoke testing in real Claude Code and Codex sessions reveals discovery quirks not present in golden-file tests | Medium | Medium | Stage 5.11 is the human verification step (manual smoke checks of the migrated `init-project` wrapper in both runtimes); failures here are release blockers and feed back into the projector logic before tagging at 5.12 |
| The IS_EXISTING profile's "leave unrelated files alone" guarantee is hard to test exhaustively | Low | Medium | Stage 5.8 golden-file fixture for IS_EXISTING includes a known set of unrelated files (sample `src/`, `tests/`, `package.json`, `.git/`); test asserts those files' hashes are unchanged after `init` |

## 12. Implementation Decisions (Pre-Stage-5.1)

Per the No-Legacy Discipline (§2a), all four open questions resolve **before** the scaffold step begins. "We'll figure it out during implementation" is how dead code paths get committed.

1. **Crate naming.** Package on crates.io: `llm-wiki-framework`. Binary: `llm-wiki`. So `cargo install llm-wiki-framework` produces an `llm-wiki` executable on `$PATH`. Reserve the name on crates.io before stage 5.1.

2. **Workspace layout.** Workspace `Cargo.toml` at the repo root with two members (`crates/llm-wiki-schema` and `tools/llm-wiki`). Required by stage 5.5: the binary's `build.rs` uses `llm-wiki-schema` as a `[build-dependencies]` entry, which mandates the workspace structure.

3. **`HOME` and XDG path resolution.** Hand-rolled, not `dirs` crate. The manifest is XDG-only (`~/.local/share/llm-wiki/manifest.json`), and the global skill paths are `~/.claude/skills/` and `~/.codex/skills/` — fixed under `HOME` regardless of platform. Hand-rolled is one file (`paths.rs`), test-redirectable, no platform-quirk surprises (`dirs::data_local_dir()` returns `~/Library/Application Support/...` on macOS, which is wrong for our case).

4. **IS_EXISTING profile and `.gitignore`.** Operate on literal directory contents. The `--initial-sources` flag takes explicit paths; the binary copies what is named, nothing more. No `git2` dependency, no implicit `.gitignore` walking. Predictable behavior over convenience. If a user wants to import all unignored files, they invoke their shell's expansion before passing to `--initial-sources`.

These four decisions are part of the plan as accepted; changing one requires revising the plan, not improvising during implementation.
