# Plan: Consolidate Skills Into a Single Source of Truth

- Document Class: Plan
- Status: Active
- Date: 2026-05-06
- Category: Tooling, framework infrastructure
- Scope: Eliminate `.claude/skills/` ↔ `.codex/skills/` duplication by introducing one canonical `skills/` source and generating per-runtime variants from it.
- Sources: review.md §9 (Codex follow-up findings), wiki/decisions/project-local-codex-skills.decision.md, wiki/specs/documentation-model.spec.md
- Related: wiki/decisions/single-source-skills.decision.md, wiki/specs/init-project-skill.spec.md, wiki/specs/knowledge-ingest-skill.spec.md, wiki/specs/knowledge-query-skill.spec.md, wiki/specs/knowledge-research-skill.spec.md, wiki/specs/knowledge-lint-skill.spec.md
- Successor: wiki/decisions/llm-wiki-binary-distribution.decision.md (will supersede this plan when D8 ships; pending smoke-test gates from §6 will be discharged by D8 rather than by completing this plan separately, but the bash renderer continues to operate the repo until then)

## 1. Deliverable

A canonical `skills/` directory at the repo root holds one definition per skill. `.claude/skills/` and `.codex/skills/` are populated from it by a small build step. After this change, editing a skill in two places is no longer possible: the divergence between Claude and Codex variants of the same skill (currently present in `init-project`, `knowledge-ingest`, `knowledge-query`, `knowledge-research`) cannot recur.

## 2. Why Not Pure Symlinks

Initial intuition was: one `skills/` dir, symlink both `.claude/skills/` and `.codex/skills/` into it. After verification this is insufficient:

- Each `.codex/skills/<name>/` has an `agents/openai.yaml` file that Claude does not use.
- `.codex/skills/knowledge/SKILL.md` is a `$`-namespace dispatcher with no Claude analogue (Claude routes by description match, not by namespace prefix).
- The current `SKILL.md` files have **diverged in content** — different frontmatter descriptions, different invocation syntax (`/knowledge-ingest` vs `$knowledge ingest`), and at least one runtime-specific behavior (the §9.4 URL/web divergence).
- Claude is missing `knowledge-lint` entirely.

A symlink forces identical content. The runtimes legitimately need different surface descriptions and different runtime configs. We need **templating**, not aliasing.

## 3. Target Architecture

Reuse the conditional-section pattern already proven in `project_guidelines.template.md` (which uses `<!-- SECTION:ML_AI -->` and `<!-- SECTION:QMD -->` markers).

```text
skills/
  README.md                     One-page explanation of the build model
  build.sh                      Render skills/ → .claude/ and .codex/
  init-project/
    SKILL.md                    Canonical body with <!-- CLAUDE -->, <!-- CODEX --> blocks
    codex/
      openai.yaml               Codex-only runtime config
  knowledge-ingest/
    SKILL.md
    codex/
      openai.yaml
  knowledge-query/
    SKILL.md
    codex/
      openai.yaml
  knowledge-research/
    SKILL.md
    codex/
      openai.yaml
  knowledge-lint/
    SKILL.md
    codex/
      openai.yaml
  knowledge/                    Codex-only dispatcher
    SKILL.md
    codex/
      openai.yaml
```

Generated outputs (gitignored or regenerated on demand):

```text
.claude/skills/<name>/SKILL.md                Frontmatter + body, Claude-resolved sections
.codex/skills/<name>/SKILL.md                 Frontmatter + body, Codex-resolved sections
.codex/skills/<name>/agents/openai.yaml       Copied from skills/<name>/codex/openai.yaml
```

Conditional markers in canonical `SKILL.md`:

```markdown
<!-- FRONTMATTER:CLAUDE -->
description: Ingest raw source material into the project wiki. ...
<!-- END -->

<!-- FRONTMATTER:CODEX -->
description: Ingest explicit raw source material into an LLM Wiki project. ...
<!-- END -->

## Invocation

<!-- CLAUDE -->
/knowledge-ingest <file-or-directory>
<!-- END -->

<!-- CODEX -->
$knowledge-ingest <file-or-directory>
$knowledge ingest <file-or-directory>
<!-- END -->
```

## 4. Out Of Scope

- MCP-based skill exposure. The format-level convergence solves the duplication problem; runtime convergence via MCP is a larger question that belongs in a separate proposal.
- Symlinking from `~/.claude/skills/` and `~/.codex/skills/` to global aliases. Required for the §9.2 fix but tracked separately as a follow-up.
- Replacing the hardcoded framework path inside skill bodies (§9.1). Will be addressed by the same edit pass but is logically a different defect.
- Distribution to other agents (Cursor, Aider, Amp). The renderer is the extension point — adding a new target is a future enhancement, not part of V1.
- Versioning skills across spawned projects (D7 follow-up).

## 5. Steps

### 5.1 Establish canonical sources

1. Create `skills/` at repo root with one subdirectory per skill listed in §3.
2. For each existing skill pair, diff Claude and Codex variants and produce one canonical `SKILL.md` with conditional blocks for the genuinely runtime-specific sections.
3. Move `agents/openai.yaml` files into `skills/<name>/codex/openai.yaml`.
4. Promote the §9.4 decision: URL/web sources route through `knowledge-research` first on **both** runtimes. This converts a runtime-specific quirk into shared canonical text and shrinks the conditional surface.
5. Port `knowledge-lint` from Codex into the canonical `skills/knowledge-lint/`. This closes the existing drift gap (Claude is currently missing it).
6. Replace hardcoded absolute paths (`/Users/.../software_project_management`, `/Users/.../llm_wiki_framework`) with skill-file-relative resolution. The build script knows the framework root because the canonical files live inside it.

### 5.2 Build script

`skills/build.sh` (bash, no dependencies) does three things:

1. For each `skills/<name>/SKILL.md`:
   - render Claude variant by keeping `<!-- CLAUDE -->` blocks, dropping `<!-- CODEX -->` blocks, and selecting `<!-- FRONTMATTER:CLAUDE -->` → write to `.claude/skills/<name>/SKILL.md`;
   - render Codex variant analogously → write to `.codex/skills/<name>/SKILL.md`.
2. Copy `skills/<name>/codex/openai.yaml` to `.codex/skills/<name>/agents/openai.yaml` when present.
3. Skip Claude rendering for the Codex-only `knowledge` dispatcher.

The script is idempotent: running it twice produces identical output.

### 5.3 Wire up regeneration

Two acceptable triggers, pick one:

- **Manual** — document `bash skills/build.sh` in `skills/README.md` and a checklist; the user runs it after editing a canonical skill. Simple, no hooks.
- **Hooked** — a Claude Code post-edit hook in `.claude/settings.json` runs `skills/build.sh` whenever a `skills/**/SKILL.md` file is edited. Removes the "I forgot to rebuild" failure mode.

Recommend manual for V1. Add a hook only if regeneration drift becomes a real issue.

### 5.4 Decide whether generated dirs are committed

Two options, pick one:

- **Commit `.claude/skills/` and `.codex/skills/`** so a fresh clone works without running the build. Cost: every canonical edit produces a two-file diff per skill (canonical + rendered).
- **Gitignore them** and rely on the build step. Cost: clones need a bootstrap; CI must run the build before any skill-discovery test.

Recommend committing the rendered output for V1. The audience for this framework today is "drop into a project and use it" — requiring a build step on first clone is friction. Revisit if the diff noise becomes annoying.

### 5.5 Repoint global symlinks (closes §9.2)

After generated output stabilizes, recreate the broken symlinks under `~/.codex/skills/` to point at this repo's `.codex/skills/<name>` paths. Same for any `~/.claude/skills/` global aliases. This is mechanical but should happen *after* the canonical sources are correct, so we don't enshrine drift.

### 5.6 Update specs

Skill specs in `wiki/specs/` currently document only the Claude/Codex pair structure. Update each to reference the canonical source under `skills/` and describe the rendering model. Add one new decision: `wiki/decisions/single-source-skills.decision.md` capturing the choice and the alternatives rejected (pure symlinks, MCP, status quo).

## 6. Verification Gates

A `pass` requires all of:

1. `bash skills/build.sh` exits 0 and produces no diff on a second run.
2. `diff -rq .claude/skills/ .codex/skills/` shows only expected runtime-specific differences (frontmatter descriptions, `agents/` subdirs, the Codex-only `knowledge` dispatcher).
3. Every skill listed in `wiki/index.md` has a canonical source under `skills/`.
4. `knowledge-lint` exists for both Claude and Codex (closing the drift gap).
5. No skill body contains an absolute path under `/Users/...`. Replaced with skill-file-relative resolution.
6. URL-routing rule is identical on both runtimes (research-first), closing §9.4.
7. Manual smoke test: invoke `init-project`, `knowledge-ingest`, `knowledge-query`, `knowledge-research`, `knowledge-lint` in Claude Code; confirm they load and behave as before.
8. Manual smoke test: same five skills under Codex via `$knowledge` dispatcher.

## 7. Evidence To Record

- A `wiki/log.md` entry capturing the consolidation, listing every file moved or generated.
- The new decision page (`single-source-skills.decision.md`) added to `wiki/index.md` under Decisions.
- Updated skill specs reflecting the canonical source location.
- The existing review.md §9 items 9.1, 9.2, 9.4, and the Claude `knowledge-lint` gap each marked resolved (in a follow-up review note or the §9 table).

## 8. Wiki Pages To Update When Done

- `wiki/specs/init-project-skill.spec.md` — point Location at `skills/init-project/`, drop hardcoded path.
- `wiki/specs/knowledge-ingest-skill.spec.md` — same; document the unified URL-routing rule.
- `wiki/specs/knowledge-query-skill.spec.md` — same.
- `wiki/specs/knowledge-research-skill.spec.md` — same.
- `wiki/specs/knowledge-lint-skill.spec.md` — same; add proof that Claude variant now exists.
- `wiki/decisions/project-local-codex-skills.decision.md` — add a `Superseded By` link to the new single-source decision (the project-local Codex-only model is now part of a unified scheme).
- `wiki/decisions/single-source-skills.decision.md` — new.
- `wiki/index.md` — list new decision; statuses unchanged otherwise.
- `wiki/log.md` — operation entry.

## 9. What Closes The Plan

- All steps in §5 complete.
- All gates in §6 pass.
- Index and log reflect the change.
- Status of this plan moves from `Draft` → `Active` on start, `Completed` when gates pass.
- A short review.md §10 entry confirms §9.1, §9.2 (after symlink rebuild), §9.4, and the missing-lint drift are closed.

## 12. Implementation Status

Implementation completed on 2026-05-06 for all repo-local deliverables:

- canonical `skills/` source tree exists
- `skills/build.sh` renders runtime outputs
- Claude and Codex skill outputs are generated and committed
- Claude `knowledge-lint` exists
- global framework symlinks point at this repo
- wiki specs, index, log, and decision records are updated
- canonical skill sources preserve the full pre-consolidation runtime
  instructions, with only targeted fixes for path resolution and
  research-first URL routing

Local verification gates passed: build idempotency, canonical source coverage,
absence of `/Users/...` paths in skill bodies, and unified research-first URL
routing.

Remaining gates before marking this plan Completed:

- manual smoke test of `init-project`, `knowledge-ingest`, `knowledge-query`,
  `knowledge-research`, and `knowledge-lint` in Claude Code
- manual smoke test of the same five operations in Codex through `$knowledge`

## 10. Risks And Mitigations

| Risk | Likelihood | Mitigation |
| --- | --- | --- |
| Build script bugs render skills broken on disk | Medium | Idempotency check in §6.1; commit generated output so fresh clone works without build |
| Conditional markers proliferate, canonical files become unreadable | Low–Medium | Promote any block that survives in both runtimes to shared text on every edit; reject markers for differences smaller than one paragraph |
| Future agent (Cursor, Aider) needs different runtime config | Medium | Renderer is one bash file; adding a new target is a small change, not a redesign |
| Hooked regeneration introduces commit-time surprises | Low | V1 keeps regeneration manual |
| Skill discovery breaks during migration | Low | Migrate one skill end-to-end first (suggest `knowledge-lint` since it's the simplest and closes a drift gap), validate, then do the rest |

## 11. Sequencing

1. Pilot with `knowledge-lint` (smallest blast radius, closes a known drift).
2. Migrate `knowledge-research` next (biggest active divergence on URL handling — exercises the conditional-block design).
3. Migrate the remaining three (`init-project`, `knowledge-ingest`, `knowledge-query`).
4. Migrate the Codex-only `knowledge` dispatcher.
5. Land the new decision page and updated specs.
6. Repoint global symlinks (§9.2).
7. Mark the §9 review items resolved.

Steps 1–4 are independent and could parallelize. Step 5 depends on 1–4. Step 6 depends on 5 because we don't want global aliases pointing at canonical-but-unblessed files.
