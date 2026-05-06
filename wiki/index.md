# Wiki Index

Project: Software Project Management Framework
Stage: D1 Bootstrap (Completed), D2 Ingest Cycle (Completed), D3 Lint Operation (Completed), D8 Distribution Tooling (Completed)
Updated: 2026-05-06

## Specs

- [Documentation Model](specs/documentation-model.spec.md) — Active — Three-layer architecture, operations, document types, binary distribution, navigation, QMD scale strategy
- [Init Project Skill](specs/init-project-skill.spec.md) — Active — `init-project` wrapper over `llm-wiki init --non-interactive`
- [Knowledge Query Skill](specs/knowledge-query-skill.spec.md) — Active — `knowledge-query` skill for querying the wiki with citations and save-back
- [Knowledge Ingest Skill](specs/knowledge-ingest-skill.spec.md) — Active — `knowledge-ingest` skill for processing raw sources into wiki pages with 3-phase pipeline
- [Knowledge Lint Skill](specs/knowledge-lint-skill.spec.md) — Active — `knowledge-lint` skill for scanning and fixing wiki consistency issues
- [Knowledge Research Skill](specs/knowledge-research-skill.spec.md) — Active — Guided research intake skill that writes bundles under `raw/research/` with manifest and summary

## Decisions

- [Three-Layer Architecture](decisions/three-layer-architecture.decision.md) — Accepted — raw/ + wiki/ + CLAUDE.md, replacing legacy three-lane model
- [Agent Owns Wiki](decisions/agent-owns-wiki.decision.md) — Accepted — Agent has full control of wiki/, humans curate raw/
- [Typed Documents](decisions/typed-documents.decision.md) — Accepted — Nine document types with distinct truth relationships
- [Knowledge Command Namespace](decisions/knowledge-command-namespace.decision.md) — Accepted — `$knowledge` is the shared Codex command surface for init/query/ingest/research/lint
- [Knowledge Research Intake](decisions/knowledge-research-intake.decision.md) — Accepted — `knowledge-research` is the guided intake surface; no separate `knowledge-intake` command
- [LLM Wiki Binary Distribution](decisions/llm-wiki-binary-distribution.decision.md) — Accepted — Single Rust binary owns global skill installation, project scaffolding, and skill projection

## Roadmaps

- [Framework V1](roadmaps/framework-v1.roadmap.md) — Active — Eight deliverables: bootstrap through self-replicating framework, plus D8 distribution tooling

## References

- [LLM Wiki Pattern](references/llm-wiki-pattern.reference.md) — Sourced — Karpathy's pattern, v2 extensions, production lessons, wiki vs RAG
- [QMD Search Engine](references/qmd-search-engine.reference.md) — Sourced — On-device hybrid search for markdown, MCP integration, solves scale ceiling
- [NiharShrotri/llm-wiki](references/niharshrotri-llm-wiki.reference.md) — Sourced — Full implementation with 3-pass ingest, QMD, CLI, web UI, auto-lint
- [LLM Wiki Ecosystem Survey](references/llm-wiki-ecosystem.reference.md) — Sourced — 30+ implementations organized by delivery model and architectural innovation
- [Three-Phase Ingest Pipeline](references/three-phase-ingest-pipeline.reference.md) — Sourced — Web-sourced explanation of extraction, page drafting, and bookkeeping as separate ingest phases

## Proposals

- [LLM Wiki Framework Binary](proposals/llm-wiki-binary.proposal.md) — Accepted — Single Rust binary owns global skill installation, project scaffolding, and skill projection; promoted to decision and roadmap D8
- [Project Registry And Search Artifacts](proposals/project-registry-search-artifacts.proposal.md) — Proposed — Post-D8 project registration, centralized per-project QMD search artifacts, and explicit `search-all` cross-project search

## Plans

- [Knowledge Research Intake Implementation](plans/knowledge-research-intake.plan.md) — Completed — Executed the `knowledge-research` upgrade into a guided intake workflow with research bundles
- [LLM Wiki Binary Implementation](plans/llm-wiki-binary.plan.md) — Completed — Implemented D8 Rust binary with staged commits, manifest install, deterministic init, projection snapshots, fixtures, and release config
- [LLM Wiki Product Layout Addendum](plans/llm-wiki-product-layout-addendum.plan.md) — Active — Post-D8 layout correction to make `llm-wiki` the root product crate and move embedded assets under `assets/`

## Experiments

(none yet)

## Evals

(none yet)

## Checklists

- [V1 Fixture Smoke](checklists/v1-fixture-smoke.checklist.md) — Active — Agent-driven ingest/query/lint smoke procedure for the committed v1 fixture

## Archive

- [Knowledge Research Intake Proposal](archive/knowledge-intake-command.proposal.md) — Archived — Superseded by the accepted Knowledge Research Intake decision
- [Project-Local Codex Skills](archive/project-local-codex-skills.decision.md) — Superseded — Archived predecessor to the binary distribution model
- [Framework Path Resolution](archive/framework-path-resolution.decision.md) — Superseded — Archived predecessor to the binary distribution model
- [Single Source Skills](archive/single-source-skills.decision.md) — Superseded — Archived predecessor to the binary distribution model
- [Single Source Skills Plan](archive/single-source-skills.plan.md) — Superseded — Archived predecessor plan replaced by D8 binary implementation
