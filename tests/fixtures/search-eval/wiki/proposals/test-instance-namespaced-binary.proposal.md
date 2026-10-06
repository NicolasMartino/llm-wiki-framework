# Coexisting Test Instance via Build-Time Namespace

- Document Class: Proposal
- Status: Accepted
- Date: 2026-06-21
- Promoted: 2026-06-21 (see wiki/decisions/test-instance-namespaced-binary.decision.md, wiki/plans/test-instance-namespaced-binary.plan.md)
- Category: Developer ergonomics, install/runtime isolation, dogfooding safety
- Scope: Add an optional **build-time namespace** baked into the binary so that
  a single environment variable at compile time (`LLM_WIKI_INSTANCE=test`)
  produces a parallel `llm-wiki-test` instance — distinct binary name, managed
  home, skill names, MCP/tool names, command surface, and manifest identity —
  that coexists with the production install in the *same* `$HOME`. A normal
  build (empty namespace) produces **exactly today's names, paths, skill
  payloads, manifests, and install outputs**; that output-equality is the hard,
  test-enforced invariant. V1 is **developer-only and `test`-only**: the build
  refuses any non-empty value other than `test`, there is no runtime flag, and
  release builds fail if `LLM_WIKI_INSTANCE` is set. This touches the path layer
  (`src/paths.rs`), the embedded skill set and projector (`src/embed.rs`,
  `src/build.rs`), install/uninstall (`src/install.rs`, `src/uninstall.rs`), the
  manifest identity (`src/manifest/schema.rs`), the CLI command name
  (`src/cli.rs`), the doctor surface (`src/doctor.rs`), the reserved MCP tool
  names, and the release workflow. It does **not** change production behavior,
  paths, names, or the single-binary distribution invariant.
- Sources:
  - src/paths.rs (managed home derives from `$HOME`; no instance seam today)
  - src/embed.rs (fixed skill names `wiki`, `wiki-init`, `wiki-query`, `wiki-ingest`, `wiki-research`, `wiki-lint`; reserved `llm_wiki_search*`)
  - src/build.rs (`apply_binary_context(doc, "llm-wiki")` injects the binary name into skill prose)
- Related:
  - wiki/proposals/project-and-skill-rename.proposal.md (prior art: skill-name plumbing, projector, install paths, runtime proof matrix)
  - wiki/decisions/llm-wiki-binary-distribution.decision.md (skills are projected from embedded source by the binary)
  - wiki/decisions/agent-owns-wiki.decision.md

## Question

How can the author validate the full installed experience — `install`,
`doctor`, the MCP surface, and especially the **skills as invoked inside a live
Claude Code / Codex session** — including unreleased changes like the Headroom
companion, with a *test-enforced guarantee* of no impact on the production
install they use every day?

## Proposal

Bake an **instance namespace** into the binary at compile time. A single
constant — `INSTANCE`, sourced from `option_env!("LLM_WIKI_INSTANCE")` and
defaulting to empty — flows through every user-visible and on-disk identifier
via one derivation API. Building normally yields production, unchanged. Building
with `LLM_WIKI_INSTANCE=test cargo build` (POSIX shells; the PowerShell
equivalent is `$env:LLM_WIKI_INSTANCE='test'; cargo build`) yields a binary named `llm-wiki-test`
whose entire footprint is suffixed, so it installs *alongside* production in the
same `$HOME` and can be exercised in a real agent session without colliding with
or touching the production install.

The namespace is **baked in, not a runtime flag**. There is no `--instance`
option to remember and, critically, no way to fat-finger the test binary into
writing production **managed/install state**: once the namespace covers every
on-disk root (see the table below), the test binary cannot address the
production managed home, skills, manifest, registry, or caches. The experience
is seamless — you run `llm-wiki-test` exactly as you run `llm-wiki`, and
isolation is structural. (Project working trees are a separate matter — see
Consequences; those are *not* isolated.)

### Scope guards (V1 is developer-only, `test`-only)

Three hard constraints keep this a dogfooding safety feature rather than a
second product identity system:

1. **`test`-only.** The build accepts an empty `INSTANCE` (production) or the
   literal `test`. Any other non-empty value fails the build with a clear error.
   The derivation API is internally general (it takes a suffix), but no other
   value is reachable or proven in V1. This avoids paying the validation cost of
   arbitrary names across filenames, Windows paths, skill names, MCP
   identifiers, manifests, and docs before there is a second consumer.
2. **No runtime flag.** Instance selection is compile-time only. A runtime
   `--instance` would reintroduce the mis-targeting surface this design exists to
   remove.
3. **Release builds reject a non-empty namespace.** The release workflow asserts
   `LLM_WIKI_INSTANCE` is unset/empty and fails loudly otherwise, so a stray
   environment value can never ship a `-test` artifact to users.

### What the namespace suffixes

For instance `test`:

| Surface | Production | Instance `test` | Where |
| --- | --- | --- | --- |
| Binary name | `llm-wiki` | `llm-wiki-test` | `src/paths.rs::managed_binary_name`, `src/cli.rs` `command(name=…)` |
| Managed home (Unix) | `~/.llm_wiki` | `~/.llm_wiki-test` | `src/paths.rs` (manifest, profile, search configs, indexes, models nest under it) |
| Managed root (Windows) | `%LocalAppData%/llm_wiki` | `%LocalAppData%/llm_wiki-test` | `src/paths.rs::from_windows_known_folders` |
| Cache root | `~/.cache/llm-wiki` | `~/.cache/llm-wiki-test` | `src/paths.rs` `cache_home` — **XDG, does *not* nest under managed home; must be namespaced explicitly** |
| Data root + registry | `~/.local/share/llm-wiki`, `…/projects.json` | `~/.local/share/llm-wiki-test`, `…/projects.json` | `src/paths.rs` `data_home`, `project_registry` — **XDG, does *not* nest under managed home; must be namespaced explicitly** |
| Skill dir + SKILL.md `name:` | `wiki`, `wiki-ingest`, … | `wiki-test`, `wiki-ingest-test`, … | `src/embed.rs` names; `src/install.rs` install paths; projector frontmatter |
| Binary the skills call | `llm-wiki` | `llm-wiki-test` | `src/build.rs::apply_binary_context` |
| MCP server + reserved tools | `llm_wiki_search`, `llm_wiki_search_all` | `llm_wiki_search_test`, `llm_wiki_search_all_test` | `src/embed.rs` reserved set |
| Manifest identity | `installed_by: "llm-wiki"` | `installed_by: "llm-wiki-test"` | `src/manifest/schema.rs` (round-trips through `src/status.rs`) |

**Not every root nests under the managed home.** `cache_home`
(`~/.cache/llm-wiki`), `data_home` (`~/.local/share/llm-wiki`), the project
registry (`data_home/projects.json`), and the Windows managed root
(`%LocalAppData%/llm_wiki`) carry the `llm-wiki`/`llm_wiki` identity directly, so
namespacing only the managed home would leave the test instance sharing
production's registry and caches — quietly breaking the isolation guarantee.
These roots are therefore part of the namespace surface and the golden snapshot.

The SKILL.md **frontmatter `name:` must be suffixed**, not just the directory:
that name is what the live Claude/Codex session registers, so an unsuffixed name
would collide with the real `wiki` skill in the same session. Suffixing it is
what lets both sets coexist and be told apart at invocation time.

**MCP/tool naming is decided here, not deferred:** trailing `_test`
(`llm_wiki_search_test`, `llm_wiki_search_all_test`). These names are part of
the integration surface, so they are settled in the proposal rather than during
implementation. The stakes are presently low — those tool names are *reserved
and not yet built* (the Headroom decision lists them as "reserved future MCP
names"), so suffixing them now is a forward-compatible naming choice, not a
breaking change to a live surface — but the spelling is fixed now so the plan
inherits it.

### Production-identity invariant

The invariant is **output equality, not byte equality**. Adding the derivation
code changes the compiled binary, so "byte-for-byte identical binary" is the
wrong claim. The enforceable, load-bearing property is: **with an empty
`INSTANCE`, the binary produces exactly today's names, paths, skill payloads,
manifests, and install outputs.** This must be guarded by test, not asserted in
prose:

- A **golden empty-namespace install snapshot**: the names, paths, projected
  skill payloads (SKILL.md + Codex YAML bytes), and written manifest for an
  empty-instance install are pinned and must match the pre-feature baseline.
- A companion **golden `test`-namespace snapshot** pins the suffixed outputs so
  the namespacing itself does not silently drift.
- A **scoped source lint** (close criterion below) prevents new hard-coded
  identity strings from bypassing the derivation API and re-introducing a
  literal `"llm-wiki"`/`".llm_wiki"` at a name-minting site.

Releases never set `LLM_WIKI_INSTANCE` (and the release workflow rejects it), so
production behavior is unaffected by this feature existing.

### Coexistence and runtime proof

Mirroring the proof matrix from the skill-rename proposal, promotion requires
demonstrating in the author's real `$HOME` (not a throwaway one). The same-`$HOME`
proof is only credible if production untouchedness is proven **mechanically**,
so the matrix is bracketed by snapshots:

0. **Pre-snapshot.** Capture a manifest of production state before any test
   action: `~/.llm_wiki/manifest.json`, the production skill directories under
   `~/.claude/skills/` and `~/.codex/skills/`, the managed binary, and the search
   caches/indexes — recorded as a content hash
   set.
1. **Build.** `LLM_WIKI_INSTANCE=test cargo build` yields `llm-wiki-test`; a
   plain `cargo build` still yields production `llm-wiki`, and the golden
   empty-namespace snapshot matches the pre-feature baseline.
2. **Install side-by-side.** `llm-wiki-test install` writes `~/.llm_wiki-test/`,
   `~/.claude/skills/wiki-test/`, `~/.codex/skills/wiki-test/`, etc.; the
   production pre-snapshot hash set is **re-verified unchanged**.
3. **Discovery.** A live Claude Code (and Codex) session lists *both* the
   production `wiki*` skills and the `wiki*-test` skills. Discovery preconditions
   are stated explicitly: skill listing may require a session restart or skill-
   cache refresh after install, so the proof records the exact step taken (e.g.
   new session) rather than assuming instant pickup.
4. **Invocation.** Invoking a `*-test` skill reaches the `llm-wiki-test` binary
   and reads/writes only `~/.llm_wiki-test/` state.
5. **Cleanup + post-snapshot.** `llm-wiki-test uninstall` removes only the test
   namespace (`~/.llm_wiki-test`, `*-test` skills). The production pre-snapshot
   hash set is re-verified a final time and must be **byte-identical** to step 0
   — "uninstall removes only test" is proven by the snapshot diff, not by
   inspection.

## Why

1. **The live-session gap is the whole point.** Swapping `$HOME` isolates state
   but the running Claude Code session still reads skills from the real
   `~/.claude/skills`, so it cannot exercise the *installed* skills/commands as
   the agent actually invokes them. A coexisting, renamed instance is the chosen
   lowest-risk way to validate the real invocation surface without disturbing
   production — short of heavier alternatives (a separate OS user, an alternate
   skill root, or a PATH shim) that this proposal rejects as more operational
   overhead for the same outcome.
2. **Seamless beats flagged.** A baked-in namespace means no per-command flag to
   remember and no failure mode where a forgotten flag points the test binary at
   production. Isolation is a property of the binary, not of operator
   discipline.
3. **Unblocks honest dogfooding.** Any future install-surface change needs a
   live agent through the real harness. This gives a safe place to do that.
4. **Reuses proven plumbing.** The skill-rename work already established how
   names flow from embedded source through the projector to install paths and
   the manifest. This is the same plumbing parameterized by one constant rather
   than hard-coded once.
5. **No production behavior change, at a bounded maintenance cost.** Empty
   namespace = today's outputs, enforced by the golden snapshot. The honest
   framing is *not* "zero cost": every future identity string now has to flow
   through the derivation API and is guarded by a source lint, which is real
   ongoing maintenance surface. The trade is that contained cost for a
   test-enforced production guarantee.

## Alternatives Considered

1. **Runtime `--instance <name>` flag (no bake-in).** Rejected as the primary
   mechanism: every command would need the flag, and a forgotten flag silently
   targets production — exactly the fat-finger risk the author wants gone. The
   baked-in constant makes mis-targeting impossible.
2. **`$HOME` swap / sandbox wrapper.** Fully isolates state and needs no code,
   but cannot validate skills/commands inside the *current* live agent session,
   which is the motivating requirement. Good for install mechanics, insufficient
   here. Worth keeping as the quick-check path.
3. **Separate git checkout with its own `$HOME`.** Heavyweight and still hits the
   same live-session gap; the running session reads one skills dir.
4. **Containerized lane.** Proves the
   product against a real dependency in isolation, but a container is not the
   author's interactive Claude/Codex session, so it does not validate the live
   invocation experience.
5. **Cargo feature flag instead of an env var.** Equivalent mechanism; a build
   env var read via `option_env!` is lighter (no feature matrix, no risk of the
   feature leaking into a release profile) and keeps the production build the
   literal default. Lean env var; revisit if a feature gate proves cleaner.
6. **Arbitrary instance names in V1.** Rejected: arbitrary strings need
   validation across filenames, Windows paths, skill names, MCP identifiers,
   manifests, and docs, with no second consumer to justify it. The mechanism
   stays general internally; only `test` is reachable and proven.

## Consequences and Tradeoffs

- **Project working trees are NOT isolated — only managed state is.** This is the
  loudest caveat. Isolation covers the managed home, installed skills, registry,
  caches, and manifest. It does **not** cover the project directories an instance
  operates on: `wiki-test ingest` run against the current project can still
  mutate that project's `wiki/` and `raw/` exactly as production would. The test
  instance protects your *install*, not your *content*. Operators must point the
  test instance at a scratch/throwaway project (or accept the writes) when
  exercising mutating skills; the plan should surface this in `doctor`/skill
  prose, not bury it.
- **New compile-time surface and maintenance cost.** The binary name, paths,
  skill names, MCP names, and manifest identity become functions of one constant
  instead of string literals — a refactor with blast radius across the
  path/skill/install layers, guarded by the golden snapshot and source lint.
- **Two installs to keep tidy.** The author can have production and `test`
  installed at once. `status`/`doctor` are already managed-home-scoped, so each
  binary reports only its own instance; uninstall is per-instance. Docs should
  make the two-instance model explicit.
- **Skill list grows in live sessions.** While a test instance is installed, the
  agent sees both `wiki*` and `wiki*-test` skills — intended coexistence, but
  visible clutter until `uninstall`, and possibly only after a session restart.
- **Not a multi-tenant feature.** A developer/dogfooding affordance for one
  author, hard-restricted to `test`. The design must not grow tenancy semantics.

## What Closes This Proposal

Promotion to a decision plus an execution plan covering:

1. **Instance constant + output-equality invariant first.** Introduce `INSTANCE`
   via `option_env!("LLM_WIKI_INSTANCE")` (rejecting any non-empty value other
   than `test` at build time), thread it through one derivation API, and land the
   golden empty-namespace install snapshot proving the empty build reproduces
   today's names, paths, skill payloads, and manifests exactly. Nothing else
   proceeds until this is green.
2. **Name derivation.** Parameterize `managed_binary_name`, the managed home, the
   **XDG `cache_home`/`data_home` roots and the project registry, the Windows
   `%LocalAppData%` root**, the CLI command name, `apply_binary_context`, the
   embedded skill names and SKILL.md frontmatter projection, the reserved MCP/tool
   names (trailing `_test`), and `installed_by`
   through that one API. Emit `cargo:rerun-if-env-changed=LLM_WIKI_INSTANCE` in
   `build.rs` so the env change forces a rebuild.
3. **Source lint against hard-coded identities.** Add a scoped check (CI + `just`)
   built from a complete identity inventory: `"llm-wiki"`, `".llm_wiki"`,
   `"llm_wiki"` (Windows root, no dot), and the reserved MCP names, flagged at any
   name-minting site outside the
   derivation API. Scope it to those sites — not every occurrence — so legitimate
   prose, help text, and fixtures are not flagged.
4. **Release-build guard.** A durable guard (not just the dist-autogenerated CI
   workflow) asserts `LLM_WIKI_INSTANCE` is empty and fails otherwise across every
   release entrypoint — the CI workflow, `just release-build`/`release-plan`
   (which call `dist` directly), and `release-e2e` — so a `-test` artifact can
   never ship.
5. **Install/uninstall/doctor.** Install writes the namespaced skill dirs and
   home, the manifest round-trips through `status`, doctor reports against the
   instance home and surfaces the shared-project-tree caveat, and uninstall
   removes only the instance subtree.
6. **Runtime proof matrix** (steps 0–5 above, snapshot-bracketed) executed in the
   author's real `$HOME` under both Claude Code and Codex, with results — including
   the discovery precondition actually used — recorded in the plan.
7. **Docs.** A short reference (or a section in the Headroom reference) on the
   two-instance model, the shared-project-tree caveat, and the
   `LLM_WIKI_INSTANCE=test` build, plus index/log updates.

## Open Questions

1. **Optional runtime convenience flag (post-V1).** Whether to later expose a
   runtime `--instance` that *defaults to* the baked-in constant, purely for
   discoverability. Lean: defer; the baked-in value is the contract and a flag
   reintroduces the mis-targeting surface this proposal removes.
2. **In-repo mirrors.** Whether `build --out .` should ever emit namespaced
   mirrors, or whether the instance is global-install-only. Lean:
   global-install-only; the repo mirrors stay production.
3. **Shared-project-tree ergonomics.** Whether the test instance should go beyond
   a warning — e.g. refuse mutating skills unless the target project is flagged
   as scratch. Lean: warn in V1, revisit if accidental writes happen in practice.
