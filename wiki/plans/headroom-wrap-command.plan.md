# Plan: Headroom No-Legacy Reduction + `llm-wiki headroom wrap`

- Document Class: Plan
- Status: Completed
- Date: 2026-07-07
- Category: Agent runtime, Headroom, wiki reduction, developer ergonomics
- Implements: wiki/decisions/headroom-single-posture-mcp-first.decision.md (Accepted; amended 2026-07-07)
- Scope: Three ordered phases, now implemented. **Phase 0** — purge the exploratory proxy wiki
  down to a minimal self-contained set, folding load-bearing rationale into the
  surviving decision. **Phase 1** — strict no-legacy removal of the last Headroom
  code and tests (no shim, no absence tests). **Phase 2** — implement
  `llm-wiki headroom wrap`, covering **both** production and test MCP namespaces.
- Sources:
  - wiki/decisions/headroom-single-posture-mcp-first.decision.md (Accepted; amended 2026-07-07)
  - wiki/evals/test-instance-live-session-proof.eval.md (prod/test coexistence)
  - src/instance.rs, src/cli.rs, src/main.rs, src/mcp/mod.rs
  - src/install.rs, src/manifest/schema.rs (Phase 1 removal targets)
  - Two 2026-07-07 read-only audits (code + wiki)
- Related:
  - wiki/references/headroom-context-compression.reference.md

## Supersession Note (2026-07-18)

Phase 2's command process model is superseded by
`wiki/plans/headroom-passthrough-launcher.plan.md`. The implemented
`llm-wiki headroom wrap -- <command>` env-injected an arbitrary child command,
which did not start Headroom's proxy for real `codex` / `claude` sessions. The
current surface is `llm-wiki headroom [--headroom-bin <PATH>]
[--unsafe-mcp-read] [--] <headroom args...>`: it exports the same guard env and
then execs the real `headroom` binary, forwarding `wrap codex`, `proxy --port
8790`, or any other Headroom-owned args to Headroom itself.

## Where This Stands (2026-10-06)

Completed 2026-07-07: all three phases and the acceptance list in "Sequence &
Acceptance" are done, with the evidence in "Implementation Evidence
(2026-07-07)". It keeps the bare `Completed` of plans finished before
2026-10-06 (`wiki/decisions/work-in-flight-is-a-pushed-branch.decision.md`,
rule 5). Phases 0 and 1 still stand on master; Phase 2's
`llm-wiki headroom wrap` command does not, because the passthrough launcher
replaced it (see the Supersession Note). Nothing is pending here.

## Principle

After this plan the Headroom *surface* is exactly: the `headroom wrap` command,
the one harness-neutral safety rule in generated guidance, and the kept wiki set
(the single consolidated decision, the trimmed reference, this plan, and the two
current accepted pure-MCP field-test evals as proof the surface works). No shim,
no proxy apparatus, no superseded proxy plan, no candidate proxy evidence
survives "just in case."

Note on mixed docs: several non-Headroom docs (the hybrid/MCP-surface repair
plan, the test-instance binary plan/decision, the MCP merge-readiness plan and
review) are kept for their MCP-cutover, test-instance, and migration value.
Their proxy/`--with-headroom` content is *stripped* (Phase 0d, hard trim); any
residual "Headroom" text in them is limited to the branch/topic name, a valid
link to the kept decision or reference, or captured error output — not a live
proxy concept.

---

## Phase 0 — Wiki Reduction (do first) — ✓ done 2026-07-07

### 0a. Make the decision self-contained (before any deletion)

Enrich the consolidated `headroom-single-posture-mcp-first.decision.md` so nothing
load-bearing is lost when the evidence docs are deleted:

- Inline the two facts currently carried by evals: (1) the 2026-07-06 Codex
  proxy retry returned an uncompressed read, credited to `HEADROOM_MCP_READ=off`
  + MCP routing, **not** the exclude list; (2) the earlier `CARVE-OUT FAIL` had
  its host attribution corrected from Codex to Claude, so excludes are treated as
  unproven on Codex, not impossible.
- Add a **"Proper Configuration Interface (Future)"** section: if Headroom exposes
  path-based read exclusion or a stable config surface naming protected tools,
  the framework replaces env hand-assembly with that interface and this
  convenience is simplified or removed. That is the concrete exit condition.

### 0b. Keep (the minimal surviving set)

- `decisions/headroom-single-posture-mcp-first.decision.md` (Accepted; amended
  2026-07-07) — the one consolidated Headroom decision (posture + wrap +
  no-legacy + future interface), enriched so it stands alone.
- `references/headroom-context-compression.reference.md` — trim to the single
  posture + one line on the `wrap` convenience.
- `plans/headroom-wrap-command.plan.md` — this plan.
- `evals/codex-mcp-field-test-pass.eval.md`,
  `evals/claude-mcp-field-test-pass.eval.md` — current proof the MCP surface
  works (2026-07-07, proxy-off).

### 0c. Delete (pure proxy exploration — after 0a folds anything load-bearing)

- `plans/headroom-external-convenience-launcher.plan.md`
- `plans/headroom-runtime-companion.plan.md`
- `plans/mcp-first-headroom-profile-narrowing.plan.md`
- `plans/headroom-single-posture-cleanup.plan.md` (cleanup is done; the decisions
  record it)
- `proposals/headroom-runtime-companion.proposal.md`
- `decisions/headroom-runtime-companion.decision.md` (fix the dangling
  `Supersedes:` pointer in the single-posture decision to note it was removed)
- `evals/headroom-codex-mcp-read-uncompressed.eval.md` (Candidate; its one fact
  folded into the decision in 0a)
- `evals/codex-mcp-field-test-proxy-off-pass.eval.md` (superseded by the cleaner
  2026-07-07 pass)
- `evals/claude-mcp-field-test-proxy-on-invalid.eval.md` (Rejected proxy run)
- `review/mcp-field-test-proxy-off-and-launcher-defect.eval.md` (five-run proxy
  field test; its conclusion already lives in the single-posture "Why")

### 0d. Hard-trim, do not delete (mixed proxy + valuable non-Headroom content)

Strip *all* proxy / `--with-headroom` / carve-out / router-bypass / Mode-C
material; keep the non-Headroom value verbatim; renumber lists and fix
cross-references so each doc still reads cleanly.

- `plans/hybrid-default-and-mcp-surface-repair.plan.md` — remove the proxy
  launcher/carve-out phases (old Phases 2–3) and their findings/acceptance
  criteria; keep the hybrid-out-of-the-box, `search-all`, MCP `status`, and
  raw-discoverability work (the hybrid-search differentiator).
- `plans/headroom-mcp-merge-readiness-repair.plan.md` — remove the proxy
  carve-out / router-bypass phases and the "Global Headroom Discovery" quickfix
  (about the removed `--with-headroom` installer); keep the MCP namespacing /
  test-instance / migration repairs.
- `review/headroom-mcp-branch-merge-readiness.eval.md` — keep the test-instance /
  MCP-config-collision findings; drop the superseded proxy blockers and the
  `--with-headroom` proof items.
- `plans/test-instance-namespaced-binary.plan.md`,
  `decisions/test-instance-namespaced-binary.decision.md` — remove the incidental
  Headroom-profile / `--with-headroom` dogfood mentions; keep the namespaced
  binary identity and MCP-derivation content.

### 0e. Rebuild bookkeeping

- Regenerate the Headroom section of `wiki/index.md` to list only the kept set.
- Append one `wiki/log.md` entry recording the reduction (what was deleted and
  why: no-legacy branch posture).

### 0f. Phase 0 gate

The wiki's Headroom footprint is the kept set only; every surviving link
resolves; the two accepted decisions read standalone with no reference to a
deleted doc.

---

## Phase 1 — Strict No-Legacy Code + Test Removal — ✓ done 2026-07-07

- **Delete `infra/headroom-proxy-e2e/`** (orphaned `__pycache__` only).
- **Remove the compat shim** (`--with-headroom` never shipped, so nothing to
  clean up):
  - `src/manifest/schema.rs`: drop `ManagedAssetKind::Unknown` + `#[serde(other)]`
    → `{ McpConfig }`; delete its unrecognized-kind test.
  - `src/install.rs`: remove the unrecognized-asset cleanup loop (~163–190).
- **Delete `tests/headroom_install.rs` entirely** — no Headroom text in tests,
  including the absence guards (your call: strict).
- **Forward-compat note (accepted):** unknown asset kinds now fail to deserialize
  instead of degrading; acceptable for a managed, versioned manifest.

### Phase 1 gate

`cargo build` / `test` / `clippy -D warnings` / `insta` green; a `headroom`/
`HEADROOM_` grep over `src/` returns **nothing**, and over `tests/` returns only
the `tests/snapshots/*.snap` rendering of the kept harness-neutral safety rule
(sourced from `templates/base/project_guidelines.md`, which is *not* removed).
Phase 2 adds the command next.

---

## Phase 2 — `llm-wiki headroom wrap` — ✓ done 2026-07-07

### 2a. CLI + module

- `src/cli.rs`: `Command::Headroom(HeadroomArgs)` → `HeadroomCommand::Wrap`
  (wrap-only for the first cut; no `env` printer).
- `src/headroom.rs` + `src/main.rs` dispatch, matching existing command signatures.

```bash
llm-wiki headroom wrap -- headroom proxy --port 8790
```

### 2b. Exclude set covers BOTH namespaces

Per `test-instance-live-session-proof.eval.md`, production and test instances
coexist. The exclude list emits **both** namespaces so a Headroom session with
either or both llm-wiki MCP servers configured is covered:

- production: `llm_wiki_*`, server id `llm-wiki`;
- test: `llm_wiki_*_test`, server id `llm-wiki-test`;
- each with defensive fully-qualified `mcp__<server>__<tool>` forms in hyphenated
  and underscored server-id spellings.

Derived from the six tool bases + the `instance.rs` naming rules (extended to
emit both instances rather than only the running one).

### 2c. Env semantics (review Finding 5)

- Default: set `HEADROOM_MCP_READ=off` on the child (the guaranteed invariant).
- `--unsafe-mcp-read`: **`env_remove("HEADROOM_MCP_READ")`** on the child — absent,
  never inherited — so a parent `HEADROOM_MCP_READ=on` cannot leak through. Blunt
  warning printed.
- Always set `HEADROOM_EXCLUDE_TOOLS` (best-effort/defensive, never a guarantee).

### 2d. Process model + framing

- `exec` the wrapped argv (unix); spawn + wait + forward exit code (non-unix).
- No "wiki-safe" language (review Finding 3). Stderr warning states provenance
  comes from routing through `llm_wiki_*` tools; the exclude list is not a
  provenance boundary (unproven on Codex).

### 2e. Out of scope (per decision)

Proxy start/health/teardown, `doctor`, strict-provenance, host smoke, base-URL
injection, config files, the `env` printer.

### 2f. Tests (review Finding 6)

- Exclude set contains all six tools in **both** prod and test namespaces + a
  fully-qualified route form.
- Default sets `=off`; `--unsafe-mcp-read` yields a child env with
  `HEADROOM_MCP_READ` absent **even when the parent has `=on`**.
- Clap: trailing args after `--`, hyphenated child args, empty command rejected.
- Process: exec/spawn failure exits nonzero with context; non-unix exit-code
  forwarding.
- ~~Resolve the `init_baseline` snapshot shift a prior spike caused *before*
  re-accepting it — confirm whether generated guidance enumerates CLI commands.~~
  **Resolved 2026-07-07:** the shift was *purely* the `0.2.7 → 0.2.8`
  `framework_version` bump in the init profiles; generated guidance does **not**
  enumerate CLI commands, so `wrap` will not perturb these snapshots.

---

## Sequence & Acceptance

1. ✓ Phase 0 green (wiki reduced, decisions standalone) — done 2026-07-07.
2. ✓ Phase 1 green (zero Headroom in code/tests; build/test/clippy/insta green) —
   done 2026-07-07.
3. ✓ Phase 2 green (wrap works for prod + test namespaces) — done 2026-07-07.
4. ✓ This plan flipped to Implemented; log updated.

## Review Items (resolved 2026-07-07)

1. **Purge scope** — confirmed **Headroom/proxy-only**; a broader wiki prune is a
   separate pass.
2. **Delete list (0c)** — accepted as written; no doc was retained as extra
   history beyond the kept set in 0b.
3. **Decision consolidation** — consolidated into the single
   `headroom-single-posture-mcp-first.decision.md` (posture + wrap + no-legacy +
   future interface); no separate amending launch-convenience decision is kept.

All phases are complete.

## Implementation Evidence (2026-07-07)

Implemented Phase 2 as a narrow in-binary launch convenience:

- Added `llm-wiki headroom wrap -- <command> [args...]`.
- The default child environment sets `HEADROOM_MCP_READ=off`.
- `--unsafe-mcp-read` removes `HEADROOM_MCP_READ` from the child environment
  rather than inheriting a parent value.
- The child always receives `HEADROOM_EXCLUDE_TOOLS` with all six MCP tool bases
  in production and test namespaces, plus defensive fully-qualified
  `mcp__<server>__<tool>` entries for hyphenated and underscored server names.
- Unix uses `exec`; non-Unix uses spawn/wait and forwards the child exit code.
- Warnings avoid “wiki-safe” framing and state that the exclude list is
  best-effort, not a provenance boundary.

No-legacy sweep verification before Phase 2:

- Removed stale active wiki references to deleted Headroom profile/install tests
  and proxy-mode vocabulary outside retained decision/reference/history notes.
- `src/`, `tests/`, and `infra/` contain no removed `--with-headroom`,
  `ManagedAssetKind::Unknown`, `headroom_install`, `headroom_profile`,
  proxy-launcher, or router-bypass symbols.

Verification passed:

- `rtk cargo fmt`
- `rtk cargo test --test headroom` — 6 passed.
- `rtk cargo test headroom` — 4 passed, 313 filtered out.
- `rtk cargo test --test identity_lint` — 3 passed.
- `rtk cargo check`
- `rtk cargo clippy -- -D warnings`
- `rtk cargo test` — 315 passed, 2 ignored.
