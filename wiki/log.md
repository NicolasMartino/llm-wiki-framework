# Wiki Log

## [2026-04-23] ingest | Karpathy LLM Wiki research

Compiled raw/research/llm-wiki-pattern-research.md into wiki reference page.
Source covers: original Karpathy gist, v2 extensions, Fulkerson production
deployment, wiki vs RAG tradeoffs.
Pages created: wiki/references/llm-wiki-pattern.reference.md

## [2026-04-23] ingest | Legacy project guidelines

Compiled raw/legacy/legacy-project-guidelines.md into framework decisions
and spec. Extracted key architectural choices as decision records. Merged
typed document system with LLM Wiki pattern into documentation model spec.
Pages created: wiki/decisions/three-layer-architecture.decision.md, wiki/decisions/agent-owns-wiki.decision.md, wiki/decisions/typed-documents.decision.md, wiki/specs/documentation-model.spec.md

## [2026-04-23] create | Framework V1 roadmap

Created roadmap for framework development with 7 deliverables: bootstrap,
ingest cycle, lint operation, query produces knowledge, spawn new project,
scale test, self-replicating framework. D1 (bootstrap) marked completed.
Pages created: wiki/roadmaps/framework-v1.roadmap.md

## [2026-04-23] create | Wiki index and log

Created wiki/index.md (master catalog) and wiki/log.md (this file).
Initial index catalogs 6 wiki pages across 4 document types.
Pages created: wiki/index.md, wiki/log.md

## [2026-04-23] ingest | QMD search engine and NiharShrotri/llm-wiki implementation

Ingested two new raw sources:
- raw/research/qmd-search-engine.md (Tobi Lutke's on-device markdown search)
- raw/research/niharshrotri-llm-wiki-implementation.md (full LLM Wiki implementation)

Key findings:
- QMD solves our scale ceiling (>100 pages) with local hybrid search via MCP
- 3-pass ingest pipeline (extraction → drafting → bookkeeping) improves quality
- Source audit pages provide provenance tracking
- Our typed-document approach (spec/decision/proposal) is differentiated from
  the entity/concept/synthesis model — both valid, ours better for software projects

Pages created: wiki/references/qmd-search-engine.reference.md, wiki/references/niharshrotri-llm-wiki.reference.md
Pages updated: wiki/specs/documentation-model.spec.md (navigation, limitations, sources), wiki/references/llm-wiki-pattern.reference.md (cross-references), wiki/index.md

## [2026-04-23] create | Init project skill and template

Created /init-project skill for spawning new projects from the framework.
Renamed project_guidelines.md to project_guidelines.template.md with
conditional section markers (ML_AI, QMD). Skill asks 6 questions to
determine project profile, generates tailored guidelines and CLAUDE.md,
scaffolds folder structure, optionally ingests initial sources. Supports
create and update modes.
Pages created: .claude/skills/init-project.md, wiki/specs/init-project-skill.spec.md
Files modified: project_guidelines.template.md (renamed, added conditional markers)
Pages updated: wiki/index.md

## [2026-04-23] ingest | LLM Wiki ecosystem survey

Ingested raw/research/llm-wiki-ecosystem-survey.md — a GitHub-wide survey
of 30+ implementations of Karpathy's LLM Wiki pattern, three weeks after
publication.

Key findings organized by architectural innovation, not by repo:
- Five delivery models: agent skill, CLI, desktop app, web app, Obsidian plugin
- Agent-skill form dominates (lowest friction, no infra needed)
- Notable innovations: two-phase compilation (order-independent ingest),
  L1/L2 cache architecture, parallel multi-agent research, automated feed
  monitoring, write-back from queries, multimodal ingest, anti-repetition
  memory, codebase-as-source, wiki-as-training-data
- Top adoption candidates for our framework: write-back from queries,
  codebase as source, L1/L2 cache formalization, two-phase compilation

Pages created: wiki/references/llm-wiki-ecosystem.reference.md
Pages updated: wiki/references/llm-wiki-pattern.reference.md (cross-ref), wiki/references/niharshrotri-llm-wiki.reference.md (cross-ref), wiki/index.md

## [2026-04-23] create | Knowledge query skill

Created /knowledge-query skill for querying the project wiki with citations
and optional save-back. Single-project scope. Reads index.md to orient,
identifies relevant pages, synthesizes answer with citations, flags gaps
and contradictions, offers to save durable answers as new wiki pages.
Optionally uses QMD if available. Symlinked to global skills.
Pages created: .claude/skills/knowledge-query/SKILL.md, wiki/specs/knowledge-query-skill.spec.md
Pages updated: wiki/index.md

## [2026-04-23] create | Knowledge ingest skill

Created /knowledge-ingest skill for processing raw sources into wiki pages.
Three-phase pipeline: extraction, page drafting, bookkeeping. Handles merge
vs create, contradiction detection, provenance tracking, auto-discovery of
unprocessed files. Supports markdown, text, PDF, images, URLs, transcripts,
code. Symlinked to global skills.
Pages created: .claude/skills/knowledge-ingest/SKILL.md, wiki/specs/knowledge-ingest-skill.spec.md
Pages updated: wiki/index.md
