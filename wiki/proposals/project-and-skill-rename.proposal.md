# Rename Project to `llm-wiki-rs` and Skills to `wiki-*`

- Document Class: Proposal
- Status: Draft
- Date: 2026-05-08
- Category: Branding, naming, developer ergonomics
- Scope: Rename the workspace package from `llm-wiki-framework` to `llm-wiki-rs`, leave the binary name (`llm-wiki`) intact, and re-namespace every Claude/Codex skill directory and user-facing invocation surface from `knowledge*` to `wiki-*`. Canonical skills are projected from embedded source by the binary (per the LLM Wiki Binary Distribution decision), so the rename touches the embedded canonical skill set, the projector, the global install paths under `~/.claude/skills/` and `~/.codex/skills/`, the in-repo runtime mirrors under `.claude/` and `.codex/`, the source tree under `src/`, the package metadata in `Cargo.toml` / `Cargo.lock`, the prose in `assets/templates/`, the project-root `AGENTS.md`, and every active `wiki/` document that references the old names. Managed runtime state under `~/.llm_wiki/` and per-project `.llm_wiki/init.toml` are intentionally unchanged by this rename. The on-disk wiki layout, document classes, and status vocabulary are untouched.
- Sources: Cargo.toml, Cargo.lock, README.md, AGENTS.md, .claude/skills/{knowledge,knowledge-ingest,knowledge-lint,knowledge-query,knowledge-research,init-project}/, .codex/skills/, src/ (skill projector and any user-visible identifiers), assets/skills/ (canonical embedded source), assets/templates/project_guidelines.md, assets/templates/CLAUDE.md, wiki/specs/knowledge-*-skill.spec.md, wiki/decisions/knowledge-command-namespace.decision.md, wiki/decisions/knowledge-research-intake.decision.md, wiki/decisions/llm-wiki-binary-distribution.decision.md, wiki/specs/documentation-model.spec.md, wiki/index.md, wiki/log.md
- Related: wiki/proposals/llm-wiki-binary.proposal.md, wiki/decisions/llm-wiki-binary-distribution.decision.md, wiki/specs/knowledge-init-skill.spec.md, wiki/proposals/blueprint-pack-init.proposal.md
- Supersedes: wiki/decisions/knowledge-command-namespace.decision.md (replaces the `$knowledge` namespace and the "existing direct skill names remain acceptable aliases" clause with a clean `wiki-*` namespace and no aliases)

## Question

Should the workspace package be renamed to `llm-wiki-rs` and every `knowledge*` skill/command be moved to a `wiki-*` namespace, or should the current names stay?

## Proposal

Yes, do both, in one change set. Default to a hyphenated `wiki-*` namespace. Keep colon-separated `wiki:*` only as a speculative fallback candidate if runtime proof shows a compelling reason to prefer it despite the filesystem portability cost (see *Runtime Proof Required* below).

### Package rename

Rename the workspace package from `llm-wiki-framework` to `llm-wiki-rs`.

- Package name: `llm-wiki-rs` (Cargo `[package].name`).
- Binary name: stays `llm-wiki` — it is already short, distinct, and printed in user-facing CLI help. Renaming the binary is a separate UX change with downstream cost (every doc, every install instruction) and no benefit attached to this proposal.
- Workspace member crate `llm-wiki-schema` keeps its name.
- Repo directory may follow (`llm_wiki_framework/` → `llm-wiki-rs/`) but is not required for the package rename to land; it is a follow-up on the user's local checkout.

The `-rs` suffix is accurate (this is a Rust project) and disambiguates from the user's other `llm_wiki` work without inventing a brand. It also matches established Rust ecosystem convention (`tokio-rs`, `actix-rs`, etc. as org/repo names) so readers don't have to learn a new convention.

### Skill / command rename

Move every skill directory and user-facing invocation surface from the `knowledge*` prefix to a `wiki-*` namespace. Starting state is the post-D8.1 set (the pre-release `init-project` name was already corrected to `knowledge-init` by the binary-distribution decision):

| Today | After |
| --- | --- |
| `knowledge` | `wiki` (parent skill / dispatcher) |
| `knowledge-init` | `wiki-init` |
| `knowledge-ingest` | `wiki-ingest` |
| `knowledge-lint` | `wiki-lint` |
| `knowledge-query` | `wiki-query` |
| `knowledge-research` | `wiki-research` |

`wiki` mirrors the on-disk directory (`wiki/`) and the conceptual unit users already work with. The installed artifact being renamed is the skill directory under `.claude/skills/` and `.codex/skills/`; "slash command" is just the runtime invocation surface users see in some clients. The proposal renames both the installed skill names and the user-facing invocation names together so the system does not have two competing vocabularies.

The verbs (`init`, `ingest`, `lint`, `query`, `research`) are unchanged. Renaming is purely the prefix. `wiki-research` is intentionally a tool-surface name, not a claim that the command writes directly into `wiki/`: the accepted intake split still holds, where research gathers into `raw/` and ingest compiles into `wiki/`.

### Runtime Proof Required

The superseded namespace decision deliberately treated `$knowledge` as a *prose* namespace layered over normal skill names, precisely so it would not depend on product-level slash-command registration. This proposal abandons that hedge: the new names are real skill *directory* names, and the names must survive end-to-end through both runtimes. Because colons are cross-platform-hostile in filenames, the default candidate is `wiki-*`, not `wiki:*`. Before promotion, verify in a scratch environment:

1. **Filesystem pre-flight.** Verify the candidate separators locally before touching the full runtime matrix. `wiki-*` is expected to pass everywhere. `wiki:*` must clear an explicit throwaway-directory check first and is rejected immediately if it fails any cross-platform portability requirement.
2. **Install.** `llm-wiki install` writes `~/.claude/skills/wiki/`, `~/.claude/skills/wiki-init/`, `~/.codex/skills/wiki/`, `~/.codex/skills/wiki-init/`, etc., without errors and with manifest entries that round-trip through `llm-wiki status`.
3. **Discovery.** Both runtimes list the installed skills in their respective surfaces (Claude's skill list, Codex's skill list) under the new names.
4. **Invocation.** A user can invoke `wiki-init`, `wiki-query`, `wiki-lint` and reach the underlying behavior.
5. **Uninstall.** `llm-wiki uninstall` removes them in reverse order with no orphans.

If a later third-party integration turns up a separator-specific problem after promotion, rerun the rename plan with the other proven separator and record the reversal in the decision and log. Today, though, the intended canonical form is `wiki-*`; `wiki:*` is a speculative variant that must beat the portability objection to stay alive.

### Compatibility

This is a clean rename with no aliases. The framework has a single user (the author) and no public release, so dual-namespace shims would add maintenance with no payoff. Anyone with a half-typed `/knowledge-…` in muscle memory updates once. The superseded decision's "existing direct skill names remain acceptable aliases" clause is dropped along with `$knowledge`.

If a public release ever happens after this lands, the public-facing names are already correct from day one — and `wiki-*` is self-explanatory enough that new users do not need a glossary to understand what the namespace refers to.

## Why

1. **Disambiguation.** The user has more than one project in the `llm_wiki` orbit. `llm-wiki-framework` and `llm_wiki` collide in conversation, on disk, and in `cargo` output. `llm-wiki-rs` makes the *implementation* of the wiki distinct from the *concept* of an llm-wiki.
2. **Branding alignment.** The framework already calls its binary `llm-wiki` and its core directory `wiki/`. The package name and skill prefix are the last two surfaces still using the old project codename (`framework`) or a generic word (`knowledge`). Aligning all four surfaces removes one layer of cognitive translation.
3. **Skill prefix is misleading.** "Knowledge" describes *what* the wiki holds; it does not name the tool. Completion surfaces in any client list `/knowledge-…` as if knowledge itself were the product, which obscures the framework. `wiki-*` names the artifact the user is operating on, which is the right unit at the command level and matches the on-disk `wiki/` directory.
4. **Ergonomics.** `/wiki-ingest` is shorter than `/knowledge-ingest`, self-describing rather than cryptic, and the namespace mirrors the directory the user already navigates daily. Brevity that costs zero discoverability.
5. **The cost is small and one-shot.** The work is a grep-and-replace across `Cargo.toml`, `Cargo.lock`, README, the embedded canonical skills under `assets/skills/`, the projected runtime mirrors under `.claude/` and `.codex/`, the projector itself in `src/`, `assets/templates/`, the root `AGENTS.md`, and every active `wiki/` document that names a skill. There is no API deprecation surface to manage.

## Alternatives Considered

1. **Rename the package only, keep `knowledge*` skill names.** Cheaper, but leaves the most-typed surface (slash commands) unchanged. The branding payoff is half-realised; the disambiguation payoff (skills appearing in client lists) is missed entirely.
2. **Rename skills only, keep `llm-wiki-framework` package name.** Possible, but then `cargo` output and `Cargo.toml` still carry a name the author does not use anywhere else. The branding asymmetry is the failure mode this proposal exists to fix.
3. **Use `lw:` or `lw-` as the skill namespace.** Faster to type, but rejected on clarity grounds: a two-letter abbreviation requires every reader to learn what `lw` means before any command makes sense, and the small character savings over `wiki-*` do not pay for that onboarding tax.
4. **Use `llm-wiki:` or `llm-wiki-` as the skill namespace.** Maximum self-description but too long before the verb starts. Rejected: at the skill-prefix layer, daily-typing cost dominates and `wiki-*` is already self-explanatory in context.
5. **Use `llm_wiki_rs` (snake_case) instead of `llm-wiki-rs`.** Cargo package names are conventionally kebab-case, and the binary is already `llm-wiki` (kebab). Snake-case here would create an internal inconsistency on the same project.
6. **Keep current names and add a project README disambiguation paragraph.** The cheapest option, but it asks the human reader to do the disambiguation work every time. The whole point of a name is to avoid that.
7. **Keep the `$knowledge` prose-namespace decision and only rename the package.** Rejected: that decision was explicitly framed as a prose alias because product-level command registration was uncertain. The hedge has run its course — `wiki-*` skill names are real installed artifacts today, so the right fix is to commit and prove, not extend the workaround.

## Consequences and Tradeoffs

- One-shot break: any external script, alias, or muscle memory referring to `knowledge-*` or `llm-wiki-framework` stops working. Acceptable given the audience (one user) and the absence of a public release.
- The accepted `knowledge-command-namespace.decision.md` is superseded — its `$knowledge` namespace and its "direct skill names remain acceptable aliases" clause both go away.
- Active `wiki/` prose should use the new naming consistently: "wiki skills", `wiki-*`, and `llm-wiki-rs`. The old `knowledge-*`, `$knowledge`, and `llm-wiki-framework` names remain only in archived history, provenance notes, and `wiki/log.md` entries that record what happened at the time.
- Future blueprint/pack work (D10) lands cleaner: pack documentation can reference `wiki-lint`, `wiki-ingest` consistently rather than carrying a "formerly `knowledge-…`" footnote.
- The `-rs` suffix commits to "this is the Rust implementation." That is true today and is unlikely to change, but if a non-Rust port ever appears, it would need its own suffix (`-py`, `-ts`) and `llm-wiki-rs` would not be the natural umbrella name. Treating that as an acceptable far-future cost.
- `cargo install llm-wiki-framework` and `cargo install llm-wiki-rs` are different crate names on crates.io. For the current one-user, pre-release audience that is acceptable, but the discontinuity is real and should be documented in README/install guidance.
- Managed runtime paths under `~/.llm_wiki/` and per-project `.llm_wiki/init.toml` stay unchanged. They are storage/config paths, not part of the user-facing naming cleanup.
- Specs, plans, decisions, the index, the root `AGENTS.md`, and template prose under `assets/templates/` need a coordinated sweep. Concretely: `wiki/specs/knowledge-*-skill.spec.md` rename to `wiki/specs/wiki-*-skill.spec.md`, the superseded namespace decision moves to `wiki/archive/` per house style, `wiki/index.md` and `wiki/log.md` get updates, and any plan, roadmap, or template text naming a skill is updated in lockstep. Stale filenames or template prose would undermine the rename's clarity goal.

## What Closes This Proposal

Promotion to a decision plus an execution plan covering:

1. **Runtime proof first.** Run the filesystem pre-flight and then the install/discovery/invocation/uninstall checks in *Runtime Proof Required* under both Claude and Codex. Record results in the plan. `wiki-*` is the expected canonical outcome; do not expand scope to `wiki:*` unless it clears the portability objection and offers a compelling practical benefit. Do not start the rename sweep until this is settled.
2. `Cargo.toml` rename (`name = "llm-wiki-rs"`), workspace member references, `Cargo.lock` regenerated, `cargo install llm-wiki-framework` instruction in README updated to `cargo install llm-wiki-rs`.
3. README, repo metadata, and any `crates.io`-shaped fields updated.
4. Embedded canonical skills under `assets/skills/` renamed (directories and any self-references inside `SKILL.md`); projector code in `src/` updated to emit the new names; runtime mirrors under `.claude/skills/` and `.codex/skills/` regenerated by `llm-wiki build --out .` rather than edited by hand.
5. Source-side rename of any `knowledge_*` identifier in `src/` that surfaces a skill name to the user (CLI help, install paths, manifest entries, status output). Internal-only `knowledge_*` identifiers that never reach the user can stay or be renamed opportunistically.
6. Sweep active prose surfaces for explicit `knowledge-…` and `llm-wiki-framework` mentions. Specifically: rename `wiki/specs/knowledge-*-skill.spec.md` to `wiki/specs/wiki-*-skill.spec.md`, archive `wiki/decisions/knowledge-command-namespace.decision.md` with a `Superseded By:` pointer to the new decision, update `wiki/index.md`, `wiki/log.md`, `AGENTS.md`, and relevant `assets/templates/` text, and leave old names only in archive/log/provenance context.
7. End-to-end verification on a scratch project: `cargo build`, `cargo test`, `llm-wiki install`, list skills in both runtimes, invoke `wiki-init` to scaffold a new project, then `llm-wiki uninstall` and confirm no orphans.

## Open Questions

1. Whether the on-disk repo directory (`llm_wiki_framework/`) should be renamed to `llm-wiki-rs/` as part of this change or left alone as a local-checkout concern. Lean: leave alone; the package name is what travels.
2. Whether `wiki/specs/knowledge-init-skill.spec.md` and similar spec/plan filenames are renamed in the same change or in a follow-up sweep. Lean: same change, since stale spec filenames undermine the rename's clarity goal (already reflected in the close criteria above).

## One-Off Legacy Migration

This rename does not make the next framework version responsible for legacy
compatibility. The product change is still a clean break to `wiki-*`, with no
runtime aliases, no dual namespace, and no new framework behavior whose job is
to preserve `knowledge*`.

For the small number of older local projects that still depend on the pre-rename
skill tree, do a one-off migration outside the framework contract:

1. Rename the in-repo runtime mirrors from `.claude/` to `.claude.legacy/` and
   from `.codex/` to `.codex.legacy/` in those legacy projects only.
2. Repoint the existing home-level skill symlinks under `~/.claude/skills/` and
   `~/.codex/skills/` so they target the legacy folders instead of the current
   canonical `.claude/skills/` and `.codex/skills/` paths.
3. Leave the renamed framework repo free to regenerate only the new canonical
   `.claude/` and `.codex/` surfaces for `wiki-*`.

This is intentionally a transition tactic for a couple of local repos, not an
ongoing architecture. Do not generalize it into `llm-wiki install`, the
manifest model, the projector, or the init template system. The framework
should remain clean; the legacy projects carry their own frozen transition
surface.
