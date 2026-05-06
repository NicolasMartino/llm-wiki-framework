# LLM Wiki Framework Binary

- Document Class: Proposal
- Status: Accepted
- Date: 2026-05-06
- Promoted To: wiki/decisions/llm-wiki-binary-distribution.decision.md, wiki/roadmaps/framework-v1.roadmap.md (D8)
- Category: Tooling, framework distribution
- Scope: Replace the current symlink-and-skill-copy distribution model with a single Rust binary (`llm-wiki`) that installs framework skills globally, scaffolds new projects, and projects canonical skill definitions into per-runtime variants.
- Sources: review.md §9-10, wiki/log.md (line-loss audit, 2026-05-06), wiki/plans/single-source-skills.plan.md, wiki/decisions/single-source-skills.decision.md, wiki/decisions/framework-path-resolution.decision.md, wiki/decisions/project-local-codex-skills.decision.md, wiki/specs/init-project-skill.spec.md, wiki/roadmaps/framework-v1.roadmap.md
- Related: wiki/specs/documentation-model.spec.md, wiki/specs/init-project-skill.spec.md, all `wiki/specs/knowledge-*-skill.spec.md`

## Question

Should the framework be packaged as a single Rust binary (`llm-wiki`) that owns global skill installation, project scaffolding, and skill rendering — replacing the current model of a working-tree repo plus symlinks plus markdown-interpreted scaffolding skills?

## Proposal

Yes. Build `llm-wiki` as a Rust binary with embedded templates and a `clap`-driven subcommand surface. It becomes the single deployment unit of the framework. Users download one binary, run `llm-wiki install`, and both Claude Code and Codex have working framework skills globally. New projects come from `llm-wiki init <path>`. Skill content lives in canonical markdown under `skills/` in this repo, embedded into the binary at compile time, and projected into per-runtime variants by typed Rust code with golden-file tests.

This proposal supersedes the bash renderer in `wiki/plans/single-source-skills.plan.md` and the global-symlink installation model in `wiki/decisions/project-local-codex-skills.decision.md` and `wiki/decisions/framework-path-resolution.decision.md`.

## Roadmap Position

This proposal **intentionally expands V1 scope**. The current `framework-v1.roadmap.md:246` D7 deliverable explicitly excludes "GUI or CLI tooling" and treats framework distribution as future work. Promoting this proposal therefore requires one of two roadmap moves:

- **(a) Extend D7** to include CLI tooling and binary distribution, removing the "GUI or CLI tooling" exclusion. Defensible because D7's stated proof is "two distinct projects bootstrapped by agents using the framework" and a binary makes that proof cheaper to produce.
- **(b) Add D8** as a new deliverable: "Distribution Tooling — `llm-wiki` binary owns global install and project scaffolding." D7 stays as documented (templates, manual bootstrap procedure); D8 is the operational form. This is the cleaner record because it preserves the original D7 framing and adds the binary as a separate, named unit of work.

This proposal recommends **(b)**. If accepted, the roadmap is updated to add D8 with the acceptance criteria and proof gates from this proposal as its definition. D7 remains as written; D8 makes the same self-replicating outcome much faster.

## Why

The current model has three fragilities the binary eliminates by construction:

1. **Broken global symlinks** (review.md §9.2). Symlinks point into a working tree at a specific absolute path. Renaming or moving the repo dangles them silently. A binary that writes files directly to `~/.claude/skills/` and `~/.codex/skills/` removes the indirection.
2. **Per-runtime skill drift.** Canonical markdown with `<!-- CLAUDE -->` / `<!-- CODEX -->` block markers (the bash renderer's approach) is just text-block deletion and pollutes the source. A typed projector reads clean canonical markdown and emits per-runtime idioms, separating skill semantics from runtime expression. The projector eliminates *cross-runtime drift on the same canonical content*; it does **not** eliminate authoring drift in the canonical itself — golden-file snapshot tests are the discipline against that second class.
3. **LLM-interpreted scaffolding silently drops content.** Today's `init-project` skill is markdown the agent re-interprets each invocation. The bash-renderer audit (recorded in `wiki/log.md` 2026-05-06 entry, with reviewer commentary in `review.md` §10) measured concrete losses on the first canonical-source draft: `init-project` Claude variant 369 → 155 lines, `knowledge-research` Codex variant 200 → 124 lines. Moving file generation to deterministic Rust with embedded templates and golden-file fixtures eliminates that drift and makes regressions visible as a CI diff.

Distributing a binary also makes "self-replicating" cheaper to demonstrate. D7's proof — "two distinct projects bootstrapped by agents using the framework, both with navigable wikis, neither requiring human edits to `wiki/`" — is unchanged; the binary does not replace any of that work. What it changes is the *setup step*: `llm-wiki init <domainA> && llm-wiki init <domainB>` becomes the precondition for two agents to begin their D7 bootstraps, instead of today's manual symlink-and-copy dance per project. Scaffolding is not proof; running the framework end-to-end against the scaffolded projects is.

## Subcommand Surface

```text
llm-wiki install   [--force]
llm-wiki build     [--target claude|codex|both] [--out <dir>]
llm-wiki init      <path> [--name X --type Y --scale Z ...] [--non-interactive]
llm-wiki status
llm-wiki doctor
llm-wiki uninstall
llm-wiki --version
llm-wiki --help
```

**Install vs build, by design.** `install` is global-only and writes a manifest. `build` is for local rendering, snapshot tests, and self-dogfooding — it never writes a manifest and has no collision policy. This split is deliberate: an earlier draft of this proposal had `install --env local`, which would have classified this repo's committed `.claude/skills/` and `.codex/skills/` files as user-authored collisions and refused. Splitting the subcommands removes that edge case entirely. Self-dogfooding uses `build --out .` to render into the working tree; it does not use `install`.

### `install`

Global only. Renders embedded canonical skills and writes them to `~/.claude/skills/<name>/SKILL.md` and `~/.codex/skills/<name>/SKILL.md` (plus `~/.codex/skills/<name>/agents/openai.yaml` where applicable). Idempotent: re-running heals broken state and updates to the binary's bundled version. Uses an explicit ownership manifest to make `uninstall` and collision detection precise (see "Install State And Manifest" below).

- `--force`: overwrite even files the manifest does not own (i.e., user-authored content at the same path). Default refuses and reports the collision.
- Default behavior on framework-owned files: compare manifest-recorded hash against current file; skip if identical, overwrite if changed.

### `build`

Renders canonical skills to a chosen output directory without touching global state and without writing a manifest. For inspection, CI snapshot tests, authoring previews, and this repo's self-dogfooding.

- `--target claude|codex|both` (default `both`): which projection(s) to render.
- `--out <dir>` (default `./build`): output directory. For self-dogfooding this repo, `--out .` writes into `./.claude/skills/` and `./.codex/skills/` directly.

`build` has no collision policy because it has no concept of ownership — it always overwrites. That is appropriate for a render-and-inspect tool but unsafe for global state, which is why `install` is a separate subcommand.

### `init`

Scaffolds a new project at `<path>`. **Create mode only in V1** — `Update` mode is explicitly out of scope and remains agent-owned (see Non-Goals).

Create mode produces: `raw/`, `wiki/` (with `index.md` and `log.md`), `CLAUDE.md`, `project_guidelines.md` from embedded templates with conditional-section resolution per project profile (ML_AI, QMD, IS_EXISTING).

- Interactive mode (default): prompts for the six questions documented in `wiki/specs/init-project-skill.spec.md:44-51` (name, description, type, new/existing, scale, optional initial sources).
- `--non-interactive`: requires all answer flags. Used by the `init-project` skill (which still owns the conversational intake) and by CI.

The `init-project` skill remains as a thin wrapper: agent conducts the question flow, validates answers, then shells out to `llm-wiki init --non-interactive ...`. Agent owns judgment and conversation; binary owns deterministic file generation.

**Update mode** (compare an existing project against the latest template, propose and apply changes — see `wiki/specs/init-project-skill.spec.md:21-23`) involves judgment calls about which template diffs to apply to a project that may have intentional customizations. That kind of judgment belongs with the agent. The agent retains the existing `init-project` skill prose for Update flows.

**The binary's `init` collision check is on framework-owned files only, not on the directory's emptiness.** This preserves the IS_EXISTING profile (adding the framework to an existing codebase). Specifically:

- If `<path>` does not exist: create it, run Create mode.
- If `<path>` exists but contains none of the framework artifacts (no `wiki/`, no `raw/`, no `CLAUDE.md`, no `project_guidelines.md`): proceed with Create mode. Other content (source code, configs, tests) is left untouched. This is the IS_EXISTING happy path.
- If `<path>` contains **any** framework artifact: refuse with a clear error naming which artifact was found, telling the user to use the agent-driven Update flow.
- `--force` is **not** a Create-over-existing-framework escape hatch in V1; framework-on-framework collision always routes to the Update flow because the agent owns that judgment.

## Install State And Manifest

The binary maintains an ownership manifest so `install`, `uninstall`, and `doctor` can reason about state precisely.

**Location**: `~/.local/share/llm-wiki/manifest.json` (XDG-compliant). Only `install` reads or writes it. `build` never touches it.

**Schema** (one entry per file the binary has written):

```json
{
  "binary_version": "0.1.0",
  "installed_at": "2026-05-06T12:34:56Z",
  "files": [
    {
      "path": "/Users/x/.claude/skills/knowledge-query/SKILL.md",
      "skill": "knowledge-query",
      "runtime": "claude",
      "kind": "skill",
      "sha256": "a3f1...",
      "installed_by_version": "0.1.0"
    },
    {
      "path": "/Users/x/.codex/skills/knowledge-query/agents/openai.yaml",
      "skill": "knowledge-query",
      "runtime": "codex",
      "kind": "runtime-config",
      "sha256": "b7e2...",
      "installed_by_version": "0.1.0"
    }
  ]
}
```

**Collision policy** for `install`. The decision is a **three-way hash comparison**: `current` (file on disk), `manifest` (what was installed last time), and `bundled` (what this binary wants to install). This distinguishes "no-op" from "upgrade" from "user has modified the framework file" — cases the earlier two-way comparison conflated.

| `current` vs `manifest` | `manifest` vs `bundled` | Interpretation | Default behavior | `--force` behavior |
| --- | --- | --- | --- | --- |
| Path absent (no current) | n/a (no manifest entry) | Fresh install | Write; record in manifest | Write; record |
| Path absent | Manifest entry exists | Recovery: file deleted externally | Restore from bundled; refresh manifest | Same |
| Match | Match | Already up to date | Skip (no-op) | Skip |
| Match | Differ | **Upgrade**: framework version moved on, file untouched by user | Overwrite with bundled; update manifest hash | Same |
| Differ | Match | User edited the framework's installed file | Refuse; report drift; exit non-zero | Back up to `<path>.bak.<UTC-ISO8601>` (e.g. `<path>.bak.20260506T123456Z`); overwrite; update manifest |
| Differ | Differ | User edited **and** framework version moved on | Refuse; report both drifts; exit non-zero | Same backup-and-overwrite as above |
| Path present, **not in manifest** (user-authored at framework path) | n/a | Collision with non-framework content | Refuse; report path; exit non-zero | Back up to `<path>.bak.<UTC-ISO8601>`; overwrite; record |
| Path is a symlink (e.g., dangling pre-binary symlink) | n/a | Pre-binary residue | Refuse; suggest `doctor` | Replace symlink with file; record |

The timestamped backup suffix means repeated `--force` runs never overwrite an earlier backup.

`uninstall` deletes only files listed in the manifest, in reverse order, then removes the manifest. Files written outside the manifest (user content, third-party skills) are never touched. After `uninstall`, the manifest is empty / removed; the directories themselves are left in place if they contain other content.

`doctor` reads the manifest plus the filesystem and reports: missing files (manifest entry but no file), drift (file exists but hash mismatches manifest), unknown framework-shaped files (skill-named paths the manifest does not own), and any pre-binary symlinks pointing at the legacy `software_project_management` location.

### `status`

Reports what is installed where, which skills are present, version of the binary, and whether installed skill files match the binary's bundled version (drift detection).

### `doctor`

Diagnoses common breakage: missing skill files, drift between installed and bundled versions, stale symlinks left over from the pre-binary model, broken `~/.codex/skills/` entries pointing at the old `software_project_management` path.

### `uninstall`

Removes skill files listed in the manifest, in reverse install order, then deletes the manifest. Files not in the manifest are never touched. Global-only, matching `install`.

## Skill Content And Rendering Model

Canonical skills live under `skills/<name>/SKILL.md` with clean readable markdown — no `<!-- CLAUDE -->` block markers. Frontmatter declares projection-relevant metadata; body is shared prose. The Rust projector understands runtime idioms and emits per-runtime variants.

Skill content is embedded into the binary at compile time via `include_str!`. Every release of the binary is a coherent self-contained snapshot. No runtime skill fetching, no skill-content service. (Distribution itself — installer download, `cargo install`, Homebrew — is network-dependent, but only at acquisition time. Once the binary is on disk, `install` / `build` / `init` all run fully offline.)

### Canonical Schema

Frontmatter is YAML and **typed** by the projector — unknown fields are an error, missing required fields are an error.

| Field | Required | Type | Purpose |
| --- | --- | --- | --- |
| `name` | yes | string | Skill identifier; matches directory name |
| `description` | yes | string | Shared description; projector may runtime-specialize per the rules below |
| `runtimes` | yes | list[enum] | Subset of `[claude, codex]`; declares which projections to emit |
| `operations` | no | list[string] | Logical operations the skill supports (`init`, `query`, `ingest`, `research`, `lint`, `dispatch`) |
| `arguments` | no | list[object] | Each `{name, required, description}`; used by projector to render invocation patterns |
| `invocation_style` | no | enum | `slash`, `namespace`, or `dispatch`; defaults inferred from runtime |
| `dispatcher_for` | no | list[string] | Codex-only; lists subcommands this skill routes |

Body sections (markdown after frontmatter) follow a fixed shape so the projector can transform structurally rather than textually:

| Section heading | Purpose | Required |
| --- | --- | --- |
| `# <Title>` | Skill display name | yes |
| `## Purpose` | One-paragraph what/why | yes |
| `## Behavior` | Numbered steps the agent follows | yes |
| `## Invocation` | Examples; projector rewrites per runtime idiom | yes |
| `## Notes` | Free prose | no |

**Projection rules**:

- `## Invocation` examples are written in a runtime-neutral form (`<skill-name> <args>`) and the projector emits `/<skill-name> <args>` for Claude, `$<skill-name> <args>` plus `$knowledge <op> <args>` for Codex.
- `description` may be templated with `{runtime}` and `{verb}` tokens; projector substitutes per target.
- A skill with `runtimes: [codex]` (e.g., the `knowledge` dispatcher) is projected only to Codex.
- The projector emits `agents/openai.yaml` for every Codex projection from a per-skill template; the YAML's only variable is the skill name.

The implementation plan will lock the schema with `serde` deriving and `cargo doc`. Schema changes are themselves snapshot-locked: a `tests/snapshots/schema.snap` records the accepted field set so additions or removals are visible in PR review.

Golden-file tests using `insta` lock the projected output. Any change to canonical content or projector logic produces a reviewable diff in CI before merge.

## Testing Strategy

Tests are not an afterthought. The drift the bash renderer experienced — measured concretely as `init-project` Claude variant 369 → 155 lines and `knowledge-research` Codex variant 200 → 124 lines on the first canonical-source pass — would have been caught by golden-file tests in any language. The V1 binary commits to that test category as a hard gate plus four others.

### Test Categories

**Unit tests** (`cargo test`, alongside source):

- Markdown frontmatter parsing: valid inputs, malformed YAML, missing required fields, unknown runtimes, conflicting fields.
- Projector trait implementations: Claude variant produces expected idiom (slash-command syntax, description-match frontmatter); Codex variant produces expected idiom (`$namespace` syntax, `agents/openai.yaml` emission).
- Embedded template loader: every canonical skill loads at compile time; missing or malformed canonical fails the build, not the run.
- Path resolution: `install` writes to `~/.claude/skills/` and `~/.codex/skills/` derived from `HOME` (not `dirs::home_dir` defaults that vary across platforms); `build --out <dir>` writes exactly under `<dir>` and never escapes it.
- Project-profile resolution for `init`: ML_AI, QMD, IS_EXISTING flags produce correct conditional-section output.

**Integration tests** (`tests/`):

- `install` redirects `HOME` to a `tempfile::TempDir` and verifies expected files appear under the redirected `~/.claude/skills/` and `~/.codex/skills/` paths with manifest entries to match.
- `build --out <tempdir>` produces expected files without touching `HOME` or writing a manifest.
- `install` is idempotent: run twice, second run produces no filesystem changes.
- `install --force` overwrites; default `install` skips identical files (hash check).
- `uninstall` removes exactly what `install` wrote and nothing else (verified by manifest).
- `init <path>` produces correct project structure for every supported flag combination.
- `init --non-interactive` rejects missing required flags with a clear error.
- `doctor` correctly diagnoses common breakage: missing skill files, drift between installed and bundled versions, dangling pre-binary symlinks.
- `status` reports drift accurately when installed files are tampered with.

**Golden-file tests** (`insta` snapshots, committed under `tests/snapshots/`):

- Every canonical skill produces a snapshot for each target runtime — currently 6 skills × 2 runtimes minus the Codex-only `knowledge` dispatcher = 11 snapshots.
- Every `init` profile produces a snapshot for the generated `project_guidelines.md` and `CLAUDE.md`. Profiles to lock: baseline, ML_AI, QMD, ML_AI+QMD, IS_EXISTING (no scaffolding).
- Snapshots are reviewed on every change — `cargo insta review` is part of the contributor workflow. `cargo insta test --check` runs in CI and fails on undeclared diff.

**Property tests** (`proptest`, where genuinely useful):

- Idempotency property: for any sequence of N consecutive `install` calls, final filesystem state equals state after one call.
- Renderer totality: any valid canonical (within the schema) produces output without panic. Generators emit valid canonical inputs; the test asserts the projector returns `Ok` for all of them.

**Post-install verification tests** (replacing the earlier handwave about "claude --version discovers skills" — neither runtime exposes a stable "list installed skills" command, so the proof is filesystem-based):

- After `install` against a redirected `HOME`, every expected skill file exists at the expected path with the expected SHA-256 (read from manifest).
- The manifest is consistent with the filesystem: every manifest entry corresponds to a real file; no orphan files written by `install` are missing from the manifest.
- Optional manual smoke test, documented but not automated for V1: in a real Claude Code or Codex session after install, ask the agent to invoke each skill by name; record the trace in the release-gate checklist. Automating this requires runtime cooperation that does not exist today; revisit when either Claude Code or Codex adds a stable skill-listing API.

### Coverage Target

- **80% line coverage** measured by `cargo-llvm-cov` (preferred over `tarpaulin` for stability and accuracy).
- Coverage is a floor, not a goal. The proposal does not endorse "raise coverage" as standalone work. The categories above are what matters; coverage just confirms no major path is unexercised.
- Files with unavoidably low coverage (e.g., platform-specific `cfg(target_os)` blocks) are listed in `.cargo-llvm-cov.toml` exclusions with a one-line justification each.
- CI fails the build if coverage drops below 80% on a default-feature run.

### CI Matrix

- GitHub Actions runs on every PR: `cargo test`, `cargo clippy -- -D warnings`, `cargo fmt --check`, `cargo insta test --check`, `cargo-llvm-cov` with the 80% gate.
- Test matrix: macOS arm64, macOS x86_64, Linux x86_64, Linux arm64. Same matrix used by `cargo-dist` for releases.
- Tagged-release workflow additionally runs the post-install verification tests with `HOME` redirection and manifest cross-checks.

### Out Of Scope For V1 Testing

- Fuzzing the markdown parser. The input space is tightly constrained (we author every canonical input); fuzzing here is theater. Revisit if user-authored skills become a feature.
- Mutation testing (`cargo-mutants`). High-value but premature for a V1 of this scope.
- Performance benchmarks. Render time is ms-scale at current skill counts; benchmark when there's a regression to investigate.

## Distribution

- `cargo-dist` produces multi-arch prebuilt binaries (macOS arm64, macOS x86_64, Linux x86_64, Linux arm64) attached to GitHub Releases.
- `cargo install llm-wiki-framework` works for users with a Rust toolchain.
- Optional Homebrew tap for macOS users when the project is ready for broader reach.

End-user install path:

```bash
curl -L https://github.com/.../releases/latest/download/llm-wiki-installer.sh | sh
llm-wiki install
```

After that single command, both Claude Code and Codex have working framework skills globally, with no working-tree, no symlinks, and no manual configuration.

## Versioning Model

The framework follows the `git` model: one global install at a time, framework guarantees backward-compat with older project shapes. Projects do not pin or know about framework versions. v2 must read v1-shaped wikis; v3 must read v1- and v2-shaped wikis. The discipline lives in the framework's compatibility promise, not in per-project metadata.

### "Wiki Shape" Defined

The framework's compat promise covers a precise set of artifacts. Anything outside this list may evolve freely between binary versions:

1. **Document type filename suffixes**: `.spec.md`, `.decision.md`, `.proposal.md`, `.roadmap.md`, `.plan.md`, `.experiment.md`, `.eval.md`, `.checklist.md`, `.reference.md`. Adding a suffix is non-breaking; removing or renaming a suffix is breaking.
2. **Metadata block fields** declared in `wiki/specs/documentation-model.spec.md`: `Document Class`, `Status`, `Date`, `Category`, `Scope`, `Sources` (required); `Owner`, `Supersedes`, `Superseded By`, `Related`, `Promotion Target` (optional). The framework must continue to read documents missing optional fields.
3. **Folder layout** under `wiki/`: `specs/`, `decisions/`, `proposals/`, `roadmaps/`, `plans/`, `experiments/`, `evals/`, `checklists/`, `references/`, `archive/`, plus root `index.md` and `log.md`.
4. **Status vocabulary** declared in the documentation-model spec.
5. **Three-layer architecture invariant**: `raw/` (immutable, human-curated), `wiki/` (agent-owned), `CLAUDE.md` + `project_guidelines.md` (schema). Any change here is breaking by definition.

### Compat Fixtures

The repository commits a `tests/fixtures/wikis/v1/` snapshot of a complete v1-shaped wiki (all current document types, every status value, archive entries, log file). The compat promise is bounded by what the binary actually does — it does **not** implement ingest, query, or lint (those remain agent-driven). Future binary versions verify only what the binary's own code path can verify against the fixture:

- **Parseability**: every document parses with the binary's typed metadata schema; no fixture file fails parsing.
- **Metadata extraction**: reading a v1-shaped document returns parsed metadata identical to a hand-coded expected struct (one struct per document type, committed alongside the fixture).
- **Template compatibility**: `init` invoked with the same answers that produced the fixture's `project_guidelines.md` regenerates a file equivalent to the fixture's (modulo intentional template improvements, which require a deliberate fixture refresh and a release note).
- **Skill availability**: every skill name referenced inside the fixture's `wiki/index.md`, `wiki/log.md`, and skill specs has a current canonical entry under `skills/`. A skill rename or removal that breaks this assertion is a deliberate breaking change requiring a `migrate` plan.
- **No round-trip claim**: the binary does not write into `wiki/` content (only `init` scaffolding writes wiki files, and only at project creation), so round-trip identity is not a meaningful gate. Removed from the fixture contract.

Operations-level compatibility (ingest/query/lint succeed against a v1 wiki) is a separate concern owned by the agent and the installed skills. It is verified by an **agent-driven smoke-test checklist** committed at `wiki/checklists/v1-fixture-smoke.checklist.md`, run manually as part of release gating against the fixture wiki. The checklist outcome is recorded; failures are release blockers.

Adding a new fixture for v2 (etc.) is a release-gate requirement when wiki shape evolves.

`llm-wiki init` writes a new project with the binary's current shape. Once written, the project's wiki is its data; the framework reads and writes against it regardless of which version originally created it.

If a deliberately-breaking change to wiki shape ever becomes necessary, the response is one of: a `migrate` subcommand that transforms older fixtures forward; versioned install dirs (`~/.codex/skills/llm-wiki-v2/`); or per-project framework pinning. Not solved now; the compat promise plus committed fixtures should hold for the foreseeable future.

## Self-Referential Dev Workflow

This repo is the framework's own development environment and uses the framework to manage itself. The `build` subcommand preserves that without entangling local dev with the global install path: framework developers run `llm-wiki build --out .` to render skills into this repo's own `.claude/skills/` and `.codex/skills/` directly, with no manifest written and no global state touched. `llm-wiki install` is reserved for promoting an installed binary to the user-wide skill set; framework developers do not run it as part of normal editing.

This repo's `.claude/skills/` and `.codex/skills/` directories remain in git for first-clone usability and self-dogfooding. `skills/` (canonical) is the source of truth and is also committed. CI runs `llm-wiki build --out <tempdir>` and diffs against the committed renders to catch un-rebuilt edits.

## Non-Goals

- **Config files.** No `~/.config/llm-wiki/config.toml` in V1. The subcommand surface is small enough that flags cover every branching need. Revisit if patterns emerge.
- **Plugin system.** Skills are part of the framework; users do not author runtime-loadable plugins. If a user wants a custom skill, they fork or contribute upstream.
- **Version pinning per project.** See "Versioning Model" — backward-compat plus committed compat fixtures replace it.
- **Network fetching of skills.** All skill content is embedded at compile time. Distribution itself (download, `cargo install`, Homebrew) is network-dependent; framework operations after install are not.
- **Runtime support beyond Claude and Codex.** Adding Cursor, Aider, Amp later is a small change to the projector — but explicitly out of scope for V1.
- **GUI / web UI.** CLI only.
- **Auto-update.** `llm-wiki self-update` can come later. For V1, users update via their installer (Homebrew, GitHub Releases download, `cargo install`).
- **Multi-project skill variants.** Skills are global and identical across all projects on a machine.
- **Wiki content tooling beyond ingest/query/lint.** The binary owns scaffolding and skill installation. Wiki operations remain agent-driven through the installed skills.
- **`init` Update mode.** Comparing an existing project against the latest template and selectively applying diffs requires judgment about which deltas to accept; that belongs with the agent. The binary's `init` is Create-only; the agent retains the existing `init-project` Update flow until evidence shows the binary should own it. The binary refuses to run `init` against a path that already contains a `wiki/` directory, with an error pointing the user to the agent-driven update.
- **Automated runtime skill-discovery tests.** Neither Claude Code nor Codex exposes a stable "list installed skills" command; the V1 proof is filesystem-and-manifest based. Revisit when either runtime adds such an API.

## Risks

| Risk | Likelihood | Impact | Mitigation |
| --- | --- | --- | --- |
| Multi-arch release pipeline becomes a maintenance burden | Medium | Medium | `cargo-dist` automates this; the burden is one-time setup, not per-release work |
| Embedded templates become stale relative to canonical sources | Low | High | Build-time check: binary fails to compile if `skills/` is missing or malformed; golden-file (`insta`) snapshot tests catch projection drift before merge |
| Tests pass but generated content silently regresses (the bug we just hit) | Medium | High | Golden-file snapshots are the explicit defense — every projection has a committed expected output; `cargo insta review` forces human acknowledgment of any diff |
| Users on niche platforms (Windows, BSD, exotic ARM) can't install | Medium | Low | `cargo install` covers them; Windows can be added to `cargo-dist` matrix when needed |
| The agent-vs-binary split for `init-project` confuses contributors | Low | Low | Document explicitly: agent owns intake, binary owns generation |
| Backward-compat promise becomes constraining for framework evolution | Medium | Medium | Reserve a `migrate` subcommand for future use; only commit to backward-compat for `wiki/` shape, not for skill internals |
| First implementation underestimates work and stalls the D8 deliverable | High | High | The implementation plan (separate document, accepted after this proposal) sequences work tightest-first: schema + projector + golden-file tests, then `install` + manifest, then `init`, then `doctor`/`uninstall`. Any subcommand can slip past D8 v0.1 if necessary, but `init` is non-negotiable for V1 because without it the agent-vs-binary split is incomplete and the existing `init-project` skill remains LLM-interpreted |
| User has authored their own skill at one of the framework's reserved skill names | Medium | Medium | Default `install` refuses; reports the collision. `--force` backs up to `<path>.bak.<UTC-ISO8601>` (timestamped, never overwrites earlier backups) before overwriting. Documented behavior, not silent overwrite |
| Manifest gets out of sync with filesystem (manual edits, partial install crash) | Medium | Medium | `doctor` detects and reports. `install --force` is the recovery path. Manifest writes are atomic (temp file + rename) so crashes leave one consistent state, not torn |

## Acceptance Criteria

The proposal is implementable and worth implementing if all of these hold for V1 (D8 v0.1):

**Functional:**

1. `llm-wiki install` writes correct skill files to `~/.claude/skills/` and `~/.codex/skills/`, idempotent across re-runs, with manifest entries matching every file written.
2. Default `install` refuses to overwrite paths the manifest does not own; `--force` backs up to `<path>.bak.<UTC-ISO8601>` (timestamped) and overwrites; repeated `--force` runs do not clobber earlier backups; all behaviors covered by integration tests.
3. `llm-wiki init <path>` produces a project structure for Create mode that matches the existing `init-project` skill output for equivalent answers (verified by golden-file fixture). Update mode is explicitly out of scope and returns a clear error if invoked against a non-empty project.
4. Canonical skill source contains no runtime-specific markup (`<!-- CLAUDE -->` / `<!-- CODEX -->` blocks); per-runtime variation is handled by the projector via the schema documented in "Skill Content And Rendering Model".
5. `llm-wiki uninstall` removes only files listed in the manifest; user-authored content at unrelated paths is untouched (verified by integration test).
6. `llm-wiki doctor` flags broken pre-binary symlinks under `~/.codex/skills/` (closing review.md §9.2) and reports manifest-vs-filesystem drift accurately.

**Test gates:**

7. Golden-file snapshots (`insta`) cover all 11 skill × runtime combinations and all 5 `init` profiles. `cargo insta test --check` passes in CI.
8. Unit tests cover markdown parsing (typed schema, all field validations), both projector impls, embedded loader, path resolution for `install` and `build --out`, and project-profile resolution.
9. Integration tests cover `install` idempotency, `--force`, `uninstall` symmetry, `init` profile correctness, `doctor` diagnostics, `status` drift detection, and the full collision matrix — using `tempfile::TempDir` and `HOME` redirection.
10. Property tests cover install idempotency and renderer totality.
11. Compat fixture: `tests/fixtures/wikis/v1/` exists and exercises every document type, every status value, archive entries, and `log.md`. The binary asserts against it (per the Compat Fixtures section above): every document parses; metadata extraction matches hand-coded expected structs per document type; `init` invoked with the fixture's original answers regenerates an equivalent `project_guidelines.md`; every skill name referenced in the fixture has a current canonical entry under `skills/`. **No round-trip claim and no "operations succeed" claim** — operations-level compatibility is the agent's checklist, not the binary's contract.
12. Line coverage ≥ 80% measured by `cargo-llvm-cov`. CI fails below. Coverage is a floor, not a goal — categories above are what matter.

**Distribution:**

13. Binary builds cleanly on macOS arm64, macOS x86_64, Linux x86_64, Linux arm64 via `cargo-dist`. Same matrix runs the test suite.
14. `cargo install llm-wiki-framework` works for users with a Rust toolchain.
15. End-user install path (`curl ... | sh && llm-wiki install`) verified by post-install file-and-manifest assertions in a release-gate workflow.

**Roadmap:**

16. `wiki/roadmaps/framework-v1.roadmap.md` updated to add D8 (or the agreed roadmap-position outcome from this proposal) before the binary ships.

## Revisit When

- The framework needs to support a runtime not covered by the current projector trait. Adding Cursor/Aider/Amp is a small change but warrants a quick review of the schema's expressivity.
- A breaking change to wiki shape becomes unavoidable. At that point, decide between a `migrate` subcommand, versioned install dirs, or per-project framework pinning.
- A user demands per-project skill overrides (e.g., a project-specific tweak to `knowledge-ingest`). At that point, decide whether to add a project-level overlay mechanism or whether projects can register custom skills via a separate path.
- The binary's release cadence diverges meaningfully from the canonical skill change cadence. May indicate a need for skill-only updates (`llm-wiki update-skills`) without re-downloading the binary.

## Implementation Outline (Non-Binding)

To be detailed in a follow-up `*.plan.md` if accepted:

1. Stand up Rust crate `llm-wiki` with `clap` CLI scaffold.
2. Define canonical skill schema and migrate the six existing skills from `skills/<name>/SKILL.md` (with `<!-- TAG -->` blocks) to clean canonical form.
3. Implement projector trait + Claude and Codex impls. `insta` snapshot tests against existing rendered output (after content-completeness audit recorded in `wiki/log.md` 2026-05-06 entry on the bash renderer).
4. Implement `build` (no manifest, write-anywhere) and `install` (manifest-tracked, global-only) as separate subcommands.
5. Migrate `init-project` skill prose into deterministic binary code; keep a thin agent-driven SKILL.md wrapper.
6. `cargo-dist` setup and first GitHub Release.
7. `doctor` and `uninstall` last.
8. Promote: archive `wiki/plans/single-source-skills.plan.md` (the bash renderer plan), supersede `wiki/decisions/framework-path-resolution.decision.md` and `wiki/decisions/project-local-codex-skills.decision.md`, write a new decision page recording the binary distribution model.
