# Manifest: Headroom Dogfood Evidence (2026-06-21)

- Bundle date: 2026-06-21
- Captured by: Headroom Runtime Companion implementation (Phase 7 of
  `wiki/plans/headroom-runtime-companion.plan.md`).
- Purpose: Hold raw evidence for the Headroom dogfood evidence pass. This bundle
  currently contains the real containerized proxy **smoke** capture produced by
  the gated `just headroom-proxy-e2e` lane. It does **not** yet contain the
  comparative Mode C vs Mode D live-agent ingest measurement; that run is
  pending (see `research-summary.md`).
- Provenance: All `captures/*` files are verbatim outputs from a single run of
  `infra/headroom-proxy-e2e/` against a real `headroom-ai==0.24.0` install in a
  Debian-based container. No project `wiki/` or `raw/` content was routed
  through the proxy; the smoke uses synthetic, content-free probes only.

## Files

| File | Source | Notes |
| --- | --- | --- |
| `captures/health.json` | `GET http://127.0.0.1:8787/health` | Proxy liveness; shows `status: healthy`, `optimize: true`, `rust_core: loaded`. |
| `captures/headroom_stats.json` | `GET /stats` | Full stats document. Compression counters are zero because the smoke sent no upstream traffic. |
| `captures/retrieve_stats.json` | `GET /v1/retrieve/stats` | CCR retrieve stats at smoke time. |
| `captures/profile.sha256` | materialized profile hash | `sha256` of `~/.llm_wiki/headroom/llm-wiki.profile.env` inside the container. |
| `captures/headroom-version.txt` | `importlib.metadata.version('headroom-ai')` | `0.24.0`. |
| `captures/headroom-package-dir.txt` | `importlib.util.find_spec('headroom')` | Installed package directory inspected by the install-time pin. |
| `captures/proxy.stdout` | `headroom proxy` stdout | Server boot log. |

## Reproduce

```bash
just headroom-proxy-e2e
# artifacts under target/headroom-proxy-e2e/
```

The container builds the Linux `llm-wiki` binary, installs
`headroom-ai[proxy]==0.24.0`, runs `llm-wiki install --with-headroom` under an
isolated HOME (the install-time package/source-surface pin runs against the real
package), sources the materialized profile, starts `headroom proxy`, captures
the stats endpoints, and asserts exactly one `llm-wiki doctor` Mode C advisory.
