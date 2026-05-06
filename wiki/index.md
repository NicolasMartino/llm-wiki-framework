# Wiki Index

Project: Software Project Management Framework
Stage: D1 Bootstrap (Completed), D2 Ingest Cycle (Completed), D3 Lint Operation (Completed)
Updated: 2026-05-02

## Specs

- [Documentation Model](specs/documentation-model.spec.md) — Active — Three-layer architecture, operations, document types, navigation, QMD scale strategy
- [Init Project Skill](specs/init-project-skill.spec.md) — Active — `init-project` skill for creating/updating projects with the framework
- [Knowledge Query Skill](specs/knowledge-query-skill.spec.md) — Active — `knowledge-query` skill for querying the wiki with citations and save-back
- [Knowledge Ingest Skill](specs/knowledge-ingest-skill.spec.md) — Active — `knowledge-ingest` skill for processing raw sources into wiki pages with 3-phase pipeline
- [Knowledge Lint Skill](specs/knowledge-lint-skill.spec.md) — Active — `knowledge-lint` skill for scanning and fixing wiki consistency issues
- [Knowledge Research Skill](specs/knowledge-research-skill.spec.md) — Active — Guided research intake skill that writes bundles under `raw/research/` with manifest and summary

## Decisions

- [Three-Layer Architecture](decisions/three-layer-architecture.decision.md) — Accepted — raw/ + wiki/ + CLAUDE.md, replacing legacy three-lane model
- [Agent Owns Wiki](decisions/agent-owns-wiki.decision.md) — Accepted — Agent has full control of wiki/, humans curate raw/
- [Typed Documents](decisions/typed-documents.decision.md) — Accepted — Nine document types with distinct truth relationships
- [Knowledge Command Namespace](decisions/knowledge-command-namespace.decision.md) — Accepted — `$knowledge` is the shared Codex command surface for init/query/ingest/research/lint
- [Project-Local Codex Skills](decisions/project-local-codex-skills.decision.md) — Accepted — Codex translations live in `.codex/skills/` and are exposed globally through `~/.codex/skills/` symlinks

## Roadmaps

- [Framework V1](roadmaps/framework-v1.roadmap.md) — Active — Seven deliverables: bootstrap through self-replicating framework

## References

- [LLM Wiki Pattern](references/llm-wiki-pattern.reference.md) — Sourced — Karpathy's pattern, v2 extensions, production lessons, wiki vs RAG
- [QMD Search Engine](references/qmd-search-engine.reference.md) — Sourced — On-device hybrid search for markdown, MCP integration, solves scale ceiling
- [NiharShrotri/llm-wiki](references/niharshrotri-llm-wiki.reference.md) — Sourced — Full implementation with 3-pass ingest, QMD, CLI, web UI, auto-lint
- [LLM Wiki Ecosystem Survey](references/llm-wiki-ecosystem.reference.md) — Sourced — 30+ implementations organized by delivery model and architectural innovation
- [Three-Phase Ingest Pipeline](references/three-phase-ingest-pipeline.reference.md) — Sourced — Web-sourced explanation of extraction, page drafting, and bookkeeping as separate ingest phases

## Proposals

- [Knowledge Research Intake](proposals/knowledge-intake-command.proposal.md) — Accepted — `knowledge-research` is the guided intake surface and saves research bundles under `raw/research/`

## Plans

- [Knowledge Research Intake Implementation](plans/knowledge-research-intake.plan.md) — Completed — Executed the `knowledge-research` upgrade into a guided intake workflow with research bundles

## Experiments

(none yet)

## Evals

(none yet)

## Checklists

(none yet)
