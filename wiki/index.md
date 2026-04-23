# Wiki Index

Project: Software Project Management Framework
Stage: D1 Bootstrap (Completed), D2 Ingest Cycle (Completed)
Updated: 2026-04-23

## Specs

- [Documentation Model](specs/documentation-model.spec.md) — Active — Three-layer architecture, operations, document types, navigation, QMD scale strategy
- [Init Project Skill](specs/init-project-skill.spec.md) — Active — /init-project skill for creating/updating projects with the framework
- [Knowledge Query Skill](specs/knowledge-query-skill.spec.md) — Active — /knowledge-query skill for querying the wiki with citations and save-back
- [Knowledge Ingest Skill](specs/knowledge-ingest-skill.spec.md) — Active — /knowledge-ingest skill for processing raw sources into wiki pages with 3-phase pipeline

## Decisions

- [Three-Layer Architecture](decisions/three-layer-architecture.decision.md) — Accepted — raw/ + wiki/ + CLAUDE.md, replacing legacy three-lane model
- [Agent Owns Wiki](decisions/agent-owns-wiki.decision.md) — Accepted — Agent has full control of wiki/, humans curate raw/
- [Typed Documents](decisions/typed-documents.decision.md) — Accepted — Nine document types with distinct truth relationships

## Roadmaps

- [Framework V1](roadmaps/framework-v1.roadmap.md) — Active — Seven deliverables: bootstrap through self-replicating framework

## References

- [LLM Wiki Pattern](references/llm-wiki-pattern.reference.md) — Sourced — Karpathy's pattern, v2 extensions, production lessons, wiki vs RAG
- [QMD Search Engine](references/qmd-search-engine.reference.md) — Sourced — On-device hybrid search for markdown, MCP integration, solves scale ceiling
- [NiharShrotri/llm-wiki](references/niharshrotri-llm-wiki.reference.md) — Sourced — Full implementation with 3-pass ingest, QMD, CLI, web UI, auto-lint

## Proposals

(none yet)

## Plans

(none yet)

## Experiments

(none yet)

## Evals

(none yet)

## Checklists

(none yet)
