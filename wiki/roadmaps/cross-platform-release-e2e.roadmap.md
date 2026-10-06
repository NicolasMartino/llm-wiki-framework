# Cross-Platform Release E2E Roadmap

- Document Class: Roadmap
- Status: Active
- Date: 2026-06-04
- Category: Release engineering, E2E, platform support
- Scope: Coordinate the post-V1 deliverables that prove release artifacts on
  Linux, macOS, Windows, and GGUF CPU paths through target-aware E2E evidence.
- Sources: templates/base/project_guidelines.md;
  wiki/proposals/full-windows-support.proposal.md;
  wiki/plans/cross-platform-release-e2e-harness.plan.md;
  wiki/plans/gguf-runtime-portability.plan.md;
  wiki/plans/gguf-runtime-smoke-probes.plan.md; wiki/index.md; wiki/log.md
- Related: wiki/proposals/full-windows-support.proposal.md,
  wiki/plans/cross-platform-release-e2e-harness.plan.md,
  wiki/plans/gguf-runtime-portability.plan.md,
  wiki/proposals/deterministic-e2e-backbone.proposal.md
- Parent Proposal: wiki/proposals/full-windows-support.proposal.md
- Execution Plan: wiki/plans/cross-platform-release-e2e-harness.plan.md

## Objective

Make every supported `llm-wiki` release artifact prove the documented product
story on its target platform before the project claims support for that target.

The roadmap separates proof types:

1. Docker and Pulumi prove Linux/container behavior.
2. Native or hosted runners prove host operating-system behavior.
3. Emulated Docker runs are packaging smoke only.
4. Windows support remains unproven until Windows-specific path, shell,
   artifact, and E2E gates pass on a Windows host, VM, or CI runner.
   Once they pass, a Windows job joins the full CI that runs on PRs into
   master (the owner, 2026-10-06), not before.
5. GGUF semantic/hybrid support remains unproven for a platform until a real
   managed-model CPU release proof passes for that platform.

## Status Boundary

This roadmap fixes the missing coordination layer between the parent proposal
and the active harness plan. It does not mark Windows support complete and does
not convert Docker results into macOS or Windows proof.

The parent proposal remains the support definition for Windows and
cross-platform release claims. The active plan remains the tactical execution
document. This roadmap owns ordering, dependencies, and proof gates.

## Sequencing Principles

1. Prove artifacts, not `cargo run`.
2. Keep no-download readiness proof separate from real-model GGUF proof.
3. Label native, virtualized, and emulated evidence distinctly.
4. Add platform support claims only after target-platform reports exist.
5. Prefer one shared runner/report contract, with platform-specific execution
   substrates.

---

### R1 - Release E2E Runner And Host Product Story

Status: Completed
Promise: A standalone release E2E runner can execute a built `llm-wiki`
artifact through the documented CLI surface from an isolated user/runtime home.
Depends On: Framework V1 D8-D11; semantic/hybrid readiness behavior
Execution Plan: wiki/plans/cross-platform-release-e2e-harness.plan.md

Included:
- independent `tools/release-e2e` runner outside the shipped product archive
- `smoke` and `search` lanes
- isolated HOME/XDG state and scratch project roots with spaces in paths
- stdout/stderr capture, JSON reports, JUnit output, and state manifests
- no-model product story covering install, build, init, register, index,
  lexical search, semantic/hybrid fail-closed readiness, forget, and uninstall

Excluded:
- packaged release archives
- Linux Docker execution
- native macOS, native Linux, Windows, and real GGUF proof

Proof:
- `cargo test --manifest-path tools/release-e2e/Cargo.toml`
- `just release-e2e smoke`
- `just release-e2e search`
- reports under `target/release-e2e/smoke/` and `target/release-e2e/search/`

Promotion Target:
- wiki/plans/cross-platform-release-e2e-harness.plan.md
- future release E2E checklist or eval

Unlocks:
- Linux container lanes
- packaged archive lanes
- native host lanes using the same runner contract

---

### R2 - Linux Docker Archive Simulation

Status: Completed
Promise: Linux release artifacts can be built inside Linux Docker, verified by
checksum, unpacked, and run through the no-model product story in a minimal
Debian runtime container.
Depends On: R1
Execution Plan: wiki/plans/cross-platform-release-e2e-harness.plan.md

Included:
- Pulumi/Docker Linux infrastructure profile
- Linux Docker builder with qmd/llama native build dependencies
- cargo-dist archive build inside Linux
- checksum verification before extraction
- safe archive extraction and packaged binary discovery
- Linux arm64 virtualized container product story
- Linux amd64 emulated packaging smoke on macOS arm64
- explicit report metadata for proof kind and architecture-native status

Excluded:
- native Linux amd64 host proof
- macOS host proof
- Windows host proof
- real GGUF CPU release proof

Proof:
- `just release-e2e-linux-dist-build-and-test` for Linux arm64
- `just release-e2e-linux-dist-build-and-test linux/amd64 ...` for amd64
  emulated packaging smoke
- `ldd` checks showing no `libgomp` runtime dependency
- reports under `target/release-e2e/` and `target/release-e2e-linux-amd64/`

Promotion Target:
- wiki/plans/cross-platform-release-e2e-harness.plan.md

Unlocks:
- native Linux amd64 proof
- routine Linux archive regression gates

---

### R3 - Native Linux amd64 Host Proof

Status: Active
Promise: The Linux amd64 release archive is built and executed on a real
`x86_64` Linux host, not through Docker Desktop emulation.
Depends On: R1; R2
Execution Plan: wiki/plans/cross-platform-release-e2e-harness.plan.md

Included:
- native `x86_64-unknown-linux-gnu` cargo-dist archive build
- checksum-verified archive product story on the native host
- GitHub Actions workflow on `ubuntu-24.04`
- report upload with archive, checksum, and E2E evidence
- `ldd` inspection of the extracted packaged binary

Excluded:
- macOS proof
- Windows proof
- real GGUF CPU proof

Proof:
- `.github/workflows/release-e2e-linux-amd64.yml` passes on a real
  `Linux/x86_64` runner
- `just release-e2e-native-linux-dist-build-and-test`
- report under `target/release-e2e-native-linux-amd64/`

Promotion Target:
- release E2E checklist or eval summarizing native Linux amd64 evidence

Unlocks:
- honest Linux amd64 release support evidence
- stronger CI release gate for Linux archive regressions

---

### R4 - Native macOS Release Archive Proof

Status: Draft
Promise: macOS release archives run the same no-model product story on native
macOS host runners.
Depends On: R1
Execution Plan: wiki/plans/cross-platform-release-e2e-harness.plan.md

Included:
- macOS arm64 archive build and no-model product-story proof on this local
  machine or a hosted macOS arm64 runner
- macOS x86_64 host/CI proof before claiming x86_64 macOS support
- platform-shell invocation through the runner
- report metadata distinguishing host architecture and target triple

Excluded:
- Docker substitution for macOS host behavior
- Windows path or shell behavior
- real GGUF CPU proof, which is R7

Proof:
- macOS arm64 archive recipe passes from a checksum-verified cargo-dist archive
- macOS x86_64 workflow or host run passes from the x86_64 archive
- reports record target triple, host OS, host architecture, commands, and
  assertions

Promotion Target:
- release E2E checklist or eval summarizing macOS artifact proof

Unlocks:
- native macOS release support evidence
- a reusable pattern for Windows native host lanes

---

### R5 - Windows Runtime Path And Artifact Baseline

Status: Draft
Promise: The Windows binary builds as a supported release artifact and uses
Windows runtime paths, executable semantics, and rendered skill commands that
match the Windows support definition.
Depends On: R1; parent proposal acceptance/source-capture cleanup
Execution Plan: Not created yet for Windows implementation; proof lane remains
in wiki/plans/cross-platform-release-e2e-harness.plan.md

Included:
- release target metadata for `x86_64-pc-windows-msvc`
- Windows Known Folder based managed/cache/data paths
- managed `llm-wiki.exe` install path and manifest behavior
- PowerShell-safe installed skill command rendering
- path-with-spaces and JSON path escaping behavior
- qmd-rs/search packaging investigation on Windows

Excluded:
- Windows ARM64 unless separately accepted
- WSL-specific support
- Authenticode/code signing unless a later decision adds it
- UNC/network path support

Proof:
- Windows build succeeds for the accepted target
- path helper tests pass without requiring Unix `HOME` or XDG state
- rendered skill commands execute through PowerShell and reach the managed
  `.exe`
- qmd-rs indexing/search is available or a narrowing decision is recorded

Promotion Target:
- wiki/proposals/full-windows-support.proposal.md
- future Windows path/support decision or spec update after validation

Unlocks:
- Windows host release E2E proof

---

### R6 - Windows Host Release E2E Proof

Status: Draft
Promise: A Windows user can run the supported release artifact through the
documented install, project, registry, search, doctor, skill-invocation, and
uninstall story without WSL or a Rust toolchain.
Depends On: R5
Execution Plan: wiki/plans/cross-platform-release-e2e-harness.plan.md; a
dedicated Windows implementation plan may be required before execution

Included:
- Windows host, VM, or CI runner proof
- PowerShell command invocation including quoted executable paths
- Known Folder managed paths
- Defender/default runnability posture
- path with spaces
- long-path gate or recorded limitation
- `.exe` invocation and uninstall cleanup behavior
- JSON round-trip checks with Windows paths

Excluded:
- Docker-only Windows proof as a replacement for host/VM proof
- Windows desktop support wording from server-only CI
- unsupported Windows ARM64 claims

Proof:
- Windows release E2E report passes from a checksum-verified artifact
- `doctor` invokes the managed `.exe`
- installed skill command strings execute through the target shell
- registry/index/search/eval JSON parses and paths resolve

Promotion Target:
- Windows support spec or decision after validation
- release E2E checklist or eval summarizing Windows evidence

Unlocks:
- first-class Windows release support claim

---

### R7 - GGUF CPU Release Artifact Proof

Status: Draft
Promise: Semantic and hybrid search with managed GGUF models work through
release artifacts on every platform where semantic/hybrid support is claimed.
Depends On: R3; R4; R6 for Windows; GGUF runtime portability implementation
Execution Plan: wiki/plans/cross-platform-release-e2e-harness.plan.md;
wiki/plans/gguf-runtime-portability.plan.md

Included:
- no-download readiness lane for routine CI
- real managed-model CPU lane for semantic/hybrid proof
- install-owned license consent, SHA-256 verification, artifact records,
  runtime probes, and `search.toml` enablement
- `LLM_WIKI_GGUF_RUNTIME=cpu` metadata checks
- semantic, hybrid, and `search-all` retrieval from a small temp wiki fixture

Excluded:
- accelerator support as a prerequisite
- hidden model downloads from search/index/doctor
- deterministic test hooks as release proof
- hybrid-quality redesign

Proof:
- release artifact installs the balanced profile with explicit consent
- semantic, hybrid, and `search-all` return the project-update fixture page
- JSON records `runtime_backend_requested=cpu`,
  `runtime_backend_used=cpu`, and `runtime_backend_fallback=false`
- reports preserve stdout/stderr and runtime metadata

Promotion Target:
- wiki/plans/gguf-runtime-portability.plan.md
- future release E2E checklist or eval
- semantic/hybrid platform support notes

Unlocks:
- semantic/hybrid release support claims per platform

---

### R8 - Release Evidence Gate

Status: Draft
Promise: Platform support status is backed by a durable release E2E evidence
record instead of scattered ad hoc terminal runs.
Depends On: R2; R3; R4; R6; R7 where semantic/hybrid support is claimed
Execution Plan: Not created yet

Included:
- release E2E checklist or eval page
- per-lane report summaries
- explicit skipped-gate limitations
- support-claim table by platform, target triple, proof kind, and GGUF status
- release process hook requiring matching reports before support wording changes

Excluded:
- changing validated specs before proof exists
- counting emulated packaging smoke as native support proof
- counting no-model readiness as real GGUF proof

Proof:
- a release E2E checklist or eval exists and is linked from the index
- each supported platform claim cites a passing report
- skipped gates are named as limitations

Promotion Target:
- wiki/specs/documentation-model.spec.md
- release notes/checklist content when added

Unlocks:
- clear release readiness decisions
- future platform additions without re-deriving the proof model
