# LLM Wiki Binary Distribution

- Document Class: Decision
- Status: Accepted
- Date: 2026-05-06
- Category: Tooling, framework distribution
- Scope: Distribute the framework as a single Rust binary (`llm-wiki`) that owns global skill installation, project scaffolding, and skill projection from canonical markdown.
- Sources: wiki/proposals/llm-wiki-binary.proposal.md, wiki/archive/single-source-skills.decision.md, wiki/archive/framework-path-resolution.decision.md, wiki/archive/project-local-codex-skills.decision.md, review.md §9-10, wiki/log.md (line-loss audit, 2026-05-06)
- Related: wiki/roadmaps/framework-v1.roadmap.md (D8, D8.1, D11), wiki/specs/wiki-init-skill.spec.md, wiki/specs/documentation-model.spec.md, wiki/decisions/binary-path-bootstrap.decision.md
- Supersedes: wiki/archive/single-source-skills.decision.md, wiki/archive/framework-path-resolution.decision.md, wiki/archive/project-local-codex-skills.decision.md, wiki/archive/single-source-skills.plan.md
- Amended By: wiki/decisions/binary-path-bootstrap.decision.md

## Choice

The framework is distributed as a single Rust binary, `llm-wiki`. The binary owns:

1. **Global skill installation**: `llm-wiki install` writes rendered framework skills directly to `~/.claude/skills/` and `~/.codex/skills/` with an ownership manifest. The pre-release D8 implementation used `~/.local/share/llm-wiki/manifest.json`; D8.1 corrects this before public release to a managed runtime home at `~/.llm_wiki/` with manifest `~/.llm_wiki/manifest.json` and installed skills invoking `~/.llm_wiki/bin/llm-wiki` by absolute path. No symlinks.
2. **Project scaffolding**: `llm-wiki init <path>` creates new project structure (Create mode only; Update mode remains agent-owned) deterministically from embedded templates.
3. **Skill projection**: canonical markdown under `assets/skills/<name>/SKILL.md` (clean, no `legacy tag marker` blocks) is embedded at compile time via `include_str!` and projected into Claude/Codex variants by typed Rust code.
4. **State diagnostics**: `llm-wiki status` and `llm-wiki doctor` read the manifest plus filesystem to report install state, drift, and pre-binary symlink breakage.
5. **Removal**: `llm-wiki uninstall` deletes only manifest-owned files in reverse install order.

Distribution: multi-arch prebuilt binaries via `cargo-dist` on GitHub Releases (macOS arm64, macOS x86_64, Linux x86_64, Linux arm64), plus `cargo install llm-wiki-rs` for users with a Rust toolchain.

Versioning follows the `git` model: one global install, the framework guarantees backward compatibility with older `wiki/` shapes (precisely defined in the accepted proposal), no per-project version pinning. Compat fixtures committed under `tests/fixtures/wikis/` lock the promise.

## Why

Three concrete fragilities in the current model drove this decision:

1. **Broken global symlinks** (review.md §9.2). After the `software_project_management` → `llm_wiki_framework` rename, all `~/.codex/skills/` symlinks dangled silently. A binary that writes files directly to the global skill paths removes the indirection by construction.
2. **Renderer drift loses content silently.** The pre-binary renderer accepted by `single-source-skills.decision.md` shipped working idempotency but lacked golden-file regression tests. The first canonical-source draft measured `init-project` Claude variant 369 → 155 lines (−58%) and `knowledge-research` Codex variant 200 → 124 lines (−38%) — both detected only by stash-and-diff inspection, not by any automated gate. A typed Rust projector with `insta` snapshot tests catches the same class of regression in CI.
3. **LLM-interpreted scaffolding cannot be tested.** `init-project` as a markdown skill produces non-deterministic output: the agent re-interprets the prose each invocation. Moving file generation to deterministic code with embedded templates and golden fixtures gives CI-checkable scaffolding output.

The binary also collapses three previously-separate decisions (`project-local-codex-skills`, `framework-path-resolution`, `single-source-skills`) into one coherent distribution model. Path resolution is no longer a concern because the binary embeds its own content. Project-local skill exposure via symlinks is no longer a concern because skills are installed globally without symlinks. Single-source rendering is no longer a separate decision because the projector is part of the binary.

## Alternatives Considered

1. **Status quo: per-runtime skill copies plus global symlinks.** Rejected: review.md §9.2 documented the dangle-on-rename failure. Symlinks are a workaround for "make project-local skills globally discoverable"; a binary with global writes solves the actual need.

2. **Bash renderer + conditional-block canonical (the just-superseded `single-source-skills.decision.md`).** Rejected after measured content loss: the renderer worked correctly but had no test gate against canonical-source incompleteness. A typed renderer with golden-file tests is the response. Bash-plus-snapshot-tests was considered but does not earn its complexity if we are committing to a binary anyway for installation and scaffolding.

3. **Python script instead of Rust.** Rejected for distribution reasons: shipping a Python script means every spawned project either depends on a system Python or carries its own. A single self-contained binary has no runtime dependency. Python remains a reasonable choice if the binary plan stalls and a pure-renderer fallback is needed; not the path forward.

4. **Per-project install (skills copied into every project's `.claude/` and `.codex/`).** Rejected: skills are framework operations, not project content. They should be installed once per machine like `git`, not duplicated per project. Spawned projects then get smaller (no skill files needed) and benefit from framework upgrades transparently.

5. **MCP server exposing skills as prompts/tools.** Deferred: cleaner long-term primitive but a larger runtime-integration question than V1 needs. The binary distribution model and an eventual MCP exposure are not mutually exclusive; revisit when MCP supports skill-shaped invocation idiomatically.

6. **Agent-only scaffolding (today's `init-project` markdown skill, kept as-is).** Rejected for the deterministic-output reasons in §3 above. The agent retains the conversational intake; the binary owns file generation. Hybrid by design.

## Consequences

The binary is now the operating model:

- Predecessor decisions and plans are archived as superseded.
- Runtime skill files remain committed for first-clone usability and
  self-dogfooding, but they are regenerated by `llm-wiki build --out .`.
- `llm-wiki install` writes global Claude and Codex skills directly with an
  ownership manifest. D8.1 moves runtime install state into `~/.llm_wiki/` so
  installed skills do not depend on shell `PATH`. No symlinks are required.
- `llm-wiki init <path>` owns deterministic Create-mode scaffolding.
- Canonical skill assets live under `assets/skills/`; committed `.claude/` and
  `.codex/` runtime mirrors are generated convenience outputs.
- The `wiki-init` skill is a thin conversational wrapper over
  `llm-wiki init --non-interactive`; the pre-release `init-project` name was
  corrected by D8.1, and the `knowledge-init` name was renamed to `wiki-init`
  by D11 before public release.
- Spawned projects need no framework skill directories. Users install skills
  globally once, then scaffold each project with the binary.
- `wiki/checklists/v1-fixture-smoke.checklist.md` captures the agent-driven
  operations-level compatibility smoke test against the committed fixture.

## Backward Compatibility Promise

The compat surface is enumerated in the accepted proposal and is load-bearing:

1. Document type filename suffixes
2. Metadata block fields declared in the documentation-model spec
3. Folder layout under `wiki/`
4. Status vocabulary
5. The three-layer architecture invariant (`raw/`, `wiki/`, `AGENTS.md` + `project_guidelines.md`, with `CLAUDE.md` allowed as a compatibility shim)

Anything outside this list — skill internals, CLI flags, manifest schema, embedded template wording — may evolve freely between binary versions.

## Revisit When

- Either Claude Code or Codex adds a stable "list installed skills" CLI/API. The post-install verification path becomes more honest with that signal.
- A breaking change to wiki shape becomes unavoidable. At that point, decide between a `migrate` subcommand, versioned install dirs, or per-project framework pinning.
- A user demands per-project skill overrides. At that point, decide whether to add an overlay mechanism or a project-level skill registration path.
- The binary's release cadence diverges meaningfully from the canonical skill change cadence. May indicate a need for skill-only updates (`llm-wiki update-skills`) without re-downloading the binary.
- Multi-agent runtimes (Cursor, Aider, Amp) become a real V2 target. The projector trait is the extension point.
