# Testing and Verification Architecture

- Document Class: ADR
- Status: Accepted
- Date: 2026-04-14
- Category: Verification architecture
- Scope: Current baseline

## Test pyramid

- **Unit tests**: domain rules, validators, mappers, auth helpers
- **Integration tests**: per service with Postgres and any required Kafka topics
- **Contract tests**: public mutation DTOs, internal mutation responses, propagation events
- **E2E tests**: browser and cross-service user flows (see lane exceptions below)

## Minimum required suites

### Unit tests

Each crate should cover:

- domain/value-object invariants
- HTTP mutation request validation
- authorization predicates
- cache invalidation and mapping logic

### Service integration tests

Each internal service should verify:

- migrations apply cleanly
- trusted internal mutation handlers enforce auth and persist correct state
- remaining Kafka consumers (`keycloak-events`, `user.sync`) dedupe and apply correctly
- exercise search indexing consumes `exercise.index`, rehydrates authoritative Postgres state, and upserts/deletes the Elasticsearch document
- query handlers return the expected read model

### BFF integration tests

The app/BFF should verify:

- public CRUD and `/api/v1/sync` requests normalize into `mutation_jobs`
- dispatcher success, retry, and terminal-failure semantics
- short wait timeout behavior returns `Pending` without losing durability
- invalidation publish failures do not roll back terminal mutation completion

### Security regression tests

The system should verify:

- end-user bearer tokens are not stored in Kafka payloads or durable mutation tables
- internal mutation handlers reject missing or untrusted delegated-auth headers
- invalid JWTs are rejected on public and read-side HTTP paths

### Contract tests

The system should verify:

- public and internal OpenAPI documents are parseable
- shared contract types round-trip for:
  - public mutation DTOs
  - mutation outcomes
  - `keycloak-events`
  - `user.sync`
  - `exercise.index`
  - `invalidations.v1`

## E2E lane boundary

The E2E crate (`code/tests/e2e/`) contains live-stack tests that require the full Docker/Pulumi environment. Its primary scope is browser flows and cross-service user-visible tests through public APIs.

### Multiplatform frontend parity lane

The frontend parity lane verifies selected user-visible flows across web, Android, iOS, and desktop with one shared scenario layer. It is intentionally narrower than the browser reference suite.

Parity scenarios must depend only on:

- the shared probe contract exposed by the app in E2E builds
- required driver capabilities such as launch, reset local state, authenticate, open screen, semantic action, snapshot read, screenshot capture, and close
- public E2E stack endpoints

The app-owned probe is enabled only in debug builds or builds compiled with
`REPFORGE_E2E_PROBE_ENABLED=1`. The Pulumi e2e stack sets that flag for the
release fullstack image; non-e2e release builds must not expose the probe action
channel.

Parity scenarios must not depend on browser-only internals such as Chromium DevTools Protocol, IndexedDB inspection, `localStorage`, JavaScript evaluation, DOM marker assertions, console logs, or hydration diagnostics. Those remain browser-reference responsibilities.

Current required parity coverage:

- `preferences_bootstrap_and_persistence` runs on web, Android, iOS, and desktop.
- `preferences_live_propagation` runs the ring `web -> android -> ios -> desktop -> web`, asserting preference convergence, version movement, and `sync.last_source == "sse"` through the shared probe.
- `exercise_library_browse` runs as the first non-preferences browse scenario on web, Android, iOS, and desktop.

Runtime parity screenshots use target-aware baselines under `code/tests/e2e/baselines/<target>/<host-os>/` rather than assuming web, Android, iOS, and desktop are pixel-identical. Web uses Chromium screenshot capture, Android uses emulator framebuffer capture, iOS uses simulator capture, and desktop uses the native E2E probe's app-owned `/screenshot` endpoint with bundled `html2canvas` rendering so desktop baselines are not produced by OS-level window capture.

### Accepted exceptions

1. **Collocated helper unit tests.** `#[cfg(test)]` unit tests for e2e helper types (e.g. `CreateKeycloakUser` builders, `TestConfig` defaults, `CorrelationTracker` logic) may remain in the E2E crate when they test e2e-specific helpers that have no other natural home. These tests do not require the live stack and do not add noise to the functional gate.

2. **Keycloak-event propagation tests.** `user_propagation_test.rs` enters the system through the Keycloak Admin API rather than a user-facing surface, but it proves the cross-service Keycloak -> Kafka -> user-api -> workout-api/exercise-api pipeline. This is a valid E2E entry point because the test requires the full live stack and verifies behavior that spans multiple real services.

## Coverage gate

Workspace-wide host coverage remains an informative summary because the monorepo still contains
browser/demo packages whose denominator is not yet a useful release gate. Enforced coverage uses
repository-owned scopes instead of the raw workspace total.

The active branch targets are:

- `server_mutation_path >= 80%`
- `runtime_backend >= 80%`
- `backend_core >= 80%`

## Critical scenarios

| Scenario | Expected behavior |
|---|---|
| Internal service responds within timeout | Public endpoint returns terminal result |
| Internal service does not respond within timeout | Public endpoint returns `Pending` and retry can observe later completion |
| Same `mutation_id` retried | Stable idempotent result |
| Dispatcher cycle fails | Readiness turns unhealthy until recovery |
| Kafka invalidation publish fails | Mutation stays terminal; propagation is degraded only |
| Elasticsearch unavailable | Exercise discovery/search returns 503; writes and detail reads continue to use Postgres |
| `exercise.index` pipeline lags | Search freshness degrades visibly through monitoring; no Postgres search fallback masks the issue |
| Full reindex after drift | Elasticsearch is rebuilt from authoritative Postgres rows and stale derived documents are removed |
