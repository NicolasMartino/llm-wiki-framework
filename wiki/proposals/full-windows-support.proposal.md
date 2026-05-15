# Full Windows Support

- Document Class: Proposal
- Status: Proposed
- Date: 2026-05-15
- Category: Distribution tooling, platform support, release engineering
- Scope: Promote Windows from compatibility design to a first-class supported
  platform for the `llm-wiki` binary, including release artifacts, managed
  runtime paths, cross-platform release E2E coverage, search parity, and
  install/doctor/uninstall behavior.
- Sources: conversational request 2026-05-15; wiki/roadmaps/framework-v1.roadmap.md;
  wiki/decisions/llm-wiki-binary-distribution.decision.md;
  wiki/plans/llm-wiki-binary.plan.md;
  wiki/decisions/binary-path-bootstrap.decision.md;
  wiki/plans/binary-path-bootstrap.plan.md;
  wiki/decisions/search-backend-selection.decision.md;
  wiki/plans/qmd-rs-search-backend.plan.md; proposal review feedback
  2026-05-15; cross-platform E2E and real-use simulation feedback
  2026-05-15; src/paths.rs; src/skill_render.rs; src/doctor.rs
- Related: wiki/specs/documentation-model.spec.md,
  wiki/specs/wiki-init-skill.spec.md,
  wiki/checklists/observability-contract.checklist.md,
  wiki/proposals/project-update-command.proposal.md

## Question

What has to be true before the project can honestly claim full Windows support,
instead of only saying the managed-runtime design is Windows-compatible?

## Proposal

Accept a dedicated post-V1 deliverable for release-grade Windows support.

The support target is: a Windows user can install `llm-wiki`, run
`llm-wiki install`, invoke installed framework skills through Claude/Codex, and
use init, registry, indexing, search, status, doctor, and uninstall without WSL,
without a Rust toolchain, and without hand-editing shell profiles.

The work should be promoted into the roadmap as the next platform deliverable
after the currently completed V1 and P1 search work. A tactical plan should
then implement it in narrow phases with Windows CI and a cross-platform release
E2E matrix as the proof gate.

## Current Baseline

The framework has Windows-compatible design intent, but the current code is not
Windows-ready.

Documented design intent:

1. D8.1 specifies the future Windows managed runtime root as a LocalAppData
   `llm_wiki` directory and the managed binary as `llm-wiki.exe`.
2. The install model renders installed skills to call a managed binary by
   absolute path rather than relying on `PATH`.
3. The D8.1 plan called out `.exe` naming, command quoting, and PATHEXT lookup
   as Windows concerns to cover.
4. Search is behind an internal adapter, so qmd-rs packaging issues can be
   isolated without changing the user-visible search contract.

Current implementation gap:

1. `Paths::from_env` still requires `HOME`, uses XDG-style cache/data fallbacks,
   and does not branch on Windows.
2. `managed_home` still resolves to `$HOME/.llm_wiki`; `cache_home` and
   `data_home` are still Unix-shaped. None route through LocalAppData or the
   Windows Known Folder APIs.
3. The current Windows-specific implementation surface is narrow:
   `managed_binary_name` adds the `.exe` suffix, `managed_binary_invocation`
   quotes Windows command paths, and `doctor` has a PATHEXT-aware path probe.
4. D8 release artifacts are documented only for macOS arm64, macOS x86_64,
   Linux x86_64, and Linux arm64.
5. D8.1 excluded Windows release artifacts, and the wiki does not yet record a
   Windows CI or release-gate proof run.

## Support Definition

Windows is first-class only when all of the following are true.

### Release Artifacts

`cargo-dist` or the release pipeline produces Windows artifacts for:

1. `x86_64-pc-windows-msvc` as the required mainstream Windows target.
2. `aarch64-pc-windows-msvc` if cargo-dist and CI support it without a bespoke
   maintenance burden.

If Windows ARM64 cannot ship in the same deliverable, the project must record
that as an explicit limitation before calling the x64 path supported. The
release notes must not imply ARM64 support unless an ARM64 artifact exists and
passes the same install smoke.

### Install And Runtime Paths

Windows install uses the D8.1 managed-home model. `%LOCALAPPDATA%` is the
typical resolved display value, not the source of truth for path resolution:
implementation must use the Windows Known Folder API directly or through a
maintained Rust wrapper such as `dirs` / `directories`. Raw environment-variable
lookup is not sufficient for redirected profiles, service accounts, or managed
enterprise setups.

```text
%LOCALAPPDATA%\llm_wiki\
%LOCALAPPDATA%\llm_wiki\bin\llm-wiki.exe
%LOCALAPPDATA%\llm_wiki\manifest.json
%LOCALAPPDATA%\llm_wiki\install.partial.json
%LOCALAPPDATA%\llm_wiki\backups\
```

`Paths::from_env` must grow a Windows branch. On Windows, `managed_home`,
`cache_home`, and `data_home` must come from platform-aware roots. `$HOME`,
`.llm_wiki`, and XDG fallback paths remain Unix-like behavior only.

Search indexes, model caches, and other rebuildable host-local state must use
platform-aware Windows locations under the framework's Windows data/cache
roots. The exact subdirectories can follow the existing Rust path helpers, but
they must be documented by `doctor` and covered by tests.

Project-local `.llm_wiki\` remains project-local metadata and should work under
normal Windows paths. Canonical wiki citations should remain repo-relative
markdown paths, independent of the host separator.

Registry canonicalization must have a concrete Windows rule. For local drive
paths, persistent project identity uses `std::fs::canonicalize`, strips the
extended-length `\\?\` prefix before storage/comparison, normalizes the drive
letter to uppercase, and persists separators as `/` in registry JSON. UNC and
network roots are outside the first Windows support deliverable unless a later
plan adds explicit proof gates for them.

### Skill Invocation

Installed Claude and Codex skills on Windows must invoke the managed binary by
an absolute, quoted `.exe` path. A user profile path with spaces must be a
release-gate fixture, for example:

```text
C:\Users\Test User\AppData\Local\llm_wiki\bin\llm-wiki.exe
```

No installed skill may rely on `PATH` for correctness. `PATH` remains terminal
convenience only, as on Unix-like systems.

### Command Parity

The Windows build must support the same user-visible V1/P1 command surface as
macOS and Linux:

1. `install`, `path`, `build`, `init`, `status`, `doctor`, `uninstall`
2. `register`, `forget`, `projects`
3. `index`, `index-all`, `search`, `search-all`
4. semantic/hybrid readiness paths when LLM search is enabled
5. JSON output contracts and `-v` / `--verbose` diagnostics

If a command cannot be supported on Windows, that is a release blocker unless a
new decision explicitly narrows the support claim.

The sibling `llm-wiki update` proposal is still proposed. If `update` is
accepted and implemented before P2 lands, Windows support must include update
smoke coverage. Update archive paths must use NTFS-safe timestamp strings
(no `:` in filenames). If P2 lands first, `update` remains outside the P2
support claim and must carry its own Windows proof gates before release.

### Cross-Platform Release E2E

P2 should introduce a release E2E harness that runs against every supported
artifact target, not only the new Windows target. Windows support should raise
the platform bar for the product as a whole.

Required supported-platform matrix:

1. macOS arm64 release artifact
2. macOS x86_64 release artifact
3. Linux x86_64 release artifact
4. Linux arm64 release artifact
5. Windows x86_64 release artifact
6. Windows ARM64 release artifact if that target is included in the support
   claim

The E2E must install and run the built artifact, not `cargo run`, and must use a
fresh redirected user/runtime home for each platform run. It should verify files
on disk after each command instead of trusting stdout alone.

Real-use simulation standard:

1. Treat `llm-wiki` as a black-box product. The harness may inspect files after
   commands run, but it must invoke documented CLI commands through the platform
   shell instead of calling internal Rust APIs, test-only hooks, or helper
   functions.
2. Exercise the same acquisition and install path a user would follow for that
   platform: unpack or run the release artifact, run `llm-wiki install`, then
   use the managed binary and installed skills produced by that install.
3. Use normal platform conventions: PowerShell on Windows, normal shell quoting
   on Unix-like systems, platform-native temp/profile directories, real path
   separators, and platform-specific environment behavior.
4. Start from empty user/runtime state and verify the resulting state is created
   by the product. Pre-seeding manifests, registry files, search stores, skill
   directories, or project files is allowed only when the scenario is explicitly
   testing migration from a documented prior release shape.
5. Run realistic user journeys, not isolated command fragments: fresh machine
   install, new project creation, project registration, indexing, searching,
   status/doctor inspection, optional project update, and cleanup.
6. Verify external contracts in the same way users and automation consume them:
   filesystem paths exist, manifests and hashes match, JSON parses and
   round-trips, stdout/stderr remain separated, and diagnostics name actionable
   recovery steps.
7. Prefer actual runtime invocation for installed Claude/Codex skills when a
   stable runtime API exists. If no stable API exists, the fallback must execute
   the command path rendered into the installed skill files and assert that this
   reaches the managed binary; it must not simply assert that the text exists.
8. Keep any shortcuts explicit. A scenario that uses a fixture, stub runtime, or
   redirected home must state which part of real use it simulates and which
   guarantee it cannot provide.

Minimum E2E command story:

1. `llm-wiki install`: verify managed binary exists, manifest exists, manifest
   hashes match files, runtime skill files exist for Claude and Codex, Codex
   `agents/openai.yaml` files exist where expected, and installed skill command
   strings point at the managed binary.
2. second `llm-wiki install`: verify idempotency by comparing manifest and file
   hashes or mtimes where stable.
3. `llm-wiki path`: verify platform-appropriate PATH guidance and no filesystem
   mutation.
4. `llm-wiki status` and `doctor`: verify they report clean install state and
   the expected managed/runtime paths for the current platform.
5. `llm-wiki build --out <tmp>`: verify rendered Claude and Codex skill trees
   and runtime config files are created under the requested output directory,
   without touching global install state.
6. `llm-wiki init`: verify `raw/`, `wiki/`, `AGENTS.md`, `CLAUDE.md`,
   `project_guidelines.md`, `.llm_wiki/init.toml`, selected pack folders, and
   generated `wiki/index.md` / `wiki/log.md` content.
7. `llm-wiki register` and `projects --format json`: verify registry file
   contents, stable project id, parseable JSON, and project paths resolving to
   the created project.
8. `llm-wiki index`: verify qmd-rs store and semantic sidecar state expected
   for the chosen search mode.
9. `llm-wiki search --format json`: verify parseable results cite canonical
   repo-relative wiki paths and that path strings round-trip on the platform.
10. `llm-wiki search-all --format json`: verify registered-project results,
    project labels, and readiness/skipped-project metadata.
11. `llm-wiki update` if implemented before P2: verify project-scoped
    mutation, including project-root writes plus target-project host-local
    cache/registry writes, archive manifest contents, generated artifact
    rewrites, preserved compiled wiki content, and search staleness or reindex
    behavior.
12. `llm-wiki uninstall`: verify manifest-owned skills and optional managed
    binary cleanup behavior, and verify unrelated files under runtime dirs are
    preserved.

Windows-container E2E may be added as a supplemental lane on a Windows host, but
it does not replace host/VM runner proof. The release claim requires host or VM
coverage because profile paths, Known Folder resolution, Defender behavior,
runtime skill discovery, and platform filesystem behavior are part of the
supported user experience.

### Search Backend Parity

qmd-rs is the selected backend for normal builds. Full Windows support therefore
requires qmd-rs-backed indexing and search to pass on Windows.

If qmd-rs or one of its native dependencies exposes a Windows packaging blocker,
the plan should use the existing adapter boundary to decide between:

1. fixing qmd-rs packaging for Windows,
2. temporarily gating semantic-only functionality while preserving lexical
   qmd-rs FTS behavior, or
3. promoting the direct SQLite FTS fallback for Windows through a separate
   decision.

The project should not silently ship a Windows binary where search commands are
missing or materially weaker than documented default behavior.

## Implementation Surface

The tactical plan should inspect and adapt these areas first:

1. release metadata and cargo-dist target matrix
2. `src/paths.rs`, specifically `Paths::from_env`, `managed_home`,
   `cache_home`, `data_home`, registry paths, and index/model path helpers
3. install, manifest, backup, partial-marker, and uninstall code
4. skill projection or install rendering where managed binary paths are injected
5. `doctor` PATH lookup and drift reporting, using first-match
   PATHEXT-resolving semantics such as the `which` crate rather than ad hoc
   `where.exe` parsing
6. project registry canonical-root handling for drive letters and separators
7. search index path construction and qmd-rs store creation on Windows
8. release E2E harness and fixtures that assert filesystem state after every
   command on every supported artifact target
9. integration tests that currently assume Unix path strings or executable bits
10. README, release notes, and install documentation

## Test And Proof Gates

The Windows support plan should not be considered complete until these gates
pass in CI or in a recorded release-gate run:

1. `cargo test --workspace --all-features` or the project-equivalent full test
   command on `windows-latest`.
2. `cargo clippy --workspace --all-targets --all-features -- -D warnings` on
   Windows, unless the project records an explicit Windows-only exception.
3. `cargo fmt --check` and `git diff --check`.
4. `cargo dist plan` and a Windows artifact build for the accepted Windows
   targets.
5. Cross-platform release E2E runs from built artifacts on every supported
   platform target, including the four existing macOS/Linux targets and each
   new Windows target.
6. The release E2E follows the Real-Use Simulation Standard: documented
   commands through platform shells, no internal API calls, no preseeded state
   except documented migration fixtures, and explicit notes for any runtime
   stubs.
7. The release E2E verifies command-by-command filesystem effects for install,
   build, init, register/projects, index, search, search-all, status, doctor,
   and uninstall. If `llm-wiki update` has landed before P2, it is included in
   the same E2E story.
8. Install smoke on Windows from a release artifact, not only from `cargo run`,
   with default Windows Defender behavior left enabled.
9. Path-resolution tests prove the Windows Known Folder path is used for the
   managed runtime and that the command does not require `HOME`, XDG variables,
   or a manually populated `LOCALAPPDATA` environment variable.
10. Redirected-profile install tests use `%USERPROFILE%`, the resolved
   LocalAppData location, and a user path containing spaces.
11. `llm-wiki install` writes Windows skill files with quoted absolute `.exe`
   invocations and a manifest under the Windows managed home.
12. Claude and Codex installed-skill smoke tests, or skill-equivalent command
   stubs when the runtime has no stable Windows discovery API, both execute the
   managed `.exe` path. If Codex skill discovery has a Windows limitation, it
   must be recorded before claiming Codex support.
13. A second `llm-wiki install` is idempotent under the same Windows runner,
    including hash verification after Defender real-time scanning.
14. `llm-wiki doctor` reports managed binary state, PATH convenience state,
    registry state, and search/index state without Unix-specific path wording.
15. `llm-wiki uninstall` removes only manifest-owned files and handles
    `--include-binary`.
16. `llm-wiki init` creates a project in a Windows temp directory with spaces
    in the path and in a local nested path longer than 260 characters.
17. Registering the same local project root through drive-letter case variants
    and, where applicable, with/without an extended-length `\\?\` prefix
    produces one registry entry.
18. `llm-wiki register`, `index`, `search`, and `search-all` work on that
    project.
19. JSON-capable commands (`projects`, `search`, `search-all`, and eval
    subcommands) round-trip through `serde_json` with Windows path fields
    escaped correctly and resolvable after parsing.
20. Snapshot and golden tests are line-ending-stable on Windows. The preferred
    defense is a committed `.gitattributes` rule forcing LF for skill assets,
    templates, snapshot fixtures, and markdown used in golden tests.
21. If `llm-wiki update` has landed before P2, update smoke passes on Windows
    and archives use NTFS-safe timestamp/path names.
22. A clean Windows smoke run and the cross-platform release E2E result are
    recorded in a wiki eval or checklist before
    the roadmap item is marked completed.

## Documentation Changes

Acceptance requires documentation updates in the same deliverable:

1. README install instructions include Windows release-artifact and Cargo paths.
2. `llm-wiki path` prints PowerShell-friendly PATH guidance on Windows.
3. `doctor` output names Windows managed paths and recovery commands.
4. The release notes state the exact supported Windows targets.
5. Release documentation states that every supported platform artifact passes
   the release E2E harness, summarizes what the harness verifies, and states
   any explicit simulation fallback such as a runtime stub.
6. `wiki/specs/documentation-model.spec.md` replaces the existing
   forward-looking Windows managed-home sentence with validated target-specific
   support language only after the proof gates pass.

## Out Of Scope

The first Windows support deliverable should not include:

1. WSL-specific behavior.
2. Automatic PowerShell profile mutation.
3. `winget`, Chocolatey, Scoop, or MSI packaging beyond the release artifacts
   produced by the existing release pipeline.
4. Self-update.
5. Windows-specific runtime targets beyond Claude and Codex.
6. Authenticode or EV code signing unless the existing release policy already
   requires equivalent signing for other platforms.
7. Automatic Windows Defender or antivirus exclusions.
8. UNC and network path support for the first Windows deliverable.

Unsigned Windows artifacts may trigger SmartScreen warnings. That should be
called out in release notes if signing remains out of scope.

## Risks

1. qmd-rs native dependencies may expose Windows build or runtime issues.
2. Path quoting can regress installed skill invocation when `%USERPROFILE%` or
   `%LOCALAPPDATA%` contains spaces.
3. Drive-letter canonicalization can create duplicate registry entries for the
   same project root if paths are compared as raw strings.
4. CRLF normalization can destabilize golden snapshots if tests assume Unix
   newlines.
5. Windows executable semantics differ from Unix executable-bit checks.
6. Defender or third-party antivirus can delay, scan-lock, or quarantine the
   managed binary between copy and hash verification, especially while artifacts
   remain unsigned.
7. Long local paths are proof-gated; UNC and network paths are explicitly
   deferred unless a later plan adds coverage.
8. Codex skill discovery on Windows may expose runtime-specific behavior that
   the current repo cannot fully validate without a runtime smoke.

## Proposed Promotion Path

If accepted:

1. Add a post-V1 roadmap item, tentatively `P2 - Full Windows Support`.
2. Write a tactical plan that owns the implementation phases and proof gates.
3. Apply the Observability Contract checklist to every changed command path.
4. Add or update a repeatable release E2E checklist/eval so future releases can
   prove every supported platform artifact still satisfies the command and file
   verification contract.
5. Replace the current forward-looking Windows sentence in
   `wiki/specs/documentation-model.spec.md` with validated target-specific
   support language only after Windows CI and release smoke pass.
