# Plan: `llm-wiki` Binary Implementation (D8)

- Document Class: Plan
- Status: Draft
- Date: 2026-05-06
- Category: Tooling, framework distribution
- Scope: Implement the `llm-wiki` Rust binary that owns global skill installation, project scaffolding, and skill projection per the accepted D8 deliverable.
- Sources: wiki/proposals/llm-wiki-binary.proposal.md, wiki/decisions/llm-wiki-binary-distribution.decision.md, wiki/roadmaps/framework-v1.roadmap.md (D8), wiki/decisions/single-source-skills.decision.md, wiki/decisions/framework-path-resolution.decision.md, wiki/specs/init-project-skill.spec.md
- Related: wiki/specs/documentation-model.spec.md, wiki/specs/knowledge-ingest-skill.spec.md, wiki/specs/knowledge-query-skill.spec.md, wiki/specs/knowledge-research-skill.spec.md, wiki/specs/knowledge-lint-skill.spec.md, skills/build.sh

## 1. Deliverable

D8: `llm-wiki` Rust binary. Single-command install of framework skills globally for Claude Code and Codex; deterministic project scaffolding; canonical-markdown skill projection with golden-file tests; `cargo-dist` multi-arch distribution.

This plan executes the binary's V1 (D8 v0.1). It does not re-litigate design choices — those are in `wiki/decisions/llm-wiki-binary-distribution.decision.md` and the accepted proposal.

## 2. In Scope

- New Rust crate at `tools/llm-wiki/` (workspace member; the framework repo gains a `Cargo.toml` workspace at the root if not already present).
- All six subcommands: `install`, `build`, `init`, `status`, `doctor`, `uninstall`.
- Embedded canonical skill content (`skills/<name>/SKILL.md`) via `include_str!` after migrating those files from `<!-- CLAUDE --> / <!-- CODEX -->` block markers to the typed schema.
- Embedded project templates (`project_guidelines.template.md`, embedded `CLAUDE.md` template).
- Manifest at `~/.local/share/llm-wiki/manifest.json` with the three-way hash policy from the proposal.
- Golden-file (`insta`) snapshot tests, integration tests with `tempfile::TempDir` and `HOME` redirection, property tests with `proptest`, post-install verification tests.
- Compat fixtures at `tests/fixtures/wikis/v1/` plus the agent-driven `wiki/checklists/v1-fixture-smoke.checklist.md`.
- `cargo-dist` configuration for the four target platforms.
- CI: GitHub Actions running `cargo test`, `cargo clippy -- -D warnings`, `cargo fmt --check`, `cargo insta test --check`, `cargo-llvm-cov` with the 80% gate.
- Update of the `init-project` agent skill (markdown) to be a thin conversational wrapper that shells out to `llm-wiki init --non-interactive`.
- Cleanup pass: remove `skills/build.sh`, drop `<!-- TAG -->` markers from canonical skills, update specs.
- Wiki bookkeeping: flip predecessor decisions and the bash-renderer plan to `Superseded` after the binary is in operation; update `documentation-model.spec.md` to record the distribution model as proven; mark D8 `Completed` on the roadmap.

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

## 4. Crate Layout

```text
tools/llm-wiki/
  Cargo.toml
  build.rs                          # compile-time validation: every embedded skill parses
  src/
    main.rs                         # clap entry point, subcommand dispatch
    cli.rs                          # clap derive structs
    schema/
      mod.rs                        # SkillFrontmatter, SkillBody, parse()
      frontmatter.rs                # YAML deserialization with serde
      body.rs                       # markdown section parsing
      validation.rs                 # field validation, required-section checks
    projector/
      mod.rs                        # trait Projector
      claude.rs                     # ClaudeProjector impl
      codex.rs                      # CodexProjector impl
      idiom.rs                      # invocation-syntax rewriting (slash vs $ namespace)
    embed.rs                        # include_str!-based access to skills/ content
    manifest/
      mod.rs                        # Manifest struct, atomic read/write
      hash.rs                       # SHA-256 helpers
      collision.rs                  # three-way comparison logic
    install.rs                      # install subcommand
    build.rs                        # build subcommand
    init/
      mod.rs                        # init subcommand
      profile.rs                    # ML_AI, QMD, IS_EXISTING resolution
      template.rs                   # conditional-section resolution
      collision.rs                  # framework-artifact detection
    status.rs
    doctor.rs
    uninstall.rs
    paths.rs                        # platform-aware path resolution (HOME-redirectable)
  tests/
    install.rs                      # integration: HOME redirection, idempotency, --force
    init.rs                         # integration: profiles, framework-artifact collision
    uninstall.rs                    # integration: manifest-only deletion
    doctor.rs                       # integration: drift detection, broken-symlink reporting
    build.rs                        # integration: --out, --target
    properties.rs                   # proptest: idempotency, totality
    fixtures/
      wikis/v1/                     # full v1-shaped wiki
      wikis/v1-expected.rs          # hand-coded expected metadata structs
    snapshots/                      # insta snapshots (committed)
```

The crate lives at `tools/llm-wiki/` rather than the repo root so the existing `skills/` directory and wiki content stay where they are. A workspace `Cargo.toml` at the repo root coordinates.

## 5. Steps

Stages run in order. Each stage produces a verifiable artifact and ends in a checkpoint commit.

### 5.1 Crate scaffold and CI (foundation)

1. Add a workspace `Cargo.toml` at the repo root pointing at `tools/llm-wiki`.
2. `cargo new --bin tools/llm-wiki` with name `llm-wiki`.
3. Add dependencies: `clap` (derive), `serde`, `serde_yaml`, `serde_json`, `sha2`, `chrono`, `dirs`, `anyhow`, `thiserror`. Dev: `tempfile`, `insta`, `proptest`, `assert_cmd`, `predicates`.
4. Add `.github/workflows/ci.yml`: matrix over (macos-14 [arm64], macos-13 [x86_64], ubuntu-latest [x86_64], ubuntu-latest-arm). Steps: fmt check, clippy with `-D warnings`, `cargo test`, `cargo insta test --check`, `cargo-llvm-cov` with 80% gate.
5. Add `.cargo-llvm-cov.toml` with platform-specific exclusions placeholder.

**Verification:** CI passes on all four platforms with an empty crate (just `fn main() {}`).

### 5.2 Schema and parser (everything depends on this)

1. Define `SkillFrontmatter` struct mirroring the proposal's canonical schema (name, description, runtimes, operations, arguments, invocation_style, dispatcher_for). All required-vs-optional and field types match the table in `wiki/proposals/llm-wiki-binary.proposal.md`.
2. Define `SkillBody` with the fixed-shape sections (Title, Purpose, Behavior, Invocation, optional Notes).
3. Implement `parse(input: &str) -> Result<SkillDoc, ParseError>` splitting frontmatter (YAML) from body (markdown), validating required sections.
4. Unit tests covering: valid input, malformed YAML, missing required field, unknown runtime value, conflicting fields, missing required body section, schema-snapshot test (`tests/snapshots/schema.snap` locks the accepted field set).

**Verification:** `cargo test schema::` green; schema snapshot committed.

### 5.3 Projector and golden-file tests

1. Define `trait Projector { fn project(&self, doc: &SkillDoc) -> Result<RenderedSkill, ProjectError>; }`.
2. Implement `ClaudeProjector`: rewrites `<skill-name>` invocation patterns to `/skill-name`, applies Claude-flavored frontmatter description, never emits `agents/openai.yaml`.
3. Implement `CodexProjector`: rewrites to `$skill-name` and `$knowledge <op>`, applies Codex-flavored description, emits `agents/openai.yaml` from per-skill template.
4. Golden-file tests under `tests/snapshots/` for all 11 skill × runtime combinations. Source canonical: the migrated `skills/<name>/SKILL.md` files (after stage 5.4).

**Verification:** `cargo insta test --check` green for every skill × runtime; the `knowledge` dispatcher snapshot exists only for Codex.

### 5.4 Migrate canonical skills to clean schema

This is the content-completeness pass that the bash renderer skipped (the bug we just hit).

1. For each `skills/<name>/SKILL.md`, diff today's rendered Claude and Codex outputs (`.claude/skills/<name>/SKILL.md` and `.codex/skills/<name>/SKILL.md`) against the current canonical with `<!-- TAG -->` blocks. Identify all content present in either runtime output but absent from canonical.
2. Rewrite each canonical as: typed YAML frontmatter (per stage 5.2 schema), fixed-shape body sections, no `<!-- TAG -->` blocks. Move runtime-specific deltas into the projector logic, not into the canonical text.
3. Per-skill `codex/openai.yaml` files stay where they are (no schema change needed).
4. Run the projector against each canonical and diff against the current committed `.claude/skills/` and `.codex/skills/` outputs. **Every difference is a deliberate decision recorded in PR review** — this prevents recurrence of the silent-content-loss bug.

**Verification:** projector output for every skill matches a committed expected (golden-file). Any intentional simplification is captured in PR commentary, not silent.

### 5.5 Embed canonical content

1. `src/embed.rs` uses `include_str!("../../../skills/<name>/SKILL.md")` for each skill (or a build-script-generated registry).
2. `build.rs` (the crate's, not the bash one) runs the parser at compile time on every embedded skill; build fails if any skill is missing or malformed.
3. Embedded `project_guidelines.template.md` and a `CLAUDE.md` template via the same pattern.

**Verification:** crate builds cleanly; introducing a malformed canonical breaks the build (negative test in CI).

### 5.6 `build` subcommand (simplest, no manifest)

1. `clap` struct: `Build { target: Option<Target>, out: PathBuf }`. Default `target=Both`, `out=./build`.
2. Iterate embedded canonicals, project via the appropriate projector(s), write to `<out>/.claude/skills/<name>/SKILL.md` and `<out>/.codex/skills/<name>/SKILL.md` (and `agents/openai.yaml`).
3. Replace `bash skills/build.sh` with `llm-wiki build --out .` in the repo's self-dogfooding workflow. Document in `skills/README.md`.
4. Integration tests: `build --out <tempdir>` produces expected files; `--target claude` skips Codex output; `--out .` overwrites without manifest interference.

**Verification:** `bash skills/build.sh` and `llm-wiki build --out <tempdir>` produce byte-identical output for every skill (one-off cross-check before the bash renderer is removed).

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
7. Golden-file tests for all five profiles: baseline, ML_AI, QMD, ML_AI+QMD, IS_EXISTING (the IS_EXISTING profile asserts that pre-existing source code, configs, tests at `<path>` are untouched and only framework files are added).

**Verification:** `cargo insta test --check` green for all five `init` profiles; framework-artifact collision is rejected even when the directory contains unrelated files.

### 5.9 `status` and `doctor`

1. `status`: read manifest, compare to filesystem, report per-skill `OK / Missing / Drifted / Unknown`. Includes binary version, install date, total file count.
2. `doctor`:
   - Detect dangling symlinks under `~/.codex/skills/` pointing at `software_project_management` (legacy from review.md §9.2).
   - Detect framework-shaped paths (`~/.claude/skills/<known-skill>/SKILL.md`, `~/.codex/skills/<known-skill>/SKILL.md`) the manifest does not own.
   - Detect manifest entries with no corresponding file (interrupted install, manual deletion).
   - Suggest concrete commands to fix each: `llm-wiki install --force` for drift, manual symlink removal for legacy residue, etc.

**Verification:** integration tests construct each broken-state scenario and assert `doctor` reports it accurately; review.md §9.2 explicitly closed.

### 5.10 Compat fixtures

1. Build `tests/fixtures/wikis/v1/` from a snapshot of this repo's current `wiki/` (every document type, every status, archive entries, log file).
2. Hand-code expected metadata structs in `tests/fixtures/wikis/v1-expected.rs`: one struct per fixture document with the fields the parser should produce.
3. Tests assert: every fixture document parses; metadata extraction matches expected; `init` invoked with the fixture's recorded answers regenerates a matching `project_guidelines.md`; every skill name referenced in fixture index/log/specs has a current canonical entry under `skills/`.
4. Create `wiki/checklists/v1-fixture-smoke.checklist.md` for the agent-driven operations-level smoke test (ingest a sample raw source against the fixture, query, lint).

**Verification:** all four fixture assertions green; checklist exists and is referenced from the V1 release-gate workflow.

### 5.11 `cargo-dist` and first release

1. `cargo dist init`. Configure for macos-aarch64, macos-x86_64, linux-x86_64, linux-aarch64. Set up the GitHub Releases workflow.
2. Tag `v0.1.0` once stages 5.1–5.10 have green CI.
3. Release-gate workflow runs the post-install verification tests (HOME redirection, manifest cross-check) on every tag.
4. Document install path in repo `README.md`: `curl -L .../llm-wiki-installer.sh | sh && llm-wiki install`.

**Verification:** `v0.1.0` produces signed prebuilt binaries for all four platforms; `cargo install llm-wiki-framework` works; `curl ... | sh` install path completes without manual intervention.

### 5.12 Migrate `init-project` skill to thin wrapper

1. Update `skills/init-project/SKILL.md` (the canonical) to: conduct the six-question intake, validate answers, then shell out to `llm-wiki init --non-interactive --name X --type Y --scale Z ...`.
2. Re-run the projector; commit the regenerated runtime variants.
3. The `init-project` skill remains agent-driven for the conversation; the binary owns file generation. Update `wiki/specs/init-project-skill.spec.md` to reflect this.

**Verification:** manual end-to-end test in Claude Code and Codex: invoke `init-project`, answer six questions, verify a complete and correct project structure is produced. Compare to a `cargo run -- init --non-interactive` baseline with the same answers.

### 5.13 Cleanup and supersession bookkeeping

1. Remove `skills/build.sh`. Self-dogfooding now uses `llm-wiki build --out .`.
2. Remove `<!-- CLAUDE -->`, `<!-- CODEX -->`, `<!-- END -->` markers from all canonical `skills/<name>/SKILL.md` files (already done in stage 5.4 if approached cleanly; this step is the verification pass).
3. Flip `wiki/decisions/single-source-skills.decision.md` and `wiki/decisions/framework-path-resolution.decision.md` to `Superseded` with `Superseded By` linking to `wiki/decisions/llm-wiki-binary-distribution.decision.md`.
4. Flip `wiki/plans/single-source-skills.plan.md` to `Superseded`.
5. Update `wiki/decisions/llm-wiki-binary-distribution.decision.md`: change `Will Supersede On D8 Completion:` to `Supersedes:`; collapse the Consequences section's split into a single now-effective list.
6. Update `wiki/specs/init-project-skill.spec.md`: binary is the authority for scaffolding; agent retains intake.
7. Update `wiki/specs/knowledge-*-skill.spec.md` (five files): canonical source is `skills/<name>/SKILL.md`; rendering is via `llm-wiki build` / `llm-wiki install`.
8. Update `wiki/specs/documentation-model.spec.md`: distribution model recorded as proven.
9. Update `wiki/roadmaps/framework-v1.roadmap.md`: D8 status `Draft` → `Active` (on stage 5.1) → `Completed` (after all gates pass).
10. Update `wiki/index.md` to reflect all status flips.
11. Append `wiki/log.md` entry per the framework's logging convention.

**Verification:** `wiki/index.md` and `wiki/log.md` are coherent; no stale "Successor" annotations remain on documents that are now Superseded; no stale "Will Supersede On D8 Completion" wording on the new decision.

## 6. Verification Gates

Each gate maps to one or more acceptance criteria from the proposal (cited in parens).

**Functional gates:**

1. `llm-wiki install` against redirected `HOME` writes correct files with manifest entries; second run is a no-op. (AC #1)
2. Default `install` refuses user-authored collisions; `--force` backs up to `<path>.bak.<UTC-ISO8601>`; repeated `--force` runs produce distinct backups. (AC #2)
3. `llm-wiki init <path>` Create mode produces correct structure for all five profiles via golden-file fixtures. Update mode invocation against a path containing any framework artifact returns a clear error. (AC #3)
4. Canonical skill source contains no `<!-- CLAUDE -->` / `<!-- CODEX -->` blocks; per-runtime variation is fully in the projector. (AC #4)
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

16. D8 entry in `wiki/roadmaps/framework-v1.roadmap.md` is `Completed`; predecessor decisions and the bash-renderer plan are `Superseded` with `Superseded By` links. (AC #16)

## 7. Evidence To Record

- `wiki/log.md` entries on stage start, stage milestones, and plan completion.
- Schema snapshot (`tests/snapshots/schema.snap`) committed.
- Per-skill golden-file snapshots (11 of them) committed.
- Per-profile `init` snapshots (5 of them) committed.
- Compat fixture (`tests/fixtures/wikis/v1/`) committed with hand-coded expected struct file.
- `wiki/checklists/v1-fixture-smoke.checklist.md` committed and referenced from the release-gate workflow.
- A short `review.md §11` entry recording the binary's V1 ship: which acceptance gates passed, which review.md items closed (§9.1, §9.2 by construction; bash renderer's content-loss class by golden-file tests), and any deferred items.

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
5.1 (scaffold) ──┬─→ 5.2 (schema) ──→ 5.3 (projector) ──→ 5.4 (canonical migration) ──→ 5.5 (embed)
                 │                                                                          │
                 │                                                                          ├─→ 5.6 (build)
                 │                                                                          │
                 │                                                                          ├─→ 5.7 (install/uninstall)
                 │                                                                          │
                 │                                                                          ├─→ 5.8 (init)
                 │                                                                          │
                 │                                                                          └─→ 5.9 (status/doctor)
                 │
                 └─→ 5.10 (fixtures) ──┐
                                       ├─→ 5.11 (cargo-dist + v0.1.0) ──→ 5.12 (init-project skill migration) ──→ 5.13 (cleanup)
                                       │
                                       (all functional stages must be green)
```

Tightest-first ordering inside the implementation phase:

- **5.2 → 5.3 → 5.4** is the bug-prevention spine: schema first so the parser is the contract, projector second so the renderer has tests, canonical migration last so content drift surfaces against committed snapshots. The bash renderer's content-loss bug would have been caught at stage 5.4 by this order.
- **5.7 (manifest)** is the most complex piece; it gets the most test surface but happens after the renderer is solid so install and rendering bugs don't entangle.
- **5.8 (init)** reuses the embed pattern from 5.5, so it lands cleanly after.
- **5.9 (status/doctor)** are reads against stable manifest format; they come last among functional stages.

5.10 (fixtures) can happen in parallel with 5.5–5.9 if a second contributor is available; otherwise it lands before 5.11.

## 11. Implementation Risks

Design risks live in the proposal. These are implementation-specific.

| Risk | Likelihood | Impact | Mitigation |
| --- | --- | --- | --- |
| Stage 5.4 (canonical migration) loses content the same way the bash renderer did | Medium | High | The pass is explicitly a *content-completeness audit*: every difference between today's runtime outputs and the new projector output is reviewed in PR commentary. Golden-file tests then lock the result |
| `HOME` redirection in tests proves portable on macOS but breaks on Linux (or vice versa) | Medium | Medium | CI matrix runs all four platforms from stage 5.1; redirection logic centralized in `paths.rs` so platform-specific behavior is one file, not scattered |
| `cargo-dist` setup is more involved than the proposal implied | Medium | Medium | Reserve a stage-5.11 spike day before tagging; if blocked, manual `cargo build --release` per platform is the V0.1 fallback |
| Manifest schema needs to evolve mid-implementation | Low | Medium | Manifest carries `binary_version`; reading an older manifest is a forward-compat case from day one. A simple `migrate_manifest()` per version transition keeps users from re-installing |
| `clap` derive macros and `serde_yaml` interact badly with `include_str!` content (build-time vs run-time validation boundary unclear) | Low | Medium | Resolve at stage 5.5: `build.rs` runs the parser at compile time; any error is a build failure; this is itself one of the test gates |
| Smoke testing in real Claude Code and Codex sessions reveals discovery quirks not present in golden-file tests | Medium | Medium | Stage 5.12 is the human verification step; failures here are roadmap blockers and feed back into the projector logic |
| The IS_EXISTING profile's "leave unrelated files alone" guarantee is hard to test exhaustively | Low | Medium | Stage 5.8 golden-file fixture for IS_EXISTING includes a known set of unrelated files (sample `src/`, `tests/`, `package.json`, `.git/`); test asserts those files' hashes are unchanged after `init` |

## 12. Open Implementation Questions

To resolve before or during stage 5.1; not blocking acceptance of this plan.

- Crate name for `cargo install`: `llm-wiki-framework`, `llm-wiki`, `llm-wiki-cli`? Reserve on crates.io before tagging.
- Workspace `Cargo.toml` at the repo root vs. nested `tools/llm-wiki/Cargo.toml` standalone: workspace is cleaner for future tooling crates; standalone is simpler today.
- `dirs` crate vs hand-rolled `HOME` resolution: `dirs::data_local_dir()` is convenient but has macOS-vs-Linux quirks (`~/Library/Application Support/...` vs `~/.local/share/...`). The manifest is XDG-only per the proposal, so hand-rolled may be cleaner.
- Use `git2` for the IS_EXISTING profile to detect-and-respect a `.gitignore`, or just operate on the literal directory contents? V1 likely the latter.

These are sequencing details, not design forks. The plan can begin without them resolved.
