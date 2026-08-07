# Eval: Release E2E GGUF CPU Proof

Document Class: Eval
Status: Accepted
Date: 2026-06-22
Category: Release E2E, GGUF runtime, semantic search
Scope: Host GGUF release-E2E proof for the `gguf` lane in `tools/release-e2e`.
Sources: `target/release-e2e/gguf/report.json`; `target/release-e2e/gguf/stdout/search-semantic-cpu.out`; `target/release-e2e/gguf/stdout/search-hybrid-cpu.out`; `target/release-e2e/gguf/stdout/search-all-auto-cpu.out`; `wiki/plans/cross-platform-release-e2e-harness.plan.md`.

## Result

Accepted. The local macOS arm64 GGUF release-E2E lane passed on 2026-06-22:

```text
rtk cargo run --manifest-path tools/release-e2e/Cargo.toml -- gguf --artifact target/debug/llm-wiki --manual-models --reuse-managed-models-from /Users/nicolasmartino/.llm_wiki
```

The report recorded `success = true`, target triple `aarch64-macos`, host
`macos/aarch64`, artifact SHA-256
`23fbbe655b9ab103a1bb3714be3a2ebe285225548f263c2d2c868d9f3263cb6c`, 10
commands, 33 assertions, and no failed assertions.

## Coverage

The lane used an isolated managed home and scratch project, reused local model
artifacts from `/Users/nicolasmartino/.llm_wiki`, then ran the release artifact
through documented CLI commands. It installed the balanced LLM search profile,
registered a temporary project, seeded scoped thresholds for the fixture,
indexed with `LLM_WIKI_GGUF_RUNTIME=cpu`, ran semantic search, hybrid search,
and `search-all --mode auto`, then verified uninstall cleanup.

Semantic, hybrid, and search-all all reported:

```text
runtime_backend_requested = cpu
runtime_backend_used = cpu
runtime_backend_fallback = false
top result = wiki/proposals/project-update-command.proposal.md
```

## Limits

This proof covers the local macOS arm64 host artifact path with reused local
GGUF bytes. It does not close native Linux amd64, native macOS release-archive,
Windows host/VM, or no-cache model download proof.
