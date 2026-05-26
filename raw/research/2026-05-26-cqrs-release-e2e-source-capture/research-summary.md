# Research Summary: CQRS Release E2E Harness Source Capture

- Date: 2026-05-26
- Source Bundle: `raw/research/2026-05-26-cqrs-release-e2e-source-capture/manifest.md`
- Scope: Local CQRS/Pulumi E2E patterns relevant to the `llm-wiki`
  cross-platform release E2E harness plan.

## Key Findings

The bundle includes project-owned snapshots under `sources/` for the inspected
CQRS files. The local external paths remain in the manifest only as provenance;
the source material used by the wiki is now the copied snapshot set plus
SHA-256 hashes.

The CQRS project uses a single Rust test runner as the E2E command surface.
The runner owns test categories, flags, subprocess execution, report paths, and
JUnit output. Relevant flags include `--skip-infra`, `--keep-infra`,
`--fail-fast`, `--headed`, `--output-dir`, `--stdout`, and `--verbose`.

The CQRS `justfile` keeps public test commands small and delegates lifecycle to
private recipes. `just test e2e` starts the E2E stack, waits for readiness,
runs the Rust runner, and tears the stack down. `test-skip-infra` reuses an
already-running stack. The private Pulumi startup recipe serializes stack
startup with `--parallel 1` because Docker Desktop can deadlock under parallel
BuildKit image builds.

The CQRS Pulumi stack is profile-driven. Dev uses one port profile, E2E uses
another, and the Pulumi modules resolve stack-specific ports and service
enablement from `infra/src/ports.ts`. Stack-specific Docker network names keep
dev and E2E containers isolated.

Runtime configuration is profile-based rather than embedded in shell scripts.
`infra/src/load-service-env.ts` maps Pulumi stack names to runtime profiles,
loads environment files, validates required keys, and then merges dynamic
Pulumi overrides. The test runner also loads an E2E profile and has tests that
assert key endpoint variables and E2E ports.

The CQRS infra model proves that Pulumi is useful for repeatable containerized
test infrastructure, but it does not imply that Docker proves every platform.
For `llm-wiki`, Docker/Pulumi should be limited to honest Linux/container
lanes, release-artifact smoke, model-cache mounting, reports, and teardown.
macOS and Windows support claims still require native host, VM, or CI runners
because shell behavior, filesystem behavior, security products, Known Folder
resolution, `.exe` invocation, and installed runtime discovery are part of the
product contract.

The CQRS accepted testing ADR also supports target-aware proof. Its
multiplatform parity lane shares scenario intent while using target-specific
drivers and baselines. The transferable lesson for `llm-wiki` is that a native
runner, an emulated container lane, a no-download readiness lane, and a
real-model GGUF lane are different proof types and must be labeled separately.

## Implications For `llm-wiki`

1. Add a release-E2E runner rather than a loose shell script collection.
2. Keep `just` as a thin entry point for common lanes.
3. Use profiles and required-key validation for artifact paths, target triples,
   shell mode, temp root policy, model-cache policy, and Docker/Pulumi inputs.
4. Use Pulumi for Linux/container lanes only, not as a substitute for Windows
   or macOS host proof.
5. Emit durable reports: JSON, JUnit, stdout/stderr logs, and a state manifest
   recording expected and actual filesystem effects.
6. Separate routine no-download readiness checks from real-model GGUF CPU
   proof.
7. Treat emulated Docker results as explicitly labeled packaging smoke unless
   the runner is native for the target being claimed.

## Wiki Pages Supported

- `wiki/plans/cross-platform-release-e2e-harness.plan.md`
- `wiki/proposals/full-windows-support.proposal.md`
- `wiki/plans/gguf-runtime-portability.plan.md`
