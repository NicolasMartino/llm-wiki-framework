# Cross-Platform Release E2E And Windows Support

- Document Class: Proposal
- Status: Proposed
- Date: 2026-05-15
- Category: Distribution tooling, platform support, release engineering
- Scope: Establish black-box release E2E guarantees for every supported
  platform artifact and promote Windows from compatibility design to a
  first-class supported platform for the `llm-wiki` binary, including release
  artifacts, managed runtime paths, search parity, and install/doctor/uninstall
  behavior.
- Sources: conversational request 2026-05-15; wiki/roadmaps/framework-v1.roadmap.md;
  wiki/roadmaps/cross-platform-release-e2e.roadmap.md;
  wiki/decisions/llm-wiki-binary-distribution.decision.md;
  wiki/plans/llm-wiki-binary.plan.md;
  wiki/decisions/binary-path-bootstrap.decision.md;
  wiki/plans/binary-path-bootstrap.plan.md;
  wiki/decisions/search-backend-selection.decision.md;
  wiki/plans/qmd-rs-search-backend.plan.md; proposal review feedback
  2026-05-15; cross-platform E2E, real-use simulation, and re-review feedback
  2026-05-15; command-surface review feedback 2026-05-15; Cargo.toml;
  src/cli.rs; src/paths.rs; src/skill_render.rs; src/doctor.rs;
  wiki/plans/cross-platform-release-e2e-harness.plan.md;
  raw/research/2026-05-26-cqrs-release-e2e-source-capture/manifest.md;
  raw/research/2026-05-26-cqrs-release-e2e-source-capture/research-summary.md
- Related: wiki/specs/documentation-model.spec.md,
  wiki/specs/wiki-init-skill.spec.md,
  wiki/checklists/observability-contract.checklist.md,
  wiki/proposals/project-update-command.proposal.md,
  wiki/roadmaps/cross-platform-release-e2e.roadmap.md,
  wiki/plans/cross-platform-release-e2e-harness.plan.md

## Source Capture

This proposal currently cites conversational input and review feedback because
the direction was developed in chat. The CQRS Pulumi/test-runner precedent has
now been captured under
`raw/research/2026-05-26-cqrs-release-e2e-source-capture/`. The associated
roadmap and tactical plan now exist because release E2E execution is already
underway. Before accepting this proposal as a durable support baseline or
promoting its outcomes into validated specs/decisions, capture the remaining
conversation and review basis as raw sources under `raw/` or replace those
references with durable project-owned source paths.

## Question

What has to be true before the project can honestly claim full Windows support
and provide strong release-artifact guarantees for every supported platform?

## Proposal

Accept a dedicated post-V1 deliverable for cross-platform release E2E and
release-grade Windows support.

The support target is: a Windows user can install `llm-wiki`, run
`llm-wiki install`, invoke installed framework skills through Claude/Codex, and
use init, registry, indexing, search, status, doctor, and uninstall without WSL,
without a Rust toolchain, and without hand-editing shell profiles.

The work is coordinated by
`wiki/roadmaps/cross-platform-release-e2e.roadmap.md` as the next
release/platform workstream after the completed V1 and P1/P2 search work. The
tactical harness plan implements it in narrow phases with Docker/Linux proof,
native host proof, Windows CI/host proof, and a cross-platform release E2E
matrix as the proof gate.

## Associated Roadmap And Tactical Plan

`wiki/roadmaps/cross-platform-release-e2e.roadmap.md` is the roadmap associated
with this proposal. It owns deliverable ordering, dependencies, proof gates,
promotion targets, and the distinction between Linux Docker simulation, native
host proof, Windows host proof, and GGUF CPU release proof.

`wiki/plans/cross-platform-release-e2e-harness.plan.md` is the tactical plan
associated with this proposal's release-E2E harness. It owns the runner,
profile, Pulumi/Linux-lane, native macOS/Windows lane, report, no-download
readiness, and real-model GGUF CPU proof design.

The roadmap and plan do not by themselves accept or complete full Windows
support. This proposal remains the parent support definition for Windows
managed paths, PowerShell skill invocation, qmd-rs/search parity,
Defender/long-path gates, and release target claims.

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

### Windows Baseline

The first Windows support claim covers:

1. Windows 10 22H2 or newer.
2. Windows 11.
3. Windows Server 2022 or newer for CI/server validation.
4. Windows PowerShell 5.1 for documented commands and installer acquisition.
5. PowerShell 7 only when release docs name `pwsh` or a PowerShell 7-specific
   installer path.

Hosted Windows CI is acceptable for routine release gates only when it can prove
the required profile, Known Folder, path-with-spaces, long-path, and Defender
behaviors. If hosted CI cannot prove one of those behaviors, the tactical plan
must use a self-hosted runner or VM. A public "Windows desktop" claim requires a
recorded Windows 10/11 client smoke before release; server-only CI is not enough
for that wording.

### Release Artifacts

`cargo-dist` or the release pipeline produces Windows artifacts for:

1. `x86_64-pc-windows-msvc` as the required mainstream Windows target.
2. `aarch64-pc-windows-msvc` if cargo-dist and CI support it without a bespoke
   maintenance burden.

If Windows ARM64 cannot ship in the same deliverable, the project must record
that as an explicit limitation before calling the x64 path supported. The
release notes must not imply ARM64 support unless an ARM64 artifact exists and
passes the same install smoke.

Current release configuration only declares shell installers and the four
non-Windows targets. P2 must update release metadata before implementation can
claim Windows.

Mandatory Windows acquisition path:

1. Ship a Windows ZIP artifact per supported Windows target containing
   `llm-wiki.exe`.
2. Publish SHA-256 checksums for each Windows artifact and make checksum
   verification part of the documented install path and E2E.
3. Document the Windows install path as PowerShell-based ZIP acquisition and
   expansion unless a PowerShell installer is added.
4. If a PowerShell installer is added, E2E must run the installer path exactly
   as documented. ZIP acquisition may remain as the fallback manual path.
5. If artifacts remain unsigned, release notes must say so and call out the
   expected SmartScreen posture. Code signing is still out of scope unless a
   later decision changes release policy.

Unix-like acquisition remains the documented shell installer or archive
extraction path for each supported target. The release E2E must use the same
documented path for each platform rather than a custom unpack helper that users
do not run.

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

Windows release artifacts must be long-path-aware. The build should embed an
application manifest with `longPathAware` enabled. The long-local-path proof
gate requires a runner with Windows long paths enabled; if the runner cannot
enable that host setting, the gate must be recorded as skipped with a known
limitation rather than mistaken for a product pass.

Search indexes, model caches, and other rebuildable host-local state must use
platform-aware Windows locations under the framework's Windows data/cache
roots. The exact subdirectories can follow the existing Rust path helpers, but
they must be documented by `doctor` and covered by tests.

Project-local `.llm_wiki\` remains project-local metadata and should work under
normal Windows paths. Canonical wiki citations should remain repo-relative
markdown paths, independent of the host separator.

Path helper mapping:

| Helper(s) | Windows location | Migration behavior |
| --- | --- | --- |
| `home` | Known Folder `Profile` | Resolve through Windows APIs; no `HOME` requirement |
| `claude_skill`, `codex_skill`, `codex_config` | Runtime skill directories under the user profile, e.g. `%USERPROFILE%\.claude\skills\...` and `%USERPROFILE%\.codex\skills\...` unless runtime docs specify a different Windows root | Existing user-authored runtime files remain protected by install collision rules |
| `managed_home` | `%LOCALAPPDATA%\llm_wiki` using Known Folder `LocalAppData` | First supported Windows release creates this root; no pre-public Windows migration promise |
| `managed_bin_dir`, `managed_binary` | `%LOCALAPPDATA%\llm_wiki\bin\llm-wiki.exe` | Install copies/verifies the running release artifact here |
| `manifest`, `partial_install`, backup paths | `%LOCALAPPDATA%\llm_wiki\manifest.json`, `%LOCALAPPDATA%\llm_wiki\install.partial.json`, `%LOCALAPPDATA%\llm_wiki\backups\...` | Interrupted installs recover from this state; local dogfood state can be repaired with documented install/doctor/uninstall paths |
| `search_config`, `search_thresholds`, `external_dependencies`, `accepted_licenses` | `%LOCALAPPDATA%\llm_wiki\*.toml` | Global managed config follows the runtime home; existing Unix config remains unchanged |
| `managed_model_root`, `model_artifacts` | `%LOCALAPPDATA%\llm_wiki\models\...` | Managed model artifacts are install-owned; missing/corrupt artifacts are repaired by install-owned flows |
| `managed_index_root`, `project_index_dir`, `qmd_rs_store_path`, `semantic_index_metadata`, `semantic_vector_index` | `%LOCALAPPDATA%\llm_wiki\indexes\<project-id>\...` | Rebuildable indexes may be removed/rebuilt; stale stores are not canonical project knowledge |
| `cache_home`, `legacy_index_root`, `legacy_project_index_dir`, `legacy_qmd_rs_store_path`, `model_cache` | `%LOCALAPPDATA%\llm_wiki\cache\...` | Used only for cache/legacy lookup or migration; all contents are rebuildable |
| `data_home`, `project_registry` | `%LOCALAPPDATA%\llm_wiki\data\projects.json` | Registry writes are atomic; canonicalized roots prevent duplicate same-project entries |
| project-local `.llm_wiki/*` | `<project>\.llm_wiki\...` | Project-local metadata remains inside the project and is not part of global runtime cleanup |

Registry canonicalization must have a concrete Windows rule. For local drive
paths, persistent project identity uses `std::fs::canonicalize`, strips the
extended-length `\\?\` prefix before storage/comparison, normalizes the drive
letter to uppercase, and persists separators as `/` in registry JSON. UNC and
network roots are outside the first Windows support deliverable unless a later
plan adds explicit proof gates for them.

### Skill Invocation

Installed Claude and Codex skills on Windows must invoke the managed binary by
an absolute `.exe` path using syntax that works in the target shell/runtime. A
user profile path with spaces must be a release-gate fixture, for example:

```text
C:\Users\Test User\AppData\Local\llm_wiki\bin\llm-wiki.exe
```

PowerShell execution is a load-bearing case. A quoted executable path by itself
is not enough; the rendered command must be executable through PowerShell, for
example `& "C:\Users\Test User\AppData\Local\llm_wiki\bin\llm-wiki.exe" ...`
when the command is interpreted by PowerShell. The E2E must execute the full
rendered command string through the target Windows shell or actual runtime, not
only spawn the `.exe` path directly.

No installed skill may rely on `PATH` for correctness. `PATH` remains terminal
convenience only, as on Unix-like systems.

### Command Parity

The Windows build must support the same user-visible V1/P1 command surface as
macOS and Linux:

1. `install`, `path`, `build`, `init`, `status`, `doctor`, `uninstall`
2. `eval run`, `eval calibrate`
3. `register`, `forget`, `projects`
4. `index`, `index-all`, `search`, `search-all`
5. semantic/hybrid readiness paths when LLM search is enabled
6. JSON output contracts and `-v` / `--verbose` diagnostics

If a command cannot be supported on Windows, that is a release blocker unless a
new decision explicitly narrows the support claim.

Command coverage matrix:

| Command surface | Coverage | Required proof |
| --- | --- | --- |
| global `--help`, `--version`, `-v/--verbose` | Smoke | Help/version run without mutation; representative verbose runs prove diagnostics stay on stderr and do not alter exit semantics |
| `build --target both/claude/codex --out` | Full | Rendered skill trees and Codex runtime config files appear only under `--out` |
| `install`, repeat install, `--force`, `--skip-path-guidance` | Full | Managed binary, manifest, backups/collisions, idempotency, and no-path-guidance behavior are verified |
| `install --configure-search`, `--disable-llm-search` | Smoke | Search profile state changes are verified without silently downloading or deleting artifacts outside install-owned flows |
| `init --non-interactive --name --description --blueprint --pack --initial-sources --no-register` | Full | Project files, copied sources, manifest, generated wiki files, pack folders, and no-register behavior are verified |
| hidden legacy `init --type`, `--scale` | Exempt from release E2E | Hidden compatibility flags remain covered by focused compatibility tests if retained |
| `eval run --eval-page --project --project-root --candidate-profile --embedding-model --query-expansion-model --reranker-model --candidate-name --output-dir --limit --format text/json --rerank --time-budget-warn-ms` | Smoke | Fixture eval page runs from the release artifact, emits text and JSON/report output, exercises candidate/profile plumbing, and writes any requested output directory |
| `eval calibrate --run-report`, flattened `eval run` flags, `--select-candidate`, `--record`, `--apply`, `--apply-profile`, `--export-raw-data`, `--raw-data-dir` | Smoke | Fixture run report calibrates through the release artifact; threshold/profile writes, recorded eval output, and raw-data export effects are verified against redirected state |
| `register [path] --name --id`, `register --update` | Full | Registry JSON has stable ids, same-root update behavior, canonical roots, and no duplicate same-project entries |
| `forget`, `forget --delete-cache` | Full | Registry entry removal and project cache cleanup are verified while project files remain untouched |
| `projects --format text/json` | Full | Text is human-readable; JSON parses and path fields resolve |
| `index`, `index --project`, `index --force` | Full | qmd-rs store, freshness metadata, and force rebuild behavior are verified |
| `index-all --force` | Full | Two registered projects are indexed; failure of one project is reported without corrupting the other |
| `search <query> --project --mode auto/lexical --format text/json --class --status --limit` | Full | Lexical/auto retrieval returns canonical wiki citations, filters work, project selection works, and JSON round-trips |
| `search <query> --project --mode semantic/hybrid --allow-lexical-fallback --rerank` | Full for readiness, full retrieval when the managed model profile is installed | Missing/disabled/stale/unconfigured readiness states fail closed with guidance; fallback behavior is explicit; successful semantic/hybrid retrieval uses a real managed profile or a documented accepted fixture profile, not mocks |
| `search-all <query> --mode auto/lexical/semantic/hybrid --include --exclude --class --status --limit --format text/json --allow-lexical-fallback --rerank` | Full for readiness and rank/fusion contracts | Multi-project results include project labels, readiness/skipped metadata, filters, limits, and JSON round-trip behavior |
| `path` | Full | Prints platform-appropriate PATH guidance and does not mutate filesystem state |
| `status` | Full | Reports clean, missing, and drifted install states from real files |
| `doctor` | Full | Reports install, PATH convenience, registry, project, search, model, and Windows runnability state; Windows runnability invokes the managed `.exe` |
| `uninstall`, `--include-binary`, `--search-artifacts`, `--force` | Full | Removes only manifest-owned global state or explicit search artifacts; unrelated files and project files are preserved |
| `update` and `defaults` if accepted before P2 | Full | Project-scoped and machine-default scopes are verified according to the accepted proposal |

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

Runners may be hosted or self-hosted. The gate is artifact behavior on the
target OS/architecture, not a commitment to a specific CI provider.

The E2E must install and run the built artifact, not `cargo run`, and must use a
fresh redirected user/runtime home for each platform run. It should verify files
on disk after each command instead of trusting stdout alone.

Real-use simulation standard:

1. Treat `llm-wiki` as a black-box product. The harness may inspect files after
   commands run, but it must invoke documented CLI commands through the platform
   shell instead of calling internal Rust APIs, test-only hooks, or helper
   functions.
2. Exercise the same documented acquisition and install path a user would
   follow for that platform, such as the documented shell installer or archive
   extraction on Unix-like systems and the documented PowerShell installer or
   ZIP expansion on Windows. Then run `llm-wiki install` and use the managed
   binary and installed skills produced by that install.
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
   the full command string rendered into the installed skill files through the
   target platform shell and assert that this reaches the managed binary; it
   must not simply assert that the text exists or spawn the binary by a
   separately constructed path.
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
   the expected managed/runtime paths for the current platform, and verify the
   managed binary is runnable by invoking it, for example with
   `llm-wiki --version`.
5. `llm-wiki build --out <tmp>`: verify rendered Claude and Codex skill trees
   and runtime config files are created under the requested output directory,
   without touching global install state.
6. `llm-wiki init`: verify `raw/`, `wiki/`, `AGENTS.md`, `CLAUDE.md`,
   `project_guidelines.md`, `.llm_wiki/init.toml`, selected pack folders, and
   generated `wiki/index.md` / `wiki/log.md` content.
7. `llm-wiki register` and `projects --format json`: verify registry file
   contents, stable project id, parseable JSON, and project paths resolving to
   the created project.
8. `llm-wiki forget --delete-cache`: verify registry removal and cache cleanup
   for a temporary registered project without deleting project files.
9. `llm-wiki index`: verify qmd-rs store and semantic sidecar state expected
   for the chosen search mode.
10. `llm-wiki index-all --force`: verify two registered projects are traversed,
    indexed, and reported without cross-project corruption.
11. `llm-wiki search --format json`: verify parseable results cite canonical
    repo-relative wiki paths and that path strings round-trip on the platform.
12. `llm-wiki search-all --format json`: verify registered-project results,
    project labels, and readiness/skipped-project metadata.
13. `llm-wiki eval run` and `llm-wiki eval calibrate`: verify the release
    artifact can run the eval CLI surface, emit JSON/report artifacts, apply or
    record calibration output under redirected state, and export raw eval data
    when requested.
14. `llm-wiki update` if implemented before P2: verify project-scoped
    mutation, including project-root writes plus target-project host-local
    cache/registry writes, archive manifest contents, generated artifact
    rewrites, preserved compiled wiki content, and search staleness or reindex
    behavior.
15. `llm-wiki uninstall`: verify manifest-owned skills and optional managed
    binary cleanup behavior, and verify unrelated files under runtime dirs are
    preserved.

Windows-container E2E may be added as a supplemental lane on a Windows host, but
it does not replace host/VM runner proof. The release claim requires host or VM
coverage because profile paths, Known Folder resolution, Defender behavior,
runtime skill discovery, and platform filesystem behavior are part of the
supported user experience.

### CQRS-Informed Harness Architecture

The tactical release E2E work should follow the architecture recorded in
`wiki/plans/cross-platform-release-e2e-harness.plan.md`. The key lesson from
the CQRS/Pulumi repository is lifecycle discipline, not "Docker proves every
platform."

Adopt these patterns:

1. A Rust runner owns E2E categories, command execution, report paths, JUnit
   output, and lifecycle flags such as `--skip-infra`, `--keep-infra`,
   `--output-dir`, `--stdout`, and `--verbose`.
2. `just` exposes small public commands while private recipes own stack
   startup, health checks, and teardown.
3. Profile files and required-key manifests describe lane-specific inputs
   instead of hiding environment assumptions in scripts.
4. Pulumi owns isolated Docker infrastructure for Linux/container lanes, with
   stack-specific names, platform selection, mounts, and cleanup.
5. Native macOS and Windows lanes use the same runner/report contract without
   Pulumi, because host filesystem, shell, security, and runtime-discovery
   behavior are exactly what those lanes must prove.
6. Target-aware evidence is accepted. A native runner, an emulated Docker lane,
   a no-download readiness lane, and a real-model GGUF lane are different proof
   types and must be labeled as such in reports.

For this proposal, the practical correction is:

1. Docker can prove Linux artifact behavior when the container OS/architecture
   matches the support target.
2. Docker can supplement Windows testing only when it runs on a Windows host
   and is labeled as container coverage.
3. Docker cannot replace Windows host/VM proof for Known Folder resolution,
   PowerShell execution, Defender interaction, path-with-spaces behavior,
   installed skill discovery, or long-path runnability.

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

If option 3 is chosen, the narrowing decision must land before the Windows
release, not after. That decision is what satisfies the Command Parity blocker
rule for a deliberately different Windows backend.

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
   command on every supported artifact target, implemented through
   `wiki/plans/cross-platform-release-e2e-harness.plan.md`
9. Windows application manifest and long-path-aware build configuration
10. integration tests that currently assume Unix path strings or executable bits
11. README, release notes, and install documentation

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
   new Windows target; follows the Real-Use Simulation Standard; and verifies
   the Command Coverage Matrix with command-by-command filesystem effects. If
   `llm-wiki update` or `llm-wiki defaults` has landed before P2, it is included
   in the same E2E story.
6. Install smoke on Windows from a release artifact, not only from `cargo run`,
   with default Windows Defender behavior left enabled.
7. Path-resolution tests prove the Windows Known Folder path is used for the
   managed runtime and that the command does not require `HOME`, XDG variables,
   or a manually populated `LOCALAPPDATA` environment variable.
8. Redirected-profile install tests use `%USERPROFILE%`, the resolved
   LocalAppData location, and a user path containing spaces.
9. `llm-wiki install` writes Windows skill files with shell-correct absolute
   `.exe` invocations, including the PowerShell call operator where required,
   and a manifest under the Windows managed home.
10. Claude and Codex installed-skill smoke tests, or skill-equivalent command
   stubs when the runtime has no stable Windows discovery API, both execute the
   full rendered command through the target runtime or shell and prove it
   reaches the managed `.exe` path. If Codex skill discovery has a Windows
   limitation, it must be recorded before claiming Codex support.
11. A second `llm-wiki install` is idempotent under the same Windows runner,
    including hash verification after Defender real-time scanning.
12. `llm-wiki doctor` reports managed binary state, PATH convenience state,
    registry state, and search/index state without Unix-specific path wording;
    its Windows runnability check invokes the managed `.exe` rather than only
    statting the file.
13. `llm-wiki uninstall` removes only manifest-owned files and handles
    `--include-binary`.
14. `llm-wiki init` creates a project in a Windows temp directory with spaces
    in the path and in a local nested path longer than 260 characters when the
    runner has long paths enabled. If long paths cannot be enabled on the
    runner, the skipped gate is recorded as a known limitation.
15. Registering the same local project root through drive-letter case variants
    and, where applicable, with/without an extended-length `\\?\` prefix
    produces one registry entry.
16. `llm-wiki register`, `forget`, `index`, `index-all`, `search`,
    `search-all`, `eval run`, and `eval calibrate` work from the release
    artifact against Windows project state.
17. JSON-capable commands (`projects`, `search`, `search-all`, and eval
    subcommands) round-trip through `serde_json` with Windows path fields
    escaped correctly and resolvable after parsing.
18. Snapshot and golden tests are line-ending-stable on Windows. The preferred
    defense is a committed `.gitattributes` rule forcing LF for skill assets,
    templates, snapshot fixtures, and markdown used in golden tests.
19. If `llm-wiki update` has landed before P2, update smoke passes on Windows
    and archives use NTFS-safe timestamp/path names.
20. A clean Windows smoke run and the cross-platform release E2E result are
    recorded in a wiki eval or checklist before
    the roadmap item is marked completed.

## Documentation Changes

Acceptance requires documentation updates in the same deliverable:

1. README install instructions include Windows release-artifact and Cargo paths.
2. `llm-wiki path` prints PowerShell-friendly PATH guidance on Windows.
3. `doctor` output names Windows managed paths and recovery commands.
4. The release notes state the exact supported Windows targets, supported
   Windows versions, required shell baseline, and whether validation came from
   hosted CI, self-hosted CI, client VM smoke, or a combination.
5. Release documentation states that every supported platform artifact passes
   the release E2E harness, summarizes what the harness verifies, names the
   documented acquisition path exercised per platform, and states any explicit
   simulation fallback such as a runtime stub.
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

## Promotion Status

Roadmap and tactical-plan bookkeeping now exists:
`wiki/roadmaps/cross-platform-release-e2e.roadmap.md` coordinates deliverables,
and `wiki/plans/cross-platform-release-e2e-harness.plan.md` owns the current
harness implementation.

Remaining promotion tasks:

1. Apply the Observability Contract checklist to every changed command path.
2. Add or update a repeatable release E2E checklist/eval so future releases can
   prove every supported platform artifact still satisfies the command and file
   verification contract.
3. Replace the current forward-looking Windows sentence in
   `wiki/specs/documentation-model.spec.md` with validated target-specific
   support language only after Windows CI and release smoke pass.
