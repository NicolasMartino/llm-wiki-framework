# Binary PATH Bootstrap

- Document Class: Proposal
- Status: Proposed
- Date: 2026-05-06
- Category: Distribution tooling, install UX
- Scope: Improve `llm-wiki install` behavior across macOS, Linux, and future Windows support when the running binary is not discoverable as `llm-wiki` on `PATH`.
- Sources: user discussion 2026-05-06, wiki/decisions/llm-wiki-binary-distribution.decision.md, wiki/plans/llm-wiki-binary.plan.md, wiki/plans/llm-wiki-product-layout-addendum.plan.md, https://docs.rs/which/latest/which/, https://docs.rs/dirs-next/latest/dirs_next/fn.executable_dir.html, https://docs.rs/dialoguer/latest/dialoguer/, https://docs.rs/is-terminal/latest/is_terminal/, https://doc.rust-lang.org/stable/cargo/commands/cargo-install.html
- Related: wiki/specs/init-project-skill.spec.md, wiki/specs/documentation-model.spec.md, wiki/proposals/llm-wiki-binary.proposal.md

## Question

How should `llm-wiki install` behave when a user runs a manually downloaded
binary from a location that is not on `PATH`, but the installed skills later
need to invoke `llm-wiki`?

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

can install framework skills successfully, but the rendered skills invoke:

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

Add PATH bootstrap handling to `llm-wiki install`.

At install time, the binary should:

1. Resolve the current executable with `std::env::current_exe()`.
2. Check whether `llm-wiki` is discoverable on `PATH`.
3. If discoverable, keep the existing behavior and render skills that invoke
   `llm-wiki`.
4. If not discoverable and the session is interactive, explain the problem and
   offer explicit choices.
5. If not discoverable and the session is non-interactive, fail unless the user
   passes an explicit fallback flag.
6. Preserve platform-specific executable names when copying the binary:
   `llm-wiki` on Unix-like systems and `llm-wiki.exe` on Windows.

The preferred interactive prompt:

```text
llm-wiki is not on PATH.

Installed skills need to call the llm-wiki binary.

Options:
1. Copy this binary into a user bin directory and use `llm-wiki`
2. Use the current absolute path in installed skills
3. Abort so I can install llm-wiki myself
```

If the user chooses the absolute-path fallback, the binary prints a warning:

```text
Installed skills will call /absolute/path/to/llm-wiki.
If you move or delete this binary, rerun `llm-wiki install` or install
llm-wiki on PATH.
```

## Crate Choices

Use small crates for the cross-platform mechanics rather than inventing them.

### `which`

Use `which` to detect whether `llm-wiki` is already discoverable on `PATH`.
The crate is a Rust equivalent of Unix `which` and supports Linux, macOS, and
Windows. It handles executable checks and platform-specific path lookup.

Primary use:

```rust
which::which("llm-wiki")
```

On Windows, `which` handles extension lookup through `PATHEXT`, so
`which::which("llm-wiki")` can resolve `llm-wiki.exe`.

### `is-terminal`

Use `is-terminal` if the implementation needs a reliable interactive vs
non-interactive check before prompting.

Primary use:

```rust
use is_terminal::IsTerminal;

let interactive = std::io::stdin().is_terminal() && std::io::stdout().is_terminal();
```

### `dialoguer` or Hand-Rolled Prompting

Use `dialoguer` only if the prompt grows beyond a simple yes/no flow. The
current binary already has simple interactive prompting for `init`, so a
hand-rolled prompt may be sufficient. If the install choice becomes a numbered
selection, `dialoguer` is a reasonable dependency.

### `dirs-next`

Do not rely on `dirs-next::executable_dir()` as the main solution. Its docs
show a user executable directory on Linux, but macOS and Windows do not provide
one through that function. `dirs-next` can still be useful for `home_dir()` if
needed, but the project already has hand-rolled `HOME` resolution for D8.

## User Bin Directory Policy

When `llm-wiki` is missing from `PATH`, the binary should prefer safe,
predictable destinations:

1. Existing writable directory already present on `PATH`.
2. On Unix-like systems, `~/.local/bin` as the suggested fallback.
3. `~/.cargo/bin` only when it already exists and is on `PATH`, or when the
   user explicitly chooses it.
4. On Windows, an existing writable PATH directory first; otherwise a
   framework-owned user directory such as
   `%LOCALAPPDATA%\Programs\llm-wiki\bin` as the suggested fallback.

The binary should not silently edit shell startup files. Instead, when the
chosen destination is not on the current `PATH`, print exact shell
instructions.

Unix-like example:

```bash
export PATH="$HOME/.local/bin:$PATH"
```

and name the likely shell profile when obvious, such as `~/.zshrc` for zsh or
`~/.bashrc` for bash.

Windows PowerShell example:

```powershell
[Environment]::SetEnvironmentVariable(
  "Path",
  "$env:LOCALAPPDATA\Programs\llm-wiki\bin;$env:Path",
  "User"
)
```

The Windows instruction should warn that a new terminal session may be required
before `PATH` changes are visible.

## Windows Compatibility

Windows is out of scope for D8 release artifacts, but this proposal should not
paint the implementation into a Unix-only corner. The bootstrap design should
be Windows-compatible from the start even if the first implementation only
ships on macOS and Linux.

Windows-specific requirements:

1. Keep executable naming platform-aware: copy to `llm-wiki.exe` on Windows.
2. Use `which` rather than manual PATH splitting so `PATHEXT` behavior is
   respected.
3. Avoid Unix-only assumptions about `HOME`; use the existing path resolver or
   platform-specific environment variables for user-local destinations.
4. Do not edit the Windows user environment automatically. Printing a
   PowerShell command is acceptable; mutating PATH should require an explicit
   future flag and separate review.
5. Render absolute paths in a form that the target runtime can execute on
   Windows. This may require quoting paths with spaces and using the `.exe`
   suffix.
6. Add Windows tests when Windows enters the supported target matrix.

## Command and Flag Shape

Keep `llm-wiki install` as the main user-facing command. Add explicit flags for
scripted use:

```text
llm-wiki install
llm-wiki install --binary-invocation path
llm-wiki install --binary-invocation absolute
llm-wiki install --install-binary-to <dir>
```

Semantics:

- `--binary-invocation path`: require `llm-wiki` on `PATH`; fail if missing.
- `--binary-invocation absolute`: render skills with the current executable's
  absolute path; warn about relocation risk.
- `--install-binary-to <dir>`: copy the current executable to
  `<dir>/llm-wiki` or `<dir>/llm-wiki.exe` depending on platform, then render
  skills with `llm-wiki` if that directory is on `PATH`; otherwise print PATH
  instructions and fail unless combined with an explicit absolute-path
  fallback.

The default interactive behavior may offer these choices. The default
non-interactive behavior should fail on ambiguity.

## Non-Goals

- Do not silently edit `.zshrc`, `.bashrc`, fish config, launchd environment,
  Windows user environment, or any other shell/OS startup configuration.
- Do not replace the cargo-dist installer. The release installer remains the
  preferred fully packaged install path.
- Do not add self-update behavior.
- Do not change the global skill install locations or manifest schema unless
  implementation proves the binary invocation choice must be recorded there.
- Do not claim Windows release support before Windows is added to the
  cargo-dist target matrix and runtime behavior is tested.

## Acceptance Criteria

1. `llm-wiki install` succeeds unchanged when `which::which("llm-wiki")`
   resolves to the running binary or another valid `llm-wiki` executable.
2. Running a manually downloaded binary not on `PATH` produces a clear
   interactive choice instead of silently installing broken skills.
3. Non-interactive install fails when `llm-wiki` is not on `PATH` unless the
   user provides an explicit binary-invocation/install-location flag.
4. Absolute-path fallback renders skills that call the resolved current
   executable path and prints a relocation warning.
5. Copy-to-bin flow refuses to overwrite an unrelated existing executable
   without an explicit force flag or confirmation.
6. The install report says which binary invocation was written into generated
   skills.
7. `llm-wiki doctor` reports when installed skills use an absolute binary path
   that no longer exists.
8. Tests cover PATH-present, PATH-missing interactive decision handling,
   non-interactive failure, absolute-path rendering, copy-to-bin collision, and
   doctor detection of stale absolute paths.
9. Platform path tests cover Unix executable names and Windows `.exe` naming,
   quoting, and PATHEXT lookup behavior at the unit-test level even before
   Windows release artifacts are shipped.

## Open Questions

1. Should the binary invocation choice be stored in the existing install
   manifest, or is it enough to derive it from installed skill file contents?
2. Should the copy-to-bin flow default to `~/.local/bin` on macOS even though
   macOS does not standardize that through `dirs-next::executable_dir()`?
3. Should `llm-wiki install` include a `--force-binary-overwrite` flag, or
   should executable overwrite always require an interactive confirmation?
4. Should the absolute-path fallback be allowed for global installs, or should
   it be limited to development builds?
5. Should Windows eventually get an explicit `--update-user-path` flag, or
   should PATH mutation stay permanently outside the binary?
6. Which Windows user-local bin directory should be preferred when no writable
   PATH directory exists?

## Revisit When

- Windows distribution moves from future compatibility to a supported release
  target.
- cargo-dist changes installer behavior or generated install paths.
- The framework adds self-update support.
- Runtime skills gain a stable way to invoke package-relative binaries without
  relying on shell `PATH`.
