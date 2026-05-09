# Wiki Index

Project: Software Project Management Framework
Stage: D1-D11 Completed; skill projection template-engine follow-on completed
Updated: 2026-05-09

## Specs

- [Documentation Model](specs/documentation-model.spec.md) — Active — Three-layer architecture, operations, document types, binary distribution, composable init templates, and `llm-wiki search` scale strategy
- [Wiki Init Skill](specs/wiki-init-skill.spec.md) — Active — `wiki-init` wrapper over `llm-wiki init --non-interactive` with blueprint and pack selection
- [Wiki Query Skill](specs/wiki-query-skill.spec.md) — Active — `wiki-query` skill for querying the wiki with citations and save-back
- [Wiki Ingest Skill](specs/wiki-ingest-skill.spec.md) — Active — `wiki-ingest` skill for processing raw sources into wiki pages with 3-phase pipeline
- [Wiki Lint Skill](specs/wiki-lint-skill.spec.md) — Active — `wiki-lint` skill for scanning and fixing wiki consistency issues
- [Wiki Research Skill](specs/wiki-research-skill.spec.md) — Active — Guided research intake skill that writes bundles under `raw/research/` with manifest and summary

## Decisions

- [Three-Layer Architecture](decisions/three-layer-architecture.decision.md) — Accepted — raw/ + wiki/ + runtime agent schema, replacing legacy three-lane model
- [Agent Owns Wiki](decisions/agent-owns-wiki.decision.md) — Accepted — Agent has full control of wiki/, humans curate raw/
- [Typed Documents](decisions/typed-documents.decision.md) — Accepted — Nine document types with distinct truth relationships
- [Knowledge Research Intake](decisions/knowledge-research-intake.decision.md) — Accepted — `wiki-research` is the guided intake surface; no separate intake command
- [LLM Wiki Binary Distribution](decisions/llm-wiki-binary-distribution.decision.md) — Accepted — Single Rust binary owns global skill installation, project scaffolding, and skill projection
- [Managed Binary Runtime Install](decisions/binary-path-bootstrap.decision.md) — Accepted — `llm-wiki install` will manage a runtime binary under `~/.llm_wiki/` so installed skills do not require `PATH`
- [Search Backend Selection](decisions/search-backend-selection.decision.md) — Accepted — qmd-rs is the D9 backend for `llm-wiki search` and `search-all`, with direct SQLite FTS5 as fallback
- [Composable Project Init](decisions/composable-project-init.decision.md) — Accepted — `llm-wiki init` becomes a blueprint + pack composition over a compile-time template engine; per-project `.llm_wiki/init.toml` records the choices
- [Skill Projection Template Engine](decisions/skill-projection-template-engine.decision.md) — Accepted — Claude/Codex skill markdown and Codex runtime config render through shared Askama templates

## Roadmaps

- [Framework V1](roadmaps/framework-v1.roadmap.md) — Active — Framework deliverables through D11 plus D4-D7 proof gates are completed; future work is post-V1 backlog

## References

- [LLM Wiki Pattern](references/llm-wiki-pattern.reference.md) — Sourced — Karpathy's pattern, v2 extensions, production lessons, wiki vs RAG
- [QMD Search Engine](references/qmd-search-engine.reference.md) — Sourced — On-device hybrid search for markdown, MCP integration, solves scale ceiling
- [qmd-rs Rust Search Crate](references/qmd-rs-search-crate.reference.md) — Sourced — Rust `qmd` crate selected as the backend for `llm-wiki` search commands
- [NiharShrotri/llm-wiki](references/niharshrotri-llm-wiki.reference.md) — Sourced — Full implementation with 3-pass ingest, QMD, CLI, web UI, auto-lint
- [LLM Wiki Ecosystem Survey](references/llm-wiki-ecosystem.reference.md) — Sourced — 30+ implementations organized by delivery model and architectural innovation
- [Three-Phase Ingest Pipeline](references/three-phase-ingest-pipeline.reference.md) — Sourced — Web-sourced explanation of extraction, page drafting, and bookkeeping as separate ingest phases
- [Askama Template Engine for D10 Composable Init](references/askama-template-engine.reference.md) — Sourced — Askama 0.16 implementation guidance for D10 templates, fragments, escaping, whitespace, and schema-crate caveats

## Proposals

- [LLM Wiki Framework Binary](proposals/llm-wiki-binary.proposal.md) — Accepted — Single Rust binary owns global skill installation, project scaffolding, and skill projection; promoted to decision and roadmap D8
- [Project Registry and Search Artifacts](proposals/project-registry-search-artifacts.proposal.md) — Accepted — Promoted to D9 implementation plan for registry, indexing, project-local search, and explicit `search-all`
- [Search Backend Selection](proposals/search-backend-selection.proposal.md) — Accepted — Promoted to backend decision; qmd-rs selected for D9 with adapter-owned metadata and model/cache handling
- [Managed Binary Install and PATH Guidance](proposals/binary-path-bootstrap.proposal.md) — Accepted — Promoted to D8.1 decision and plan; make installed skills call a managed binary path while PATH remains convenience guidance
- [Composable Project Init: Blueprints and Packs](proposals/blueprint-pack-init.proposal.md) — Accepted — Promoted to D10 decision and plan; replace the static project guidelines template with a blueprint + pack composition model and add a per-project `.llm_wiki/` folder with an `init.toml` manifest
- [Skill Projection on the Composable-Init Template Engine](proposals/skills-template-engine.proposal.md) — Accepted — Promoted to decision and completed plan; skill projection now uses the shared Askama template engine

## Plans

- [Knowledge Research Intake Implementation](plans/knowledge-research-intake.plan.md) — Completed — Executed the `knowledge-research` upgrade into a guided intake workflow with research bundles
- [LLM Wiki Binary Implementation](plans/llm-wiki-binary.plan.md) — Completed — Implemented D8 Rust binary with staged commits, manifest install, deterministic init, projection snapshots, fixtures, and release config
- [LLM Wiki Product Layout Addendum](plans/llm-wiki-product-layout-addendum.plan.md) — Completed — Moved `llm-wiki` to the root product crate layout and embedded assets under `assets/`
- [Managed Binary Runtime Install](plans/binary-path-bootstrap.plan.md) — Completed — D8.1 tactical execution for managed runtime home, manifest v2, outside-PATH install proof, and `wiki-init` rename
- [qmd-rs Search Backend](plans/qmd-rs-search-backend.plan.md) — Completed — D9 backend slice landed qmd-rs adapter, query sanitization, metadata, snippets, doctor checks, eval replay, and release findings
- [Project Registry and Search Artifacts](plans/project-registry-search-artifacts.plan.md) — Completed — Implemented D9 registry/search commands and default-on qmd-rs release behavior
- [Project and Skill Rename](plans/project-and-skill-rename.plan.md) — Completed — D11 executed: package renamed to `llm-wiki-rs`, `knowledge*` skill surface renamed to `wiki-*`, the Codex dispatcher renamed from `knowledge` to `wiki`, runtime mirrors regenerated, and the one-off legacy home-level symlink migration now points at frozen `.claude.legacy/` / `.codex.legacy/` trees
- [Composable Project Init](plans/composable-project-init.plan.md) — Completed — D10 execution adopted Askama, migrated init templates, shipped blueprints + packs, two-step interactive flow, AGENTS/CLAUDE schema handling, and `.llm_wiki/init.toml`
- [Skill Projection Template Engine](plans/skill-projection-template-engine.plan.md) — Completed — Migrated Claude/Codex skill markdown and Codex runtime config rendering onto `templates/skills/` Askama templates with byte-stable real-skill snapshots

## Experiments

(none yet)

## Evals

- [Search Backend Selection Eval](evals/search-backend-selection.eval.md) — Accepted — Fixed query set, qmd-rs/Tobi QMD/SQLite BM25 baselines, and recommendation for qmd-rs as D9 backend
- [V1 Proof Run](evals/v1-proof-run.eval.md) — Accepted — Evidence for D4-D7: durable query knowledge, temp project spawning, 50-page index check, and two-blueprint self-replication proof

## Checklists

- [V1 Fixture Smoke](checklists/v1-fixture-smoke.checklist.md) — Completed — Temp fixture smoke ingested a raw source, answered from fixture spec/decision, and linted bookkeeping with no unresolved issues

## Archive

- [Knowledge Command Namespace](archive/knowledge-command-namespace.decision.md) — Superseded — Pre-D11 decision that `$knowledge` was the Codex command surface; superseded by the D11 rename to `$wiki`
- [Knowledge Research Intake Proposal](archive/knowledge-intake-command.proposal.md) — Archived — Superseded by the accepted Knowledge Research Intake decision
- [Project-Local Codex Skills](archive/project-local-codex-skills.decision.md) — Superseded — Archived predecessor to the binary distribution model
- [Framework Path Resolution](archive/framework-path-resolution.decision.md) — Superseded — Archived predecessor to the binary distribution model
- [Single Source Skills](archive/single-source-skills.decision.md) — Superseded — Archived predecessor to the binary distribution model
- [Single Source Skills Plan](archive/single-source-skills.plan.md) — Superseded — Archived predecessor plan replaced by D8 binary implementation
