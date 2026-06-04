# Plan: Cross-Platform Release E2E Harness

- Document Class: Plan
- Status: Active
- Date: 2026-05-25
- Category: Release engineering, E2E, platform support
- Scope: Build a release-artifact E2E harness that proves supported
  `llm-wiki` binaries behave like the documented product on their target
  platforms, while using Docker/Pulumi only where it gives honest coverage.
- Sources: user request 2026-05-25; wiki/proposals/full-windows-support.proposal.md;
  wiki/plans/gguf-runtime-portability.plan.md;
  wiki/plans/gguf-runtime-smoke-probes.plan.md;
  raw/research/2026-05-26-cqrs-release-e2e-source-capture/manifest.md;
  raw/research/2026-05-26-cqrs-release-e2e-source-capture/research-summary.md;
  justfile; tools/release-e2e/Cargo.toml; tools/release-e2e/src/main.rs;
  infra/release-e2e/Pulumi.yaml; infra/release-e2e/index.ts;
  infra/release-e2e/linux-builder.Dockerfile; vendor/qmd-0.3.2/Cargo.toml
- Related: wiki/proposals/full-windows-support.proposal.md,
  wiki/plans/gguf-runtime-portability.plan.md,
  wiki/plans/gguf-runtime-smoke-probes.plan.md,
  wiki/checklists/observability-contract.checklist.md
- Parent Proposal: wiki/proposals/full-windows-support.proposal.md

## Deliverable

The project has a repeatable release E2E harness that runs built `llm-wiki`
artifacts, not `cargo run`, through realistic user journeys on every supported
target. The harness verifies command effects on disk, JSON contracts,
stdout/stderr separation, installed skill invocations, search readiness, and
cleanup behavior from isolated user/runtime homes.

The harness has two distinct responsibilities:

1. Prove product behavior on host operating systems and architectures.
2. Use containerized infrastructure where containers honestly represent the
   thing being tested.

Docker is therefore a Linux/container proof lane and a useful lifecycle model.
It is not a replacement for macOS or Windows host/VM proof. A Windows support
claim still requires a Windows host, VM, or CI runner because Known Folder
resolution, PowerShell invocation, Defender behavior, `.exe` semantics, path
with spaces, and runtime skill discovery are part of the supported experience.

## Source Capture Note

The CQRS local-repository material that informed this plan is captured under
`raw/research/2026-05-26-cqrs-release-e2e-source-capture/`. Before this plan is
accepted as a release-process baseline, capture the remaining conversational
review basis under `raw/` or replace it with another durable project-owned
source path.

## CQRS Lessons To Reuse

The CQRS repository is a useful model in five areas.

1. A single Rust test runner owns lifecycle, categories, flags, subprocess
   execution, and JUnit/report output. In CQRS this is
   `code/tools/test-runner`, with `--skip-infra`, `--keep-infra`,
   `--fail-fast`, `--headed`, `--output-dir`, `--stdout`, and `--verbose`.
2. Infrastructure lifecycle is explicit and repeatable. `just test e2e` starts
   the `e2e` stack, waits for health checks, runs tests, and tears the stack
   down through an RAII-style guard.
3. Stack profiles separate dev and E2E behavior. CQRS uses 7xxx ports for dev
   and 9xxx ports for E2E, with config overrides layered on top.
4. Runtime profile files and required-key manifests prevent accidental
   environment drift. CQRS loads `config/runtime/<env>/<mode>/<service>.env`
   and validates `config/runtime/<service>.required`.
5. Target parity uses target-aware proof, not false sameness. The ADR keeps
   browser, Android, iOS, and desktop baselines target-specific while sharing a
   scenario contract.

For `llm-wiki`, the translation is not "run every platform in Docker." The
translation is: build one harness with explicit lifecycle, target-aware lanes,
profiles, reports, and cleanup; then attach the correct execution substrate to
each lane.

## Harness Architecture

### Runner

Add a small Rust runner, tentatively `tools/release-e2e`, with command
categories:

| Category | Purpose |
| --- | --- |
| `smoke` | Fast release-artifact checks that do not require GGUF downloads |
| `search` | qmd-rs lexical/auto search, indexing, registry, and JSON checks |
| `gguf` | Real managed-model semantic/hybrid CPU checks |
| `windows` | Windows-specific path, PowerShell, `.exe`, and Known Folder checks |
| `all` | Full lane for the current platform |

The runner should expose:

```text
llm-wiki-release-e2e smoke
llm-wiki-release-e2e search
llm-wiki-release-e2e gguf --manual-models
llm-wiki-release-e2e all --artifact <path> --checksum <path>
llm-wiki-release-e2e all --skip-infra
llm-wiki-release-e2e all --keep-infra
llm-wiki-release-e2e all --output-dir target/release-e2e
```

The exact binary name can change during implementation, but the responsibilities
should stay stable:

1. unpack or locate a release artifact
2. verify the published checksum
3. create a fresh isolated user home/runtime home and temp project root
4. invoke documented CLI commands through the target platform shell
5. inspect files and JSON after each command
6. emit a machine-readable report plus JUnit-compatible output
7. clean up unless `--keep-infra` or an equivalent debugging flag is passed

### Just Recipes

Add top-level recipes patterned after CQRS:

```text
just release-e2e
just release-e2e smoke
just release-e2e search
just release-e2e-gguf
just release-e2e-skip-infra
just _release-e2e-linux-up
just _release-e2e-linux-down
```

The private `_release-e2e-*` recipes are lifecycle hooks, not the public
contract. The public contract is the runner plus the small `just` entry points.

### Profiles

Add profile files instead of scattering environment assumptions through shell
scripts:

```text
e2e/release/profiles/local.env
e2e/release/profiles/macos-arm64.env
e2e/release/profiles/linux-amd64.env
e2e/release/profiles/linux-arm64.env
e2e/release/profiles/windows-amd64.env
e2e/release/profiles/gguf.required
```

The profile loader should validate required keys before starting a lane. The
profiles should separate:

1. artifact input locations
2. target triple and expected archive suffix
3. shell invocation mode
4. temp root policy
5. model-cache policy
6. Docker/Pulumi settings for Linux container lanes

Do not use profile files to hide product state. Product state must still be
created by `llm-wiki` commands unless the scenario explicitly tests migration
from a documented prior release shape.

### Pulumi

Pulumi is appropriate for a Linux release-E2E container lane. It can own:

1. an isolated Docker network and container names
2. a runner container per Linux target
3. bind mounts for release artifacts and reports
4. optional read-only model cache mounts for GGUF lanes
5. platform selection such as `linux/amd64` and `linux/arm64`
6. teardown of containers, networks, and temporary volumes

Pulumi should not be on the critical path for native macOS or Windows proof.
Those lanes run on their host/VM/CI runners and use the same Rust runner/report
contract without a Docker stack.

The CQRS caution about Docker Desktop deadlocking under parallel BuildKit image
builds should be copied into the implementation notes. If the Linux lane builds
runner images, stack startup should serialize builds or avoid BuildKit
parallelism until proven stable.

### Reports

Each lane writes:

```text
target/release-e2e/<lane>/report.json
target/release-e2e/<lane>/junit.xml
target/release-e2e/<lane>/stdout/
target/release-e2e/<lane>/stderr/
target/release-e2e/<lane>/state-manifest.json
```

The state manifest records the artifact path, checksum, target triple, OS,
architecture, temp roots, command list, expected filesystem effects, and actual
filesystem effects. This makes release evidence durable enough to summarize in
a wiki eval or checklist.

## Execution Plan

### Stage 0 - Plan Alignment And Source Capture

1. Keep this plan separate from the Windows proposal. The proposal defines what
   support means; this plan defines how release E2E proves it.
2. Update the Windows proposal to reference this plan as the tactical harness.
3. Update the GGUF portability plan so Linux/Windows CPU proof is promoted
   through this harness instead of ad hoc manual commands.
4. Capture the CQRS source material under `raw/` before marking this plan
   accepted.

Gate:

- The wiki distinguishes Docker/Linux proof from Windows/macOS host proof.
- Existing GGUF and Windows pages link to this plan instead of duplicating
  harness details.

## Implementation Progress 2026-05-26

Stage 1 has started. The repository now has a dedicated
`llm-wiki-release-e2e` tool crate at `tools/release-e2e/`, invoked through
`cargo run --manifest-path tools/release-e2e/Cargo.toml` so normal release
archives continue to ship only the product binary. The first implemented lane
is `smoke`, which accepts a supplied `--artifact`, optional `--checksum`,
`--target-triple`, `--output-dir`, `--skip-infra`, `--keep-infra`, `--verbose`,
and `--stdout`.

The smoke lane remains black-box: it invokes the supplied `llm-wiki` artifact
through the platform shell, sets an isolated temporary `HOME`, redirects XDG
cache/data roots under that home, removes LLM Wiki test-hook environment
variables, runs `llm-wiki --version`, and writes report artifacts under
`target/release-e2e/smoke/`:

```text
report.json
junit.xml
state-manifest.json
stdout/version.out
stderr/version.err
```

The top-level `justfile` now exposes:

```text
just release-e2e smoke
just release-e2e-skip-infra smoke
```

The local Stage 1 gate passed with `target/debug/llm-wiki` as the artifact:

```text
cargo test --manifest-path tools/release-e2e/Cargo.toml
cargo check --manifest-path tools/release-e2e/Cargo.toml
just release-e2e smoke
just release-plan
```

This completes the runner skeleton and artifact `--version` smoke. It does not
yet implement Linux, Windows, or real-model GGUF lanes. The release-plan gate
confirms the helper tool stays out of dist archives; each archive still lists
only `[bin] llm-wiki`.

Stage 2 now has a first local-host implementation in the `search` lane. Against
`target/debug/llm-wiki`, `just release-e2e search` runs 19 black-box commands
from an isolated HOME/XDG environment and a scratch path containing a space:

```text
version
install --non-interactive --disable-llm-search --skip-path-guidance
managed-version
install-second
path
status
doctor
build --out <tmp>
init --no-register --non-interactive
register
projects --format json
index --project release-e2e-probe
search --mode lexical --format json
search-all --mode lexical --format json
search --mode semantic --format json
search --mode hybrid --format json
forget --delete-cache
projects --format json
uninstall --include-binary
```

The lane records stdout/stderr for every command, writes JSON/JUnit/state
reports under `target/release-e2e/search/`, and currently records 60 passing
assertions. It verifies managed binary install, disabled no-model search
configuration, skill files, path/status/doctor output, build output, init
files, registry JSON, qmd-rs lexical indexing/search, lexical search-all,
semantic/hybrid fail-closed readiness with `llm_search_disabled`, cache
deletion, empty registry after forget, and uninstall cleanup.

This satisfies the local built-artifact no-model gate. It still does not
acquire or unpack release archives; Stage 3+ must run the same product story
against packaged release artifacts on Linux/macOS/Windows runners.

Stage 3 now has its first implementation slice. The runner exposes a `linux`
lane that reuses the Stage 2 product-story assertions through Docker instead of
copying a separate shell script. It mounts an unpacked Linux `llm-wiki` ELF
artifact at `/artifact/llm-wiki`, maps the isolated HOME and scratch run
directory into the container, executes each product command with `/bin/sh`, and
keeps all JSON/JUnit/state/stdout/stderr reports in the host
`target/release-e2e/linux/` tree. The lane records execution metadata in each
report:

```text
execution.kind = docker
execution.docker_image
execution.docker_platform
execution.docker_network
execution.architecture_native
execution.proof_kind
```

The proof kind distinguishes native Linux containers from architecture-native
virtualized containers and emulated packaging smoke. The lane refuses non-ELF
host artifacts up front, so running it with the macOS `target/debug/llm-wiki`
does not create a false Linux result. New just recipes expose the Docker lane:

```text
just release-e2e-linux-build
just release-e2e-linux-build-and-test
just release-e2e-linux <linux-elf-artifact>
just release-e2e-linux-skip-infra <linux-elf-artifact> network=<existing-network-or-none>
just release-e2e-linux-infra-up
just release-e2e-linux-infra-down
```

A minimal Pulumi stack now lives under `infra/release-e2e/`. It follows the
CQRS-derived pattern by creating a stack-specific internal Docker network and
exporting the image/platform/mount profile used by the runner. The runner can
also create and tear down an internal Docker network directly when Pulumi is
not in use.

The Linux arm64 Docker proof now has a real Linux ELF artifact path without
macOS cross-compilation. `infra/release-e2e/linux-builder.Dockerfile` creates a
small Rust/Debian builder image with the Linux native dependencies needed by
`llama-cpp-sys` (`libclang`, CMake, compiler toolchain, and pkg-config).
`just release-e2e-linux-build` builds the binary inside that container as the
host UID and writes it under
`target/release-e2e-linux-aarch64/release/llm-wiki`; `just
release-e2e-linux-build-and-test` then runs the same artifact through the
Docker Linux lane.

The first minimal-Debian runtime attempt exposed a real portability dependency:
the default `llama-cpp-2` feature set enabled OpenMP and produced a Linux
binary requiring `libgomp.so.1`. The vendored qmd dependency now disables
`llama-cpp-2` default features so the Linux artifact does not depend on
`libgomp`; the rebuilt artifact runs in `debian:bookworm-slim` with only the
standard C/C++ runtime libraries present.

On 2026-05-26, `just release-e2e-linux
target/release-e2e-linux-aarch64/release/llm-wiki` passed the full no-model
product story in Docker: 19 commands, 60 assertions, no failed assertions, and
`execution.proof_kind = virtualized_linux_container` with
`execution.architecture_native = true`. This closes the local Linux arm64
container proof for an unpacked ELF artifact. It still does not close release
archive acquisition/unpacking, Linux amd64, native Linux host proof, macOS host
proof, Windows host/VM proof, or real GGUF release proof.

### Stage 1 - Runner Skeleton

1. Create the Rust runner and a focused test helper library for:
   - command execution
   - target-shell quoting
   - artifact unpacking
   - checksum verification
   - isolated home/temp directory creation
   - JSON parsing
   - filesystem effect assertions
   - report writing
2. Implement `--artifact`, `--checksum`, `--target-triple`, `--output-dir`,
   `--skip-infra`, `--keep-infra`, `--verbose`, and `--stdout`.
3. Add unit tests for path handling, shell command construction, JSON report
   structure, and cleanup behavior.
4. Add public `just release-e2e smoke` for the current host.

Gate:

- The runner can execute `llm-wiki --version` from an unpacked artifact and
  write a report without touching the developer's real `~/.llm_wiki`.

### Stage 2 - Black-Box Product Story

Implement the minimum no-model release story:

1. acquire/unpack the release artifact exactly as documented
2. verify SHA-256 checksum
3. run `llm-wiki install`
4. verify managed binary, manifest, backups, skills, and rendered command
   strings
5. run a second install and verify idempotency
6. run `path`, `status`, and `doctor`
7. run `build --out <tmp>`
8. run `init` in a temp project
9. run `register`, `projects --format json`, `index`, `search --mode lexical`,
   `search-all --mode lexical`, `forget --delete-cache`, and `uninstall`
10. verify files after every command

Semantic/hybrid search in this stage should verify fail-closed readiness states
when models are not installed. It should not use test-only hooks or forged
artifact records.

Gate:

- macOS arm64 local host can run the no-model story from a built artifact.
- The report records every command, exit status, stdout/stderr path, and
  filesystem assertion.

### Stage 3 - Linux Container Lane

1. Add a Pulumi stack for Linux release E2E, with stack-specific container
   names and an isolated network.
2. Run the same runner inside Linux containers against mounted release
   artifacts.
3. Support native Linux arm64 on arm64 hosts and Linux amd64 either on native
   amd64 runners or through an explicitly labeled emulation lane.
4. Treat emulated Docker results as packaging smoke unless the report records a
   native runner.

Gate:

- Linux amd64 and Linux arm64 lanes can run the no-model story.
- Reports state whether the lane was native or emulated.

### Stage 4 - Native macOS And Windows Lanes

1. Run the same runner on macOS arm64 and macOS x86_64 host/VM/CI runners.
2. Run the same runner on Windows x86_64 host/VM/CI runners.
3. Use PowerShell for Windows command strings, including the call operator for
   quoted executable paths.
4. Verify Known Folder managed paths, path with spaces, `.exe` invocation,
   CRLF-stable snapshots, JSON path escaping, and uninstall behavior.
5. Add Windows ARM64 only if the release support claim includes it.

Gate:

- Each supported artifact target has a host/VM/CI report.
- Windows support is not marked complete based on a Docker result.

### Stage 5 - GGUF CPU Semantic/Hybrid Lane

Split model behavior into two lanes:

1. Routine CI lane: no downloads, no hidden model state. It verifies disabled,
   missing, stale, and runtime-failed readiness states from the release
   artifact.
2. Real-model lane: environment-dependent, allowed to be nightly/manual, and
   allowed to use a prewarmed byte cache only if `llm-wiki install` still owns
   license consent, SHA-256 verification, artifact recording, runtime probes,
   and enabled `search.toml` promotion.

The real-model lane should reuse the GGUF CPU smoke scenario at the product
level:

1. force CPU with `LLM_WIKI_GGUF_RUNTIME=cpu`
2. install/configure the balanced profile through the release artifact
3. index a small temp wiki fixture
4. verify semantic, hybrid, and `search-all` return the project-update page
5. record `runtime_backend_requested=cpu`, `runtime_backend_used=cpu`, and
   `runtime_backend_fallback=false`

Gate:

- The routine release E2E remains fast and no-download.
- Real GGUF proof is available before claiming semantic/hybrid support for a
  platform.

### Stage 6 - CI Integration And Release Evidence

1. Wire the runner into the release pipeline after artifact build.
2. Upload per-lane reports as CI artifacts.
3. Record the release E2E result in a wiki eval or checklist before marking a
   platform support item complete.
4. Keep a manual override path for self-hosted runners when hosted CI cannot
   prove a required platform behavior.

Gate:

- A release cannot claim a target unless the corresponding release E2E report
  exists and passes.
- Skipped gates are named as limitations, not silently counted as passes.

## Verification Commands

Initial implementation should end with:

```text
cargo fmt --check
cargo check
cargo test --workspace
cargo dist plan
just release-e2e smoke
just release-e2e search
```

Environment-dependent gates:

```text
just release-e2e-gguf
just release-e2e-skip-infra search
```

The exact commands can change if the runner name changes. The invariant is that
release E2E invokes built artifacts through documented command surfaces and
writes durable reports.

## Open Questions

1. Whether to place the runner under `tools/release-e2e`, `tests/release-e2e`,
   or a cargo workspace crate if the repository layout changes.
2. Whether Linux arm64 proof should require a native arm64 runner or accept an
   explicitly labeled emulated smoke for early implementation.
3. Whether real-model GGUF release proof belongs in every release, nightly, or
   pre-release candidate gates.
4. Whether Windows ARM64 is included in the first Windows support claim.

## Risks

1. A harness that calls internal Rust APIs would give false confidence. Keep it
   black-box.
2. Docker lanes can hide host filesystem and shell behavior. Keep them scoped
   to Linux/container proof.
3. Large GGUF downloads can make routine release gates unreliable. Split
   no-download readiness from real-model proof.
4. Hosted Windows CI may not prove Defender, long paths, or redirected profile
   behavior. Use self-hosted runners or record limitations when needed.
5. Report generation can become noisy. Keep the runner reports structured and
   concise, with command logs stored as files.

## Closure Criteria

This plan is complete when:

1. The runner exists and is documented.
2. The no-model black-box story passes from built artifacts on macOS arm64,
   Linux amd64, Linux arm64, and Windows x86_64.
3. Windows host proof covers Known Folder paths, path with spaces, PowerShell
   invocation, `.exe` runnability, JSON path escaping, and uninstall behavior.
4. Real-model GGUF CPU proof is recorded for each platform where
   semantic/hybrid search is claimed.
5. Release E2E reports are attached to the release gate and summarized in a
   wiki eval or checklist.
