# Plan: Managed Binary Runtime Install

- Document Class: Plan
- Status: Completed
- Date: 2026-05-06
- Category: Distribution tooling, install UX
- Scope: Implement D8.1 managed binary install behavior so runtime skills call `~/.llm_wiki/bin/llm-wiki` by absolute path and PATH setup is only convenience guidance.
- Sources: wiki/proposals/binary-path-bootstrap.proposal.md, wiki/decisions/binary-path-bootstrap.decision.md, wiki/decisions/llm-wiki-binary-distribution.decision.md
- Related: wiki/roadmaps/framework-v1.roadmap.md (D8.1), wiki/specs/documentation-model.spec.md, wiki/specs/knowledge-init-skill.spec.md

## Deliverable

D8.1: Managed binary runtime install. `llm-wiki install` copies or verifies the
running binary under `~/.llm_wiki/bin/llm-wiki`, renders installed skills to
call that managed path, writes manifest v2 under `~/.llm_wiki/`, and reports
PATH only as terminal convenience.

All binary acquisition paths must converge on this same install behavior. The
release installer may invoke or offer to invoke `llm-wiki install`, but it must
not duplicate skill-copy, manifest, backup, or managed-binary logic. `cargo
install llm-wiki-framework` cannot rely on a Cargo post-install hook, so its
documented completion step is:

```bash
cargo install llm-wiki-framework
llm-wiki install
```

This plan executed the accepted decision. Specs were promoted after the
implementation landed and verification passed.

## Existing Implementation Touchpoints

The load-bearing rendering change starts in the current installer, not in a new
renderer. At implementation start, inspect these sites first:

- `src/install.rs`: `render_install_files` computes installed skill files.
- `src/embed.rs`: embedded asset registry names skills and `include_str!`
  paths.
- `crates/llm-wiki-schema/src/projector/`: Claude and Codex projection logic.
- `crates/llm-wiki-schema/tests/real_skills.rs` and snapshots: real canonical
  skill projection coverage.
- `tests/install.rs`, `tests/post_install.rs`, and `tests/status_doctor.rs`:
  redirected-`HOME` integration harnesses for install, manifest, doctor, and
  uninstall behavior.

## In Scope

- Managed runtime home at `~/.llm_wiki/`.
- Managed binary at `~/.llm_wiki/bin/llm-wiki` on Unix-like systems.
- Future-compatible Windows path helpers for `%LOCALAPPDATA%\llm_wiki\bin\llm-wiki.exe`.
- Manifest v2 at `~/.llm_wiki/manifest.json`.
- `sha256` hashing for all manifest and backup entries.
- `install.partial.json` transaction marker and interrupted-install recovery.
- Self-install behavior: skip copy when running from the managed binary path.
- Skill rendering to managed absolute binary paths.
- Scoped backup snapshots under `~/.llm_wiki/backups/`.
- `llm-wiki path`.
- `doctor` checks for managed binary state, stale transaction state, and PATH
  visibility as convenience status.
- `uninstall --include-binary`, with default uninstall leaving the managed binary in place.
- Release installer and `cargo install` documentation that route users through
  the same `llm-wiki install` behavior.
- `init-project` to `knowledge-init` rename as a direct pre-release rename.

## Out Of Scope

- Automatic shell profile edits.
- Windows release artifacts.
- Self-update.
- Automated rollback command.
- Specs promotion before behavior is implemented and tested.

## Phases

### 0. Rendering and test-harness spike

1. Confirm the current installed skill command strings are produced through
   `src/install.rs::render_install_files`, `src/embed.rs`, and the
   `llm-wiki-schema` projectors.
2. Decide whether managed binary path injection belongs in projector input,
   rendered skill context, or install-time rendering parameters. Do not use a
   blind post-process pass over rendered Markdown unless no structured option
   exists.
3. Confirm the outside-PATH integration test can reuse the existing
   `assert_cmd` + `tempfile::TempDir` + redirected `HOME` pattern from
   `tests/install.rs` and `tests/post_install.rs`.
4. Identify every hard-coded skill name and embedded asset path that the
   `init-project` to `knowledge-init` rename must update.

Verification: add a short Phase 0 note to this plan, under a new
`Implementation Notes` section, identifying the exact rendering entry point,
test harness pattern, and rename touch list before code changes begin.

### 1. Manifest v2 and hashing

1. Add manifest v2 structs for binary, skills, backups, ownership, and
   `hash_algorithm`.
2. Pin all hashing helpers to `sha256`.
3. Add round-trip JSON tests for the manifest shape.

Verification: manifest v2 round-trips; every hash entry records
`hash_algorithm: "sha256"`. The schema commits to per-entry algorithm fields
for forward compatibility and to keep records explicit.

Manifest v2 sketch:

```json
{
  "schema_version": 2,
  "installed_by": "llm-wiki",
  "installed_at": "2026-05-06T12:34:56Z",
  "binary": {
    "path": "/Users/alice/.llm_wiki/bin/llm-wiki",
    "version": "0.1.0",
    "hash_algorithm": "sha256",
    "hash": "0123456789abcdef...",
    "ownership": "manifest-owned"
  },
  "skills": [
    {
      "runtime": "codex",
      "name": "knowledge-query",
      "path": "/Users/alice/.codex/skills/knowledge-query/SKILL.md",
      "kind": "skill",
      "hash_algorithm": "sha256",
      "hash": "abcdef0123456789...",
      "ownership": "manifest-owned"
    }
  ],
  "backups": [
    {
      "id": "install-20260506T123456Z",
      "path": "/Users/alice/.llm_wiki/backups/install-20260506T123456Z/backup-manifest.json"
    }
  ]
}
```

`install.partial.json` sketch:

```json
{
  "schema_version": 1,
  "started_at": "2026-05-06T12:34:50Z",
  "current_exe": "/Users/alice/Downloads/llm-wiki",
  "target_binary": "/Users/alice/.llm_wiki/bin/llm-wiki",
  "current_exe_hash_algorithm": "sha256",
  "current_exe_hash": "0123456789abcdef...",
  "phase": "binary-copy"
}
```

A partial marker is stale when it references a different managed target path,
records a current executable hash that does not match the binary being
installed, or coexists with a final manifest whose managed binary hash does not
match the marker. A partial marker that remains after a successful atomic
manifest write is a marker leak: doctor should report it as cleanup-needed if
the manifest and managed binary hashes match, not as an interrupted install.

### 2. Managed runtime paths

1. Add platform-aware managed home and managed binary path helpers.
2. Keep helpers testable under redirected `HOME` and simulated Windows inputs.
3. Add quoting tests for rendered absolute command paths.

Verification: Unix paths, Windows `.exe` naming, quoting, and PATHEXT lookup
assumptions are covered at unit level.

### 3. Binary copy and transaction recovery

1. Resolve the running executable with `std::env::current_exe()`.
2. Abort before modifying managed state if resolution or file read fails.
3. Write `~/.llm_wiki/install.partial.json` before copying.
4. Copy to the managed binary path unless running from that path.
5. Verify copied or in-place binary by `sha256`.
6. Write the final manifest via temporary file plus rename in the managed
   runtime home.
7. Delete the partial marker after the final manifest write succeeds.

Verification: current-exe failure aborts cleanly; self-install skips
copy-over-self; interrupted installs recover or reject according to documented
collision rules. If a crash happens after manifest rename but before partial
marker deletion, doctor classifies the state as a completed install with a
marker leak when manifest and binary hashes match.

### 4. Skill install and backups

1. Create a scoped backup snapshot before replacing known framework skill paths.
   With no public migration surface, the backup protects user-authored content
   or local dogfood edits at framework skill paths; the empty snapshot proves
   the protection step ran even on first install.
2. Render installed skills so binary calls use the managed absolute binary path.
   Implement this at the structured rendering boundary identified in Phase 0,
   preferably by passing managed binary path context into projection/install
   rendering rather than by text replacement after rendering.
3. Keep existing manifest-owned collision discipline and `--force` behavior.
4. Always write a backup manifest, even for empty snapshots.

Verification: installed skills contain the managed path, not bare `llm-wiki`;
backup manifests are written for populated and empty snapshots.

### 5. PATH guidance command

1. Keep default install PATH guidance after successful install only.
2. Add `llm-wiki path` for guidance without reinstalling skills.
3. Detect Unix shell profile from `$SHELL` when clear; otherwise print zsh,
   bash, and fish examples.

Verification: missing PATH never fails install; `llm-wiki path` does not write
skills or manifests.

### 6. Doctor and uninstall

1. Update `doctor` to verify managed binary presence, `sha256`, manifest v2,
   stale `install.partial.json`, and installed skill command paths.
2. Compare any `which`-resolved `llm-wiki` against the managed binary. If the
   `PATH` binary and managed binary differ, warn that terminal invocations and
   installed skills may be using different versions.
3. Update uninstall so default removal removes manifest-owned skills and Codex
   `agents/openai.yaml` runtime config files under installed skill directories,
   but leaves the managed binary in place.
4. Add `uninstall --include-binary` for explicit managed binary removal.

Verification: doctor reports drift accurately; uninstall removes only
manifest-owned state unless `--include-binary` is passed.

### 7. Skill rename

1. Rename canonical asset `assets/skills/init-project/` to
   `assets/skills/knowledge-init/`.
2. Update Rust references that name the skill or embedded path, including
   `src/embed.rs`, install target generation, projector tests, real-skill
   snapshots, compatibility tests, and scaffolding or dispatcher references.
3. Update generated Claude and Codex skill targets.
4. Route `$knowledge init` to `knowledge-init`.
5. Run `rg -n "init-project" wiki assets src crates tests` and classify every
   remaining mention as historical documentation, current spec text pending
   promotion, or a bug to fix. This explicitly includes
   `wiki/specs/knowledge-init-skill.spec.md` and any `Sources` / `Related`
   metadata that names it.

Verification: no runtime asset, generated skill, embed path, dispatcher route,
or test snapshot still depends on `init-project`; specs are left untouched until
promotion.

### 7.5 Documentation updates

1. Update installation documentation so the Cargo path is explicitly:

   ```bash
   cargo install llm-wiki-framework
   llm-wiki install
   ```

2. Update release installer copy or generated installer guidance so the
   installer invokes or offers to invoke `llm-wiki install`.
3. Update README, release notes, and any cargo-dist installer text that still
   implies acquiring the binary alone installs runtime skills.
4. Document that `which llm-wiki` may resolve to the Cargo or release-installer
   binary while installed skills intentionally call `~/.llm_wiki/bin/llm-wiki`.

### 8. End-to-end gates

1. Run a manually downloaded binary from outside `PATH` under redirected `HOME`.
2. Install without shell profile edits.
3. Invoke an installed skill or skill-equivalent stub.
4. Confirm the stub executes the managed binary path.
5. Verify the `cargo install llm-wiki-framework` documented path ends in
   `llm-wiki install` and uses the same install code path.
6. Verify the release installer invokes or offers to invoke `llm-wiki install`
   rather than implementing separate install behavior.
7. Implement the outside-PATH test with the existing Rust integration-test
   harness: `assert_cmd` for the binary under test, `tempfile::TempDir` for
   `HOME`, and a sanitized `PATH` that omits the binary location.
8. Run full local verification: format, tests, clippy, snapshots, coverage.

Verification: the headline outside-PATH behavior is proven by an integration
test, not by install output snapshots alone.

## Spec Promotion

After implementation and tests pass:

1. Update `wiki/specs/documentation-model.spec.md` with the managed binary
   runtime home and manifest v2 as validated behavior.
2. Replace `wiki/specs/knowledge-init-skill.spec.md` with the validated
   `knowledge-init` skill spec.
3. Update affected `knowledge-*` skill specs if dispatcher or installed path
   behavior changes.
4. Mark this plan `Completed`.
5. Mark D8.1 `Completed` on the roadmap.

## Risks

| Risk | Impact | Mitigation |
| --- | --- | --- |
| Interrupted install leaves confusing managed state | High | `install.partial.json`, atomic manifest write, and doctor recovery checks |
| Self-install copy behavior differs by platform | Medium | Explicit verify-in-place branch |
| Rename and managed binary install interact badly | Medium | Rename runs as a separate phase after managed binary state is coherent |
| Shell profile guidance misidentifies the active shell | Low | `$SHELL` caveat and fallback examples for common shells |
| Existing specs appear stale during implementation | Medium | Leave specs unchanged until behavior is proven; decision and plan carry pending direction |

## Implementation Notes

Phase 0 findings:

- Rendering entry point: `src/install.rs::render_install_files` parses each
  `embed::SKILLS` entry, projects it through `llm-wiki-schema`, and constructs
  the installed `InstallFile` list. Managed binary path injection should happen
  in or immediately below this function by passing install context into
  rendering; do not add a blind Markdown post-process after files are rendered.
- Asset registry: `src/embed.rs` hard-codes each skill name and `include_str!`
  path. The `knowledge-init` rename must update this registry.
- Projector tests: `crates/llm-wiki-schema/tests/real_skills.rs` and snapshot
  names include `init-project`; the rename must update fixtures and snapshots.
- Integration harness: use the existing `assert_cmd` plus `tempfile::TempDir`
  plus redirected `HOME` pattern from `tests/install.rs` and
  `tests/post_install.rs`; add a sanitized `PATH` for the outside-PATH case.
- Current manifest path: `src/paths.rs::manifest` still points at
  `~/.local/share/llm-wiki/manifest.json`; D8.1 changes it to
  `~/.llm_wiki/manifest.json`.
- Rename touch list: `assets/skills/init-project/`, `src/embed.rs`,
  `assets/skills/knowledge/SKILL.md`, `tests/build.rs`, `tests/compat.rs`,
  `tests/install.rs`, `tests/post_install.rs`, `tests/properties.rs`,
  `tests/status_doctor.rs`, `crates/llm-wiki-schema/tests/real_skills.rs`,
  and the related snapshots.
