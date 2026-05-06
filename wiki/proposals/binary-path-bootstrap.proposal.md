# Managed Binary Install and PATH Guidance

- Document Class: Proposal
- Status: Proposed
- Date: 2026-05-06
- Category: Distribution tooling, install UX
- Scope: Make `llm-wiki install` create a stable managed binary location for runtime skills, then guide users when `llm-wiki` is not discoverable on `PATH`.
- Sources: user discussion 2026-05-06, proposal review 2026-05-06, wiki/decisions/llm-wiki-binary-distribution.decision.md, wiki/plans/llm-wiki-binary.plan.md, wiki/plans/llm-wiki-product-layout-addendum.plan.md, https://docs.rs/which/latest/which/, https://docs.rs/dirs-next/latest/dirs_next/fn.executable_dir.html, https://docs.rs/dialoguer/latest/dialoguer/, https://docs.rs/is-terminal/latest/is_terminal/, https://doc.rust-lang.org/stable/cargo/commands/cargo-install.html
- Related: wiki/specs/init-project-skill.spec.md, wiki/specs/documentation-model.spec.md, wiki/proposals/llm-wiki-binary.proposal.md

## Question

How should `llm-wiki install` ensure installed skills can reliably call the
`llm-wiki` binary, while still helping users get a convenient command on
`PATH`?

## Problem

The current skill installation model assumes the binary is already installed as
`llm-wiki` on `PATH`. That is true for the release installer and `cargo install`
when their install directories are correctly configured. It is not true when a
user downloads a macOS, Linux, or future Windows binary and runs it directly
from `Downloads` or another ad hoc location.

In that manual-download case:

```bash
/Users/alice/Downloads/llm-wiki install
```

can install framework skills successfully, but the rendered skills currently
invoke:

```bash
llm-wiki init ...
```

Those skill calls fail if `llm-wiki` is not discoverable on `PATH`.

The same failure mode will exist on Windows when Windows support is added:

```powershell
.\Downloads\llm-wiki.exe install
```

can install skills, but later skill calls to `llm-wiki` fail if the executable
is not discoverable through the user's `PATH` or `PATHEXT` rules.

## Proposal

Change `llm-wiki install` so PATH availability is not required for installed
skills to work.

This is proposed future behavior. It should not be treated as validated runtime
behavior until the implementation lands, the strict test gates pass, and the
affected specs are promoted.

Install should always create and verify a managed runtime home, make the
currently running binary available there, install runtime skills that invoke the
managed binary by absolute path, and only then perform PATH guidance as a
convenience diagnostic.

Two path choices are resolved by this proposal:

1. The managed runtime home is `~/.llm_wiki` on Unix-like systems and
   `%LOCALAPPDATA%\llm_wiki` on Windows. The underscore form matches the
   framework repository and avoids mixing two spellings in user-visible state.
2. The new manifest at `~/.llm_wiki/manifest.json` replaces the D8 manifest
   path. The old manifest is migrated once and reported as legacy state, but it
   is not kept synchronized.

Managed locations:

```text
~/.llm_wiki/
~/.llm_wiki/bin/
~/.llm_wiki/bin/llm-wiki
~/.llm_wiki/manifest.json
```

Future Windows equivalent:

```text
%LOCALAPPDATA%\llm_wiki\
%LOCALAPPDATA%\llm_wiki\bin\
%LOCALAPPDATA%\llm_wiki\bin\llm-wiki.exe
%LOCALAPPDATA%\llm_wiki\manifest.json
```

The install operation should:

1. Resolve the current executable with `std::env::current_exe()`.
2. Create the managed runtime home.
3. Write an install transaction marker under the managed runtime home.
4. Copy the current executable to the managed binary path, unless the current
   executable already resolves to that path.
5. Verify the managed binary `sha256` hash matches the current executable
   `sha256` hash.
6. Create a scoped backup snapshot of the known framework skill target paths.
7. Install Claude and Codex skills.
8. Render skills so binary calls use the managed binary absolute path.
9. Write the manifest atomically under the managed runtime home and clear the
   transaction marker.
10. Verify manifest-owned skill files and the managed binary.
11. Check whether `llm-wiki` is discoverable on `PATH`.
12. If not discoverable, inform the user that installed skills still work and
    offer PATH help.

Example post-install message:

```text
Installed llm-wiki runtime:
  ~/.llm_wiki/bin/llm-wiki

Installed skills call that managed binary directly, so they work without PATH.

For terminal convenience, `llm-wiki` is not currently on PATH.
Add this to ~/.zshrc:
  export PATH="$HOME/.llm_wiki/bin:$PATH"
```

## Ordering

Install correctness must come before PATH convenience.

1. Resolve the running executable.
2. Write an install transaction marker.
3. Copy or verify the managed binary.
4. Back up known framework skill target paths.
5. Write and verify installed skill files.
6. Write and verify the final manifest.
7. Clear the transaction marker.
8. Report success.
9. Check PATH and guide the user if needed.

If PATH setup fails, is declined, or is ignored, the installed skills still work
because they invoke the managed binary path.

## Executable Resolution and Install Transactions

`std::env::current_exe()` is the primary way to locate the binary being run. If
it fails, or if the resolved path cannot be read as a file, install must abort
with a clear error before modifying managed state. The first implementation
should not guess from `argv[0]` or search `PATH` as a fallback, because that
can install a different binary than the one the user invoked.

If `current_exe()` resolves to the managed binary path, install must skip the
copy step and verify the binary in place. This makes repeated installs from
`~/.llm_wiki/bin/llm-wiki install` deterministic and avoids platform-specific
copy-over-self behavior.

Install must record an in-progress transaction before copying the managed
binary. The marker can be a small JSON file such as:

```text
~/.llm_wiki/install.partial.json
```

If install is interrupted before the final manifest is written, the next
install sees the marker and treats only the expected managed binary path as
part of the interrupted framework install. Recovery behavior:

1. If the managed binary exists and its `sha256` matches the current executable,
   continue and write the final manifest.
2. If the managed binary exists but its `sha256` differs, replace it only under
   the normal manifest-owned or `--force` collision rules.
3. If the marker references a different target path, treat it as stale state and
   require `--force` or interactive confirmation before overwriting anything.

The final manifest write should be atomic: write a temporary manifest file in
the managed runtime home, fsync where practical, then rename it into place.
Clearing the transaction marker happens only after the final manifest and
verification steps succeed.

## Managed Binary Policy

The installer copies rather than moves the current executable.

Moving the currently running binary is platform-sensitive and can surprise
users who expect the downloaded file to remain where they put it. Copying is
predictable and lets `doctor` verify or repair the managed copy.

Managed binary and file hashes use `sha256`.

If a managed binary already exists:

1. If its `sha256` matches the current executable `sha256`, leave it in place.
2. If it is an older manifest-owned `llm-wiki`, replace it.
3. If it differs and is not manifest-owned, refuse unless `--force` is passed
   or an interactive confirmation is accepted.

Preserve platform-specific executable names:

- Unix-like systems: `llm-wiki`
- Windows: `llm-wiki.exe`

## Scoped Skill Backup

Before replacing installed skills, create a timestamped backup snapshot under
the managed runtime home:

```text
~/.llm_wiki/backups/install-<UTC timestamp>/
~/.llm_wiki/backups/install-<UTC timestamp>/backup-manifest.json
```

The backup scope is limited to known framework skill target paths, not every
user skill:

```text
~/.claude/skills/knowledge-init/
~/.claude/skills/knowledge-query/
~/.claude/skills/knowledge-ingest/
~/.claude/skills/knowledge-research/
~/.claude/skills/knowledge-lint/
~/.codex/skills/knowledge-init/
~/.codex/skills/knowledge-query/
~/.codex/skills/knowledge-ingest/
~/.codex/skills/knowledge-research/
~/.codex/skills/knowledge-lint/
~/.codex/skills/knowledge/
```

Legacy `init-project` paths should also be backed up during the rename window:

```text
~/.claude/skills/init-project/
~/.codex/skills/init-project/
```

The backup manifest records:

1. original path
2. backup path
3. whether the original path existed
4. `sha256` file hashes for backed-up files
5. timestamp
6. binary version

The first implementation does not need an automated rollback command. The
backup snapshot must be structured enough for manual rollback and for a future
`llm-wiki rollback <backup-id>` command.

If no known framework skill targets exist yet, install still writes a backup
manifest recording the empty snapshot. That keeps install behavior uniform and
proves the backup step ran before file replacement.

## Installed Skill Set

`llm-wiki install` should install the full bundled framework skill set for each
supported runtime, not only the skill needed for the current command.

Current proposed skill set:

```text
knowledge-init
knowledge-query
knowledge-ingest
knowledge-research
knowledge-lint
knowledge
```

Claude receives the non-dispatcher skills that declare Claude runtime support.
Codex receives those skills plus the `knowledge` dispatcher and any required
runtime config files.

## Skill Rename: `knowledge-init`

Rename the agent-facing initialization skill from `init-project` to
`knowledge-init` for consistency with the rest of the framework command family:

```text
knowledge-init
knowledge-query
knowledge-ingest
knowledge-research
knowledge-lint
knowledge
```

Canonical asset target:

```text
assets/skills/knowledge-init/SKILL.md
```

Installed targets:

```text
~/.claude/skills/knowledge-init/SKILL.md
~/.codex/skills/knowledge-init/SKILL.md
```

The Codex dispatcher continues to support:

```text
$knowledge init
```

but routes to `knowledge-init`.

The old `init-project` installed paths are legacy targets. The rename is a
separable migration phase inside the same release: managed binary install must
remain rollbackable even if the rename step finds a conflict. During the first
rename implementation, `install` should back up old paths before writing
current skills.

Proposed rename policy:

1. Remove `init-project` when it is manifest-owned.
2. Back up `init-project` before removal.
3. Refuse to delete unknown user-authored `init-project` paths unless `--force`
   is provided.
4. Do not install a temporary `init-project` alias by default. Keeping both
   names active would weaken the clarity gained by the rename.
5. Report unresolved legacy `init-project` paths in `doctor`.
6. If the rename phase fails after managed binary installation succeeds, report
   the rename as the failing phase and leave the managed binary manifest state
   coherent.

## Manifest Migration

The current D8 manifest lives at:

```text
~/.local/share/llm-wiki/manifest.json
```

This proposal moves the manifest to:

```text
~/.llm_wiki/manifest.json
```

Migration behavior:

1. On install, read the new manifest path first.
2. If missing, read the old D8 manifest path.
3. If the old manifest exists, migrate entries into the new manifest schema and
   write `~/.llm_wiki/manifest.json`.
4. Do not delete the old manifest automatically in the first version; `doctor`
   should report it as legacy state after successful migration.
5. A later cleanup release may remove or archive the old manifest after a clear
   deprecation period.

The manifest should record the managed binary entry, including path, `sha256`
hash, version, and ownership status, so `doctor` and `uninstall` can reason
about it.

## Manifest Schema

The exact schema can evolve before implementation, but the first version should
make binary ownership, skill ownership, backup provenance, and migrated state
explicit. Sketch:

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
  ],
  "migration": {
    "from_manifest": "/Users/alice/.local/share/llm-wiki/manifest.json",
    "synchronized_with_legacy_manifest": false
  }
}
```

The backup manifest should use the same `hash_algorithm` field for backed-up
files. Empty backup snapshots still write a valid backup manifest with an empty
file list.

## PATH Guidance

After successful install, use `which` to detect whether `llm-wiki` is
discoverable on `PATH`.

Primary use:

```rust
which::which("llm-wiki")
```

On Windows, `which` handles extension lookup through `PATHEXT`, so
`which::which("llm-wiki")` can resolve `llm-wiki.exe`.

If `llm-wiki` is not on `PATH`, print guidance. Do not block install success.

Unix-like example:

```bash
export PATH="$HOME/.llm_wiki/bin:$PATH"
```

For Unix-like shells, detect the likely profile from `$SHELL` with the caveat
that the login shell may differ from the currently interactive shell. If `$SHELL`
is not clearly zsh, bash, or fish, print examples for all three instead of
guessing.

Windows PowerShell example:

```powershell
[Environment]::SetEnvironmentVariable(
  "Path",
  "$env:LOCALAPPDATA\llm_wiki\bin;$env:Path",
  "User"
)
```

Warn that a new terminal session may be required before `PATH` changes are
visible.

Optional interactive help may offer to create a symlink or copy in an existing
writable PATH directory, but that is convenience only. It must not replace the
managed binary as the runtime target for installed skills.

The first implementation should start with printed guidance rather than editing
shell profiles. A later proposal can add explicit profile-editing behavior if
the project accepts the added platform and shell risk.

## Implementation Notes

These are implementation notes to carry into the plan, not acceptance-contract
requirements. Use small crates for cross-platform mechanics rather than
inventing them.

### `which`

Use `which` to check whether `llm-wiki` is discoverable on `PATH`.

### `is-terminal`

Use `is-terminal` if the implementation needs reliable interactive vs
non-interactive detection before offering PATH help.

### `dialoguer` or Hand-Rolled Prompting

Use `dialoguer` only if the prompt grows beyond simple yes/no or numbered
selection behavior. The binary already has simple interactive prompting for
`init`, so hand-rolled prompting may be sufficient.

### `dirs-next`

Do not rely on `dirs-next::executable_dir()` as the main solution. Its docs
show a user executable directory on Linux, but macOS and Windows do not provide
one through that function. The managed runtime home should be framework-owned
and explicit.

## Windows Compatibility

Windows is out of scope for D8 release artifacts, but this proposal should not
paint the implementation into a Unix-only corner. The design should be
Windows-compatible from the start even if the first implementation only ships
on macOS and Linux.

Windows-specific requirements:

1. Keep executable naming platform-aware: copy to `llm-wiki.exe` on Windows.
2. Use `which` rather than manual PATH splitting so `PATHEXT` behavior is
   respected.
3. Use `%LOCALAPPDATA%\llm_wiki\bin` as the managed binary directory unless a
   later Windows-specific decision chooses a better location.
4. Do not edit the Windows user environment automatically. Printing a
   PowerShell command is acceptable; mutating PATH should require an explicit
   future flag and separate review.
5. Render managed absolute paths in a form that the target runtime can execute
   on Windows. This may require quoting paths with spaces and using the `.exe`
   suffix.
6. Add Windows tests when Windows enters the supported target matrix.

## Command Shape

Keep `llm-wiki install` as the main user-facing command.

Suggested commands:

```text
llm-wiki install
llm-wiki install --force
llm-wiki install --skip-path-guidance
llm-wiki path
```

Semantics:

- Default: copy/verify managed binary, install skills using managed binary
  path, then print PATH guidance if `llm-wiki` is not discoverable.
- `--force`: replace colliding manifest-owned files and the managed binary
  after backup or confirmation according to the existing collision policy.
- `--skip-path-guidance`: install managed runtime and skills, but do not print
  PATH convenience guidance.
- `llm-wiki path`: print PATH guidance for the managed bin directory without
  reinstalling skills. This is a separate command because it does not perform an
  install.

## Doctor and Uninstall

`llm-wiki doctor` should verify:

1. Managed binary exists.
2. Managed binary `sha256` hash matches the manifest entry.
3. Installed skills invoke the managed binary path.
4. Manifest lives at `~/.llm_wiki/manifest.json`.
5. Known skill paths match the current expected framework set.
6. Legacy `init-project` paths are absent, manifest-owned, or explicitly
   reported as user-authored conflicts.
7. Legacy D8 manifest state exists only when expected during migration.
8. `llm-wiki` PATH visibility is reported as convenience status, not an error.
9. Stale `install.partial.json` state is absent or recoverable.

`llm-wiki uninstall` should remove manifest-owned skill files and manifest
entries. Removing the managed binary should require an explicit flag such as:

```text
llm-wiki uninstall --include-binary
```

Default uninstall should avoid deleting the executable that may be running the
command.

## Non-Goals

- Do not silently edit `.zshrc`, `.bashrc`, fish config, launchd environment,
  Windows user environment, or any other shell/OS startup configuration.
- Do not make PATH availability a precondition for skill correctness.
- Do not replace the cargo-dist installer. The release installer remains the
  preferred fully packaged install path.
- Do not add self-update behavior.
- Do not claim Windows release support before Windows is added to the
  cargo-dist target matrix and runtime behavior is tested.

## Acceptance Criteria

1. `llm-wiki install` creates `~/.llm_wiki/bin/llm-wiki` on Unix-like systems
   and records it in `~/.llm_wiki/manifest.json`.
2. `llm-wiki install` creates a scoped backup snapshot before replacing known
   framework skill paths.
3. Installed skills invoke the managed binary absolute path, not a bare
   `llm-wiki` command.
4. An end-to-end test runs a manually downloaded binary from outside `PATH`,
   installs skills without shell profile edits, invokes an installed skill or
   skill-equivalent stub, and confirms it executes the managed binary path.
5. Install verifies the managed binary `sha256` hash after copying or
   verify-in-place.
6. Existing managed binary collisions follow the same refusal/force discipline
   as skill file collisions.
7. Existing D8 manifests under `~/.local/share/llm-wiki/manifest.json` are read
   and migrated once into `~/.llm_wiki/manifest.json` without data loss; the old
   manifest is not kept synchronized.
8. PATH guidance is printed after successful install when `llm-wiki` is not
   discoverable, and missing PATH is not treated as install failure.
9. `init-project` is renamed to `knowledge-init` in canonical assets,
   installed paths, docs, and the Codex dispatcher.
10. Legacy `init-project` installed paths are backed up and either removed when
    manifest-owned or refused as user-authored conflicts unless `--force` is
    provided.
11. `llm-wiki doctor` reports missing or drifted managed binary state.
12. `llm-wiki uninstall` leaves the managed binary in place unless an explicit
    include-binary flag is provided.
13. Platform path tests cover Unix executable names and Windows `.exe` naming,
    quoting, and PATHEXT lookup behavior at the unit-test level even before
    Windows release artifacts are shipped.
14. If `current_exe()` resolution fails, install aborts before modifying
    managed state and prints a clear diagnostic.
15. Running `~/.llm_wiki/bin/llm-wiki install` skips copy-over-self and verifies
    the managed binary in place.
16. Interrupted installs leave an `install.partial.json` marker that a later
    install can either recover from or reject under the documented collision
    rules.
17. The manifest and backup manifest record `hash_algorithm: "sha256"` beside
    every hash.
18. `llm-wiki path` prints PATH guidance without reinstalling skills.

## Open Questions

1. Should `uninstall --include-binary` also remove empty `~/.llm_wiki/`
   directories?
2. Should Windows eventually get an explicit `--update-user-path` flag, or
   should PATH mutation stay permanently outside the binary?
3. Should PATH guidance offer to symlink into an existing writable PATH
   directory, or only print instructions?
4. Should automated rollback ship with the backup snapshot feature, or can it
   follow in a later release?

## Revisit When

- Windows distribution moves from future compatibility to a supported release
  target.
- cargo-dist changes installer behavior or generated install paths.
- The framework adds self-update support.
- Runtime skills gain a stable way to invoke package-relative binaries without
  absolute paths.
