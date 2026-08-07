# Research Manifest: CQRS Release E2E Harness Source Capture

- Date: 2026-05-26
- Research Question: What reusable release-E2E and Pulumi infrastructure
  patterns from the local CQRS full-stack project should inform the
  `llm-wiki` cross-platform release E2E plan?
- Goal: Capture the external local-repository evidence inside this project's
  `raw/` tree so the wiki does not depend only on absolute paths outside the
  framework repository.
- Source Modes: local repository inspection
- Selection Method: User identified `/Users/nicolasmartino/Documents/workout/cqrs-fullstack`
  as the relevant precedent, especially its Pulumi infrastructure and E2E
  model. The selected files are the minimum set needed to support the
  release-runner, profile, lifecycle, and platform-proof claims now reflected
  in the wiki.

## Source Inventory

The files below are project-owned snapshots copied into this source bundle.
The original local paths are recorded only as provenance; future wiki readers
can rely on the `sources/` copies and hashes.

| Snapshot | Original path | Purpose | SHA-256 |
| --- | --- | --- | --- |
| `sources/001-cqrs-agents.md` | `/Users/nicolasmartino/Documents/workout/cqrs-fullstack/AGENTS.md` | Repository-level instructions, including Pulumi-managed infrastructure and E2E command semantics | `78aa909908898004e5cff4fec0d2c0c066b2d18910ea8aab42f6cd873e56e499` |
| `sources/002-cqrs-justfile.txt` | `/Users/nicolasmartino/Documents/workout/cqrs-fullstack/justfile` | Public `just test e2e` / `test-skip-infra` commands and private `_e2e-infra-up`, `_e2e-infra-down`, and `_wait-for-services` lifecycle recipes | `ce208162239226aed5b4545735d59897111581f80f976e47ca4fe74e4cce0cbf` |
| `sources/003-cqrs-infra-index.ts` | `/Users/nicolasmartino/Documents/workout/cqrs-fullstack/infra/index.ts` | Central Pulumi composition and exported stack outputs | `0ecd07f48a644998597ccddeafa3a0b92537f95fb7d8f1305b78b15160e55103` |
| `sources/004-cqrs-infra-ports.ts` | `/Users/nicolasmartino/Documents/workout/cqrs-fullstack/infra/src/ports.ts` | Dev/e2e stack profile ports and service enablement | `10adfb35f8ed8ab676c5ec40f16625e027847061ca15ebd54b534c3f821a3747` |
| `sources/005-cqrs-load-service-env.ts` | `/Users/nicolasmartino/Documents/workout/cqrs-fullstack/infra/src/load-service-env.ts` | Stack-to-environment profile loading and required-key validation | `75f07f8074e881b87653b66182d22c93d87c220121380dbf4ddbbc3ca9f98445` |
| `sources/006-cqrs-network.ts` | `/Users/nicolasmartino/Documents/workout/cqrs-fullstack/infra/src/network.ts` | Stack-specific Docker network naming and isolation | `86185da20a0fa8cc613ae6f3a70bb8a74b27ecec9550f5a10208504f1f935fb2` |
| `sources/007-cqrs-fullstack.ts` | `/Users/nicolasmartino/Documents/workout/cqrs-fullstack/infra/src/fullstack.ts` | Docker image build platform selection, e2e probe enablement, and e2e-specific service behavior | `ce878423f3d22b6d58ef4423e5a20a944b6f3e550c5c564d893c35ec558869ef` |
| `sources/008-cqrs-nginx.ts` | `/Users/nicolasmartino/Documents/workout/cqrs-fullstack/infra/src/nginx.ts` | E2E-only ingress/proxy composition | `8b1f701ad8060dce598ed6493df08a8e2672e1c6d3fddce09dd31265ba19b241` |
| `sources/009-cqrs-test-runner-cli.rs` | `/Users/nicolasmartino/Documents/workout/cqrs-fullstack/code/tools/test-runner/src/cli.rs` | Rust runner categories and lifecycle/report flags | `9839d64e84889872833c97bc916b14e70ddb6e57dd17576f9312c3623b3166d8` |
| `sources/010-cqrs-test-runner-env.rs` | `/Users/nicolasmartino/Documents/workout/cqrs-fullstack/code/tools/test-runner/src/env.rs` | E2E profile loading and assertions for required endpoint variables | `eeaccd506c7a0f1f2db58bc1d163a12393c6291e70ea2a77cc559e816eee339b` |
| `sources/011-cqrs-test-runner-infra.rs` | `/Users/nicolasmartino/Documents/workout/cqrs-fullstack/code/tools/test-runner/src/infra.rs` | RAII-style infra guard, `--skip-infra`, health checks, and teardown | `fcbe7bb97bde0330809b9cdaea9401530d8b188edcd23aff895e5cb3e09174b6` |
| `sources/012-cqrs-test-runner-suite-e2e.rs` | `/Users/nicolasmartino/Documents/workout/cqrs-fullstack/code/tools/test-runner/src/suite/e2e.rs` | nextest/JUnit E2E command construction and profile env injection | `5812e9f519ff193c702420289ed65fa6e78ffdf49e28885f7323d8b0e6e6d08c` |
| `sources/013-cqrs-testing-verification-architecture.adr.md` | `/Users/nicolasmartino/Documents/workout/cqrs-fullstack/docs/current/adr/testing-and-verification-architecture.adr.md` | Accepted E2E lane boundary and target-aware parity proof model | `226acb452fedfbaff0829b93ebe743682d6ceb405dcab4f4b8de41257ff585ec` |

## Gaps

- This capture preserves selected source snapshots, not the full CQRS
  repository. It is enough to support the release-runner, Pulumi lifecycle,
  profile-loading, health-check, and target-aware proof claims currently made
  by the Proposed plan.
- The original source paths are local to the author's machine. They are
  provenance only; the durable source material is the copied `sources/` tree
  plus the SHA-256 hashes above.
- This capture does not prove that the same Pulumi/test-runner implementation
  should be copied mechanically. It supports the narrower plan claim: reuse the
  lifecycle/profile/reporting architecture while keeping platform proof honest.

## Ingest Readiness

Ready for ingest into the cross-platform release E2E plan and Windows support
proposal. Recommended document type: plan/proposal updates, not a standalone
reference unless the release-E2E architecture becomes a broader reusable
framework pattern.
