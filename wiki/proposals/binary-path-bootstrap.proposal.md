# Managed Binary Install and PATH Guidance

- Document Class: Proposal
- Status: Proposed
- Date: 2026-05-06
- Category: Distribution tooling, install UX
- Scope: Make `llm-wiki install` create a stable managed binary location for runtime skills, then guide users when `llm-wiki` is not discoverable on `PATH`.
- Sources: user discussion 2026-05-06, wiki/decisions/llm-wiki-binary-distribution.decision.md, wiki/plans/llm-wiki-binary.plan.md, wiki/plans/llm-wiki-product-layout-addendum.plan.md, https://docs.rs/which/latest/which/, https://docs.rs/dirs-next/latest/dirs_next/fn.executable_dir.html, https://docs.rs/dialoguer/latest/dialoguer/, https://docs.rs/is-terminal/latest/is_terminal/, https://doc.rust-lang.org/stable/cargo/commands/cargo-install.html
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

Install should always create and verify a managed runtime home, copy the
currently running binary into that home, install runtime skills that invoke the
managed binary by absolute path, and only then perform PATH guidance as a
convenience diagnostic.

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
3. Copy the current executable to the managed binary path.
4. Verify the managed binary hash matches the current executable hash.
5. Install Claude and Codex skills.
6. Render skills so binary calls use the managed binary absolute path.
7. Write the manifest under the managed runtime home.
8. Verify manifest-owned skill files and the managed binary.
9. Check whether `llm-wiki` is discoverable on `PATH`.
10. If not discoverable, inform the user that installed skills still work and
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

1. Copy and verify managed binary.
2. Write and verify installed skill files.
3. Write and verify manifest.
4. Report success.
5. Check PATH and guide the user if needed.

If PATH setup fails, is declined, or is ignored, the installed skills still work
because they invoke the managed binary path.

## Managed Binary Policy

The installer copies rather than moves the current executable.

Moving the currently running binary is platform-sensitive and can surprise
users who expect the downloaded file to remain where they put it. Copying is
predictable and lets `doctor` verify or repair the managed copy.

If a managed binary already exists:

1. If it matches the current executable hash, leave it in place.
2. If it is an older manifest-owned `llm-wiki`, replace it.
3. If it differs and is not manifest-owned, refuse unless `--force` is passed
   or an interactive confirmation is accepted.

Preserve platform-specific executable names:

- Unix-like systems: `llm-wiki`
- Windows: `llm-wiki.exe`

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

The manifest should record the managed binary entry, including path, hash,
version, and ownership status, so `doctor` and `uninstall` can reason about it.

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

Name the likely shell profile when obvious, such as `~/.zshrc` for zsh or
`~/.bashrc` for bash.

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

## Crate Choices

Use small crates for cross-platform mechanics rather than inventing them.

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

## Command and Flag Shape

Keep `llm-wiki install` as the main user-facing command.

Suggested flags:

```text
llm-wiki install
llm-wiki install --force
llm-wiki install --skip-path-guidance
llm-wiki install --print-path-guidance
```

Semantics:

- Default: copy/verify managed binary, install skills using managed binary
  path, then print PATH guidance if `llm-wiki` is not discoverable.
- `--force`: replace colliding manifest-owned files and the managed binary
  after backup or confirmation according to the existing collision policy.
- `--skip-path-guidance`: install managed runtime and skills, but do not print
  PATH convenience guidance.
- `--print-path-guidance`: print PATH guidance for the managed bin directory
  without reinstalling skills.

## Doctor and Uninstall

`llm-wiki doctor` should verify:

1. Managed binary exists.
2. Managed binary hash matches the manifest entry.
3. Installed skills invoke the managed binary path.
4. Manifest lives at `~/.llm_wiki/manifest.json`.
5. Legacy D8 manifest state exists only when expected during migration.
6. `llm-wiki` PATH visibility is reported as convenience status, not an error.

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
2. Installed skills invoke the managed binary absolute path, not a bare
   `llm-wiki` command.
3. Running a manually downloaded binary from outside `PATH` installs working
   skills without requiring shell profile edits.
4. Install verifies the managed binary hash after copying.
5. Existing managed binary collisions follow the same refusal/force discipline
   as skill file collisions.
6. Existing D8 manifests under `~/.local/share/llm-wiki/manifest.json` are read
   and migrated without data loss.
7. PATH guidance is printed after successful install when `llm-wiki` is not
   discoverable, and missing PATH is not treated as install failure.
8. `llm-wiki doctor` reports missing or drifted managed binary state.
9. `llm-wiki uninstall` leaves the managed binary in place unless an explicit
   include-binary flag is provided.
10. Platform path tests cover Unix executable names and Windows `.exe` naming,
    quoting, and PATHEXT lookup behavior at the unit-test level even before
    Windows release artifacts are shipped.

## Open Questions

1. Should `~/.llm_wiki/manifest.json` replace the old manifest immediately, or
   should the migration keep both manifests synchronized for one release?
2. Should the managed runtime home use `~/.llm_wiki` or `~/.llm-wiki`? The
   current proposal uses underscore to match the user's requested convention,
   but the binary/package name uses hyphen.
3. Should `uninstall --include-binary` also remove empty `~/.llm_wiki/`
   directories?
4. Should Windows eventually get an explicit `--update-user-path` flag, or
   should PATH mutation stay permanently outside the binary?
5. Should PATH guidance offer to symlink into an existing writable PATH
   directory, or only print instructions?

## Revisit When

- Windows distribution moves from future compatibility to a supported release
  target.
- cargo-dist changes installer behavior or generated install paths.
- The framework adds self-update support.
- Runtime skills gain a stable way to invoke package-relative binaries without
  absolute paths.
