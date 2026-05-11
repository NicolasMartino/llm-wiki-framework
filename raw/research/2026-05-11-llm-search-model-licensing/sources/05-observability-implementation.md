# Source 05: Observability Implementation Evidence

- Source paths:
  - `src/cli.rs`
  - `src/main.rs`
  - `src/search/commands.rs`
  - `tests/search_commands.rs`
  - `wiki/checklists/observability-contract.checklist.md`
- Retrieved: 2026-05-11
- Source type: Local code and wiki observation after rebase onto master

## Relevant Facts

- `src/cli.rs` defines the root global `-v` / `--verbose` flag.
- `src/cli.rs` defines `CliContext { verbose: bool }` and a `diagnostic(...)`
  helper that emits only when verbose mode is enabled.
- `src/main.rs` calls `init_tracing(cli.verbose)`, creates
  `CliContext::new(cli.verbose)`, and passes that context to command handlers.
- `src/search/commands.rs` emits `search` diagnostics for raw query,
  sanitized FTS query, class/status filters, limit, registry path, project
  selection, selected project, wiki root, qmd-rs store path, backend,
  index-state, result count, and no-result explanation.
- `src/search/commands.rs` emits `search-all` diagnostics for raw query,
  sanitized FTS query, filters, limit, registry path, selected project list,
  per-project wiki root/store path/index state/result count/no-result
  explanation, and fused result count.
- `tests/search_commands.rs` covers verbose search/search-all diagnostics,
  global verbose flag placement after subcommands, JSON stdout parseability,
  zero-result reasons, filter-excluded results, and limit-zero results.
- `wiki/checklists/observability-contract.checklist.md` makes this diagnostic
  boundary a standing review gate for future command work.

## Implication For llm-wiki

The Stage 0 observability substrate is present. Semantic/hybrid search should
extend the same `CliContext` and tracing path with selected mode, readiness,
threshold, expansion, semantic, fusion, rerank, fallback, and zero-result
events.
