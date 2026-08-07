# Research Summary: Headroom Dogfood Evidence (2026-06-21)

This is the raw evidence bundle for the Headroom dogfood evidence pass (Phase 7
of `wiki/plans/headroom-runtime-companion.plan.md`). Per that phase, this
bundle may produce only raw evidence; a `wiki/references/` page is promoted only
if a comparative measurement shows meaningful savings with no provenance
breakage. **No reference page is promoted from this bundle**, because the
comparative measurement has not been run yet — only a real-proxy smoke has.

## Measurement fixture (defined before interpreting any result)

The intended comparative dogfood, when run, must hold these fixed:

- **Workload**: one fixed ingest fixture (a single `raw/research/` bundle)
  ingested identically in both runs.
- **Variable under test**: Mode D (`HEADROOM_OPTIMIZE=false`) baseline vs Mode C
  (vetted profile sourced, optimize on) — the only difference between runs.
- **Cache policy**: reset CCR between runs (fresh proxy process per run) so one
  run's cache cannot warm the other.
- **Token-count method**: compare the proxy's reported before/after token counts
  and `headroom_stats` compression counters for the Mode C run; record the
  Mode D run's totals as the uncompressed baseline.
- **Redaction**: transcripts redacted to remove any secret material before they
  enter `raw/`; only token counts, `headroom_stats`, and advisory snippets are
  retained.
- **Provenance check**: after each run, diff the agent's written `wiki/` pages
  against the source to confirm no citation references a line/item that was
  absent from the (possibly compressed) view. Any such mismatch is a
  provenance breakage and fails the run regardless of savings.

## Threshold for "meaningful" (defined up front)

A promoted positive `wiki/references/headroom-dogfood-evidence.reference.md` is
warranted only if the Mode C run shows **both**:

1. a real reduction in tokens delivered to the model on incidental tool output
   (target: a clearly non-noise reduction, e.g. >= 15% on the compressible
   portion of the workload as reported by `headroom_stats`), AND
2. zero observable provenance breakage on the `wiki/`/`raw/` citation check.

A flat, negative, or provenance-breaking result stays here in `raw/` and does
not promote a positive reference page.

## What was actually run (2026-06-21)

A real containerized proxy **smoke**, via `just headroom-proxy-e2e`, against
`headroom-ai==0.24.0`. Captured in `captures/`:

- `health.json`: the proxy booted and reported `status: healthy`, `ready: true`,
  `version: 0.24.0`, `config.optimize: true`, `rust_core: loaded`. This confirms
  Mode C (optimization on) with the vetted profile sourced.
- `headroom_stats.json`: the `/stats` document is reachable and well-formed. All
  compression counters are **zero** (`requests_compressed: 0`,
  `api_requests: 0`) because the smoke deliberately sent no upstream model
  traffic — it used synthetic, content-free probes only.
- `profile.sha256`: `70553e88b044aa7f5f229d8544885a51bbba9c896745515f10a47c0ccc2c999e`,
  identical to the embedded `assets/headroom/llm-wiki.profile.env`, proving the
  materialized profile is byte-for-byte the vetted asset inside a real install.

## Conclusion

- **Proven**: the vetted profile materializes byte-identically in a real
  install, `headroom-ai 0.24.0` passes the install-time package/source-surface
  pin, and a real `headroom proxy` runs with optimization on and the profile's
  exclude set. The `llm-wiki doctor` Mode C advisory fires exactly once against
  the real proxy.
- **Pending**: the comparative Mode C vs Mode D token-savings measurement over a
  representative ingest workload. It requires routing a live agent ingest
  session through the proxy, which is outside the implementation environment
  here. The smoke is not a representative workload and intentionally produced
  zero compression, so it cannot stand in for the comparative measurement.
- **Therefore**: no positive dogfood reference page is promoted. The safety and
  architecture of the companion (opt-in, tool-name carve-out, upstream pin,
  doctor advisory, no SDK/vendor) are validated by the implementation, tests,
  and the real-proxy smoke independent of any savings number, consistent with
  the proposal's statement that absent/negative savings do not block the safety
  decision.
