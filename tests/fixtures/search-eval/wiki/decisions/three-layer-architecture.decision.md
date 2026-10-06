# Three-Layer Architecture

- Document Class: Decision
- Status: Accepted
- Date: 2026-04-23
- Category: Architecture
- Scope: Adopt raw/ + wiki/ + an agent schema file as the project structure, replacing the legacy three-lane docs/ model.
- Sources: raw/research/llm-wiki-pattern-research.md, raw/legacy/legacy-project-guidelines.md
- Amended By: wiki/decisions/composable-project-init.decision.md

## Choice

Use a three-layer architecture:

1. `raw/` - immutable source material (human-curated)
2. `wiki/` - compiled knowledge (agent-owned)
3. Agent schema file - conventions and workflows for the active agent runtime.
   Historically this was `CLAUDE.md`; D10 makes generated `AGENTS.md`
   canonical while preserving a small `CLAUDE.md` compatibility shim.

## Why

The legacy model used three documentation lanes (`docs/current/`,
`docs/next/`, `docs/process/`) with distributed READMEs for navigation.
This worked for human-authored documentation but was not optimized for
agent navigation.

The LLM Wiki pattern provides:

- a single entry point (`wiki/index.md`) instead of browsing multiple READMEs
- agent ownership of the knowledge layer (compile once, query many times)
- explicit operations (ingest, query, lint) instead of ad-hoc maintenance
- knowledge that compounds across sessions

## Alternatives Considered

1. **Keep the legacy three-lane model.** Rejected: navigation requires
   browsing multiple directories and READMEs. No formal ingest or lint.
   No agent ownership.

2. **Pure Karpathy model (flat wiki/).** Rejected: loses the typed document
   system (spec, decision, proposal, roadmap, plan) which provides valuable
   structural guarantees about what kind of truth each document represents.

3. **Hybrid: legacy lanes inside wiki/.** Rejected: adds a nesting layer
   that complicates agent navigation without adding value. Status metadata
   already separates validated truth from proposals.

## Consequences

- All documentation lives in `wiki/` under typed subdirectories
- The agent must update `wiki/index.md` on every mutation
- Folder READMEs are eliminated; `wiki/index.md` is the sole navigation file
- Document type (spec vs proposal) carries the truth-status distinction that
  the legacy model encoded in folder structure (current/ vs next/)

## Revisit When

- `wiki/index.md` exceeds ~50,000 tokens and a single-index approach
  becomes insufficient
- Multi-team usage requires access control that flat folders cannot provide
