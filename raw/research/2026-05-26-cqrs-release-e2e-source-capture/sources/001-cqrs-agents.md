# AGENTS.md

This file provides guidance to Codex (Codex.ai/code) when working with code in this repository.

## Project Overview

RepForge is a Rust monorepo for a fitness/workout platform implementing CQRS with Kafka-based messaging. It features:
- Multi-service backend architecture (BFF, user-api, workout-api)
- Cross-platform Dioxus frontend (web, desktop, mobile)
- Offline-first with optimistic updates and local persistence
- Keycloak for OIDC authentication

## ⚠️ CRITICAL: Compilation Requirement

**NO TASK IS COMPLETE UNTIL ALL CRATES COMPILE.**

Before considering any task finished, you MUST verify:

```bash
cargo check --workspace
```

This command must pass with ZERO compilation errors. Warnings are acceptable, but errors are not.

**Why this matters:**
- Rust is a workspace - changes in one crate can break others
- A passing `cargo check -p my-crate` does NOT guarantee the workspace compiles
- Pre-existing errors in other crates must be fixed, not ignored
- Breaking the build blocks all development work

**When to check:**
- ✅ After every code change that modifies Rust files
- ✅ Before declaring a task complete
- ✅ After deleting files or refactoring
- ✅ After updating dependencies

If `cargo check --workspace` fails, you must fix ALL errors before the task is complete, even if they appear unrelated to your changes.

## ⚠️ CRITICAL: No Lint Suppression

**NEVER suppress lint warnings.** Fix the underlying code instead.

Forbidden patterns:
- `#[allow(clippy::...)]` — fix the Rust code
- `// eslint-disable` — fix the TypeScript code
- `// allow-raw-var` or similar bypass comments
- `--max-warnings` set to anything other than 0

Every lint rule exists for a reason. When clippy or ESLint flags something, the correct response is to fix the code, not to suppress the warning. This applies to all languages and all linters in the project.

`just validate` is the single gate that enforces this. It must pass with zero warnings and zero suppressions.

## Build Commands

```bash
# Rust builds
cargo build                    # Full workspace
cargo build -p bff             # Specific service
cargo build --release          # Production

# Tests
cargo test                     # All tests
cargo test -p user-api         # Specific crate
cargo test -p user-api -- test_verify_email  # Single test by name

# Linting
cargo clippy
cargo fmt
```

## Development with Just

Use `just` (justfile) for development. Run `just help` for all commands.

```bash
# Dev stack (Pulumi/Docker)
just dev              # Start shared deps + all backend service containers
just dev deps         # Start shared deps only (Postgres, Kafka, Keycloak, etc.)
just dev exercise-api # Start shared deps + only exercise-api
just dev down         # Destroy the dev stack
just dev down exercise-api  # Disable one service, keep deps up

# Local service runner (host process against dev stack)
just local fullstack  # Run fullstack/BFF locally with profile env vars
just local exercise-api  # Run exercise-api locally

# Direct commands
just fullstack        # Dioxus server only (:7081)
just desktop          # Desktop app
just android          # Build + install Android (requires emulator)
just ios-dev          # Build + install iOS simulator
just gallery          # Component preview app (:7100)

# Quality checks
just check            # Check all targets compile
just check-web        # Check wasm32 target
just check-tokens     # Check for CSS vars that should use semantic tokens
just lint             # Clippy all crates
just fmt              # Format code

# Validation (lint, format, type checks — all languages)
just validate         # Full validation pyramid (Rust + TypeScript + i18n + CSS tokens)

# Testing
just test             # Full gate: validate + unit + e2e
just test unit        # Cargo workspace unit tests only
just test e2e         # E2E integration tests (starts/stops e2e infra)
just test-skip-infra e2e  # E2E against already-running infra
just test-baselines   # Create visual baselines for browser e2e tests
just test-visual-fail-first  # Strict fail-first visual run (empty baseline compare)
just test-visual-create      # Create/update browser visual baselines
just test-visual-verify      # Strict visual verification against baselines
just test-visual-audit       # Manifest vs baseline inventory audit
```

Runtime environment configuration lives in `config/runtime/`. Profile files
provide env vars for each `<environment>/<mode>/<service>` combination.
`just local <service>` loads the `dev/local/<service>.env` profile automatically.

Before running `just fullstack` directly (without `just local`), create
`code/frontend/app/.env` from `code/frontend/app/.env.example`. The required
local variable is `APP_DATABASE_URL`.

## Infrastructure (Pulumi + Docker)

**PRINCIPLE: All infrastructure must be managed via Pulumi.** This includes:
- Development stack (7xxx ports)
- E2E test stack (9xxx ports)
- All services: Postgres, Kafka, Keycloak, user-api, workout-api, fullstack

Never use local workarounds (like running `dx serve` directly) for test infrastructure. All containers should be deployed consistently through Pulumi to ensure tests run against the same setup as production.

All infrastructure (Postgres, Kafka, Keycloak, services) is managed via Pulumi:

```bash
cd infra
npm install
pulumi up -s cqrs      # Dev stack (7xxx ports)
pulumi destroy -s cqrs
```

**Dev stack services (7xxx ports):**
- Keycloak: http://localhost:7051 (admin/admin)
- Kafka: localhost:9092 (KRaft mode, host listener)
- Kafka UI: http://localhost:7052
- Mailpit: http://localhost:7057 (SMTP on 7025)
- User-API: http://localhost:7061
- Workout-API: http://localhost:7062
- Exercise-API: http://localhost:7063
- Elasticsearch: http://localhost:7200
- Prometheus: http://localhost:7090
- Fullstack: http://localhost:7081 (run locally with `just local fullstack`)

### E2E Testing (Pulumi-Orchestrated)

E2E tests run in an isolated stack with 9xxx ports. `just test e2e` orchestrates:
1. Spinning up all containers (Postgres, Kafka, Keycloak, services)
2. Waiting for health checks to pass
3. Running the e2e suite through the Rust test runner with appropriate env vars
4. Tearing down containers

```bash
cd infra
npm run e2e            # Full cycle: up → down, primarily for infra smoke/debug
npm run e2e:up         # Spin up e2e stack
npm run e2e:down       # Tear down e2e stack

# Or use just (recommended)
just test e2e          # Starts infra, runs e2e tests, tears infra down
```

The E2E stack is driven by the normal Pulumi service modules with stack-specific ports from `infra/src/ports.ts`. The justfile/test-runner lifecycle owns E2E up/down; there is no separate `infra/src/e2e-tests.ts` or `runE2eTests` gate.

## Architecture

```
Client → BFF (JWT validation) → Kafka → Services (user-api, workout-api)
```

**Event Flow:**
- `keycloak-events` → user-api (user provisioning)
- `user.sync` → workout-api (user cache replication)

**Command Flow:**
- BFF receives commands → token exchange → outbox → Kafka
- `commands.<service>.v1` → service validates OBO token → `outcomes.<service>.v1`

**Offline-First CQRS (Frontend):**
1. Command created with unique `command_id`
2. Stored in local outbox (SQLite/IndexedDB)
3. Projected optimistically to local store
4. Background sync pushes to `POST /api/v1/sync`
5. SSE invalidation triggers pull (only when outbox empty)

## Workspace Structure

```
code/
├── bff/               # Backend-for-Frontend (public HTTP entrypoint)
├── user-api/          # User management service
├── workout-api/       # Workout management service
├── shared/
│   ├── contracts/     # Kafka & HTTP message types (source of truth)
│   ├── messaging/     # KafkaConsumer, KafkaProducer, DLQ, outbox
│   ├── db/            # Database pool utilities
│   ├── auth/          # Keycloak JWT validation, token exchange
│   └── observability/ # Tracing, metrics
├── frontend/
│   ├── app/           # Main Dioxus app (storage, sync, auth, screens)
│   ├── shared-ui/     # Stateless Dioxus components (NO networking)
│   ├── gallery/       # Component preview app
│   ├── workout-shared/ # Workout domain types, DTOs
│   └── bff-workspace/ # BFF client workspace logic
└── tests/e2e/         # End-to-end integration tests

infra/                 # Pulumi IaC (Docker containers)
docs/                  # Unified documentation root
  current/             # Current baseline (spec + adr)
  next/                # Future proposals
  process/             # Governance, planning, references, audits
```

## Domain-Driven Design

### Domain Layer Principles
- **NO async, NO IO, NO infrastructure dependencies**
- Entities encapsulate state transitions and business rules
- Adapters orchestrate IO and call domain methods

### User Entity Lifecycle (user-api)
```rust
User::from_keycloak(&KeycloakUser) -> Result<Self, UserError>
user.verify_email() -> Result<(), UserError>
user.delete() -> Result<(), UserError>
user.apply_keycloak_update(&KeycloakUser) -> Result<(), UserError>
```

States: `Created → EmailVerified → Active → Deleted`

### Dependency Rules
| Layer | May Depend On | Must NOT Depend On |
|-------|---------------|-------------------|
| `domain/` | `ports::outbound` types only | `adapters/`, `sqlx`, `axum`, `messaging` |
| `adapters/` | `domain/`, `ports/`, infra crates | - |

## Kafka Client Libraries

| Library | Used For | Why |
|---------|----------|-----|
| **rdkafka** | Consuming | Consumer groups, offset management |
| **rskafka** | Producing | Partition-level control, murmur2 hash |

**macOS setup:** `brew install librdkafka`

## Consumer Architecture

```
1. Poll message
2. BEGIN transaction
3. Atomic dedup: INSERT ON CONFLICT DO NOTHING (rows=0? skip)
4. Parse payload (malformed → DLQ)
5. Process business logic (domain methods)
6. Write events to outbox
7. COMMIT
8. Ack Kafka offset
```

**Tombstones:** Delete events use `payload: serde_json::Value::Null` in outbox. The outbox loop sends a true Kafka tombstone (`value: None`).

## Frontend Platform Targets

The app crate uses feature flags for platform-specific compilation:
- `server` - SSR + server functions
- `web` - Browser/WASM
- `desktop` - Native desktop
- `mobile` - Android/iOS

### Platform Storage
| Platform | Backend | Location |
|----------|---------|----------|
| Desktop | SQLite | `~/Library/Application Support/com.repforge.app/app.db` |
| Mobile | SQLite | App sandbox |
| Web | IndexedDB | Browser storage |

All platforms store: `todos`, `outbox`, `dead_letter` tables/stores. Conditional compilation (`#[cfg(target_arch = "wasm32")]`) selects the backend.

### Cross-Platform Verification

**CRITICAL:** The `app` crate has platform-specific types that differ between native and web:

| Type | Native | Web |
|------|--------|-----|
| `Database` | `struct Database` with `#[derive(Clone)]` | `type Database = Rexie` (no Clone) |

`cargo test` and `cargo check -p app` only verify the **native** target. Code that compiles on native may fail on wasm32.

**After modifying `frontend/app/src/`**, always run:
```bash
just check-web    # Verify wasm32 builds
# or
just check        # Check all targets (includes wasm32)
```

Common cross-platform pitfalls:
- Assuming `Database` implements `Clone` (it doesn't on web)
- Using `std::fs` or `std::time::Instant` (not available on wasm32)
- Platform-specific imports without `#[cfg()]` guards

### Required Providers (AppRoot)

The `AppRoot` component in `app/src/app.rs` MUST provide these contexts:

| Provider | Purpose | Symptom if Missing |
|----------|---------|-------------------|
| `LanguageContext` | i18n translations | Raw keys shown: "nav.home", "profile.title" |
| `ThemeProvider` | Theme CSS variables | White background, invisible text, no colors |

```rust
// In AppRoot:
use_context_provider(|| LanguageContext::new(Language::En));

rsx! {
    ThemeProvider {
        // ... app content
    }
}
```

Anti-regression tests in `app/src/app.rs` verify these imports exist at compile time.

### Authentication
- **Web:** OAuth redirect flow with PKCE
- **Native (mobile):** Device Authorization Grant with magic link email
- **SSE:** Ticket-based for browser (EventSource can't set headers), direct JWT for native

## i18n Enforcement (MANDATORY)

**CRITICAL: ALL user-facing strings MUST use the i18n system via fluent-static.**

### Rules

1. ❌ **NEVER** use string literals for user-facing text
2. ✅ **ALWAYS** use `lang.messages().key_name()` for all text shown to users
3. ✅ Error messages MUST come from FTL files
4. ✅ Labels, titles, buttons MUST come from FTL files
5. ✅ Placeholders, hints, tooltips MUST come from FTL files

### Allowed String Literals (ONLY These)

- ✅ Element IDs: `id: "submit-button"`
- ✅ React-style keys: `key: "{index}"`
- ✅ URLs: `href: "https://..."`
- ✅ CSS classes: `class: "container"`
- ✅ Test data (in `*_test.rs` or `fixtures.rs` files only)

### Validation Before Committing

```bash
# Check for hardcoded strings (CI will run this)
just check-i18n

# Validate FTL keys exist
cargo test -p gallery -- i18n_validation
cargo test -p shared-ui -- i18n
```

### CI Enforcement

CI will **FAIL** the build if:
- Hardcoded user-facing strings detected in UI components
- FTL keys missing in `en.ftl` or `es.ftl`
- i18n validation tests fail

### Adding New Translations

1. Add key to `code/frontend/shared-ui/locales/en.ftl`:
   ```ftl
   validation-name-too-short = Name must be at least 3 characters
   ```

2. Add Spanish translation to `es.ftl`:
   ```ftl
   validation-name-too-short = El nombre debe tener al menos 3 caracteres
   ```

3. Use in code:
   ```rust
   let msg = lang
       .map(|l| l.messages().validation_name_too_short().to_string())
       .unwrap_or_default();
   ```

4. Run checks:
   ```bash
   cargo check -p shared-ui  # Ensures fluent-static sees new key
   just check-i18n           # Ensures no hardcoded strings
   ```

### How It Works

The `just check-i18n` command runs a Rust test (`test_no_hardcoded_user_facing_strings` in `code/frontend/gallery/src/tests/conventions.rs`) that:
1. Scans all UI source files (components, forms, screens)
2. Extracts string literals from each line
3. Checks for forbidden patterns: "must be", "required", "invalid", "Please ", "Cannot "
4. Reports violations with file paths and line numbers
5. Fails the build if any violations are found

This is a **cargo test**, not a bash script, making it:
- ✅ Portable (works on Windows without bash)
- ✅ Maintainable (Rust is easier to refactor than bash)
- ✅ Integrated with CI (runs with `cargo test`)

### See Also

- `.Codex/agents/dioxus-gallery.md` - Gallery-specific i18n requirements
- `i18n_enforcement_plan.md` - Detailed enforcement strategy
- `code/frontend/gallery/src/tests/conventions.rs` - i18n enforcement test implementation

## Gallery Conventions

**MANDATORY: Use dioxus-gallery agent for all gallery work**

When creating, modifying, or debugging UI components in the gallery (`code/frontend/gallery/`), you MUST use the dioxus-gallery agent defined at `.Codex/agents/dioxus-gallery.md`. This agent enforces:
- Individual underscore-prefixed style properties (NOT format! macros or style strings)
- Functional IDs for Playwright testing
- fluent-static i18n compliance
- Known Dioxus workarounds (e.g., width on input elements)
- Gallery-specific conventions (pancake stack layout, styles module usage)

Do not implement gallery code directly - invoke the dioxus-gallery agent using the Task tool.

Screens vs Workflows:
- **Screens:** Source of truth for UI structure (`Props` in / events out)
- **Workflows:** Compose screens with state, handle intents, own side effects

Screen previews instantiate real components with fixtures. Workflows render real screens driven by workflow state.

### MANDATORY: Gallery Code Must Use dioxus-gallery Agent

**BLOCKING REQUIREMENT:** ALL work on the UI Gallery (`code/frontend/gallery/`) MUST be done exclusively through the `dioxus-gallery` specialized agent. This agent enforces critical gallery conventions that are easily violated otherwise.

**When to use the dioxus-gallery agent:**
- Creating new gallery showcase pages or components
- Modifying existing gallery code
- Fixing gallery styling issues
- Debugging Playwright tests for gallery pages
- Resolving i18n/translation issues in gallery
- Reviewing gallery code

**How to invoke:**
```bash
# Use the Task tool with subagent_type="dioxus-gallery"
# Example: "Create a showcase for the Button component"
# Example: "Fix the broken styling on the Foundations page"
# Example: "Debug why the gallery test can't find #submit-btn"
```

**Why this is mandatory:**
The gallery has strict requirements that are NOT enforced by standard Rust compilation:
1. **NEVER use inline styles** - ALWAYS use `crate::styles::` module functions
2. **ALL interactive elements MUST have IDs** - Required for Playwright tests
3. **ALL user-visible strings MUST use i18n** - No hardcoded English text
4. **Specific patterns** - Pancake stack layout, fluent-static conventions, etc.

Violating these conventions causes:
- Failed Playwright tests (missing IDs)
- Broken i18n (hardcoded strings only work in English)
- Styling bugs (inline styles not using theme tokens)
- Code review rejections

**NEVER modify `code/frontend/gallery/` directly.** Always delegate to the dioxus-gallery agent first, even for "simple" changes. The agent will ensure compliance with all gallery conventions.

## Documentation & Process Governance

The repository uses a four-lane documentation model. Each lane has a distinct purpose and lifecycle:

| Lane | Path | Contains | Source of Truth For |
|------|------|----------|---------------------|
| Specs | `docs/current/spec/` | Business-functional truth | What the product does |
| ADRs | `docs/current/adr/` | Technical architecture decisions | How the system is built |
| Proposals | `docs/next/` | Future architecture proposals | What we plan to change |
| Process | `docs/process/` | Governance, planning, references, audits, checklists | How we work |

### Lifecycle Rules

1. Large changes begin as a **proposal** in `docs/next/` (status: Proposed -> Accepted).
2. Accepted proposals produce an **execution plan** in `docs/process/planning/`.
3. Completed plans promote durable outcomes into `docs/current/spec/` or `docs/current/adr/`.
4. Finished plans are **archived** to `docs/process/planning/archive/` or deleted.
5. Planning docs must never become the de facto source of truth — the code and `docs/current/` are.

### Process Sub-Lanes

- `docs/process/governance/` - Stable process rules (naming, doc lifecycle, spec/ADR boundaries)
- `docs/process/planning/` - Active execution plans (archived plans go in `planning/archive/`)
- `docs/process/reference/` - Supporting inventories and references
- `docs/process/checklists/` - Operational procedures
- `docs/process/audits/` - Review outputs and migration evidence

### When Creating or Changing Documents

- **New feature or architecture change?** Start with a proposal in `docs/next/`.
- **Executing an accepted proposal?** Create a plan in `docs/process/planning/` with exit gates.
- **Finished a plan?** Promote outcomes to `docs/current/`, archive or delete the plan.
- **Superseding an old plan?** Mark it superseded with a pointer to the replacement, then archive it.
- **Never** create accepted current-state ADRs outside `docs/current/adr/`.
- **Never** let planning documents become the canonical source of truth.

See `docs/process/governance/documentation-lifecycle.process.md` and `docs/process/governance/planning-lifecycle.process.md` for full rules.

## Specifications

Key business specs:
- `docs/current/spec/06-domain-model.spec.md` - Domain entities and lifecycles
- `docs/current/spec/08-offline-sync.spec.md` - Offline-first sync behavior
- `docs/current/spec/09-authentication-and-authorization.spec.md` - Auth/permission behavior
- `docs/current/spec/17-design-flow.spec.md` - UI/UX design system
- `docs/current/spec/17-A through 17-F` - Workflow specifications (core, planning, coaching, etc.)
- `docs/current/spec/18-ui-state-and-flows.spec.md` - Frontend state management

Key technical ADRs:
- `docs/current/adr/cqrs-pattern3-runtime-architecture.adr.md`
- `docs/current/adr/messaging-and-kafka-architecture.adr.md`
- `docs/current/adr/event-consumer-runtime-architecture.adr.md`
- `docs/current/adr/sse-realtime-architecture.adr.md`
- `docs/current/adr/testing-and-verification-architecture.adr.md`

Always consult the relevant spec + ADR before architectural changes.
