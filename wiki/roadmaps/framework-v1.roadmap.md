# Framework V1 Roadmap

- Document Class: Roadmap
- Status: Active
- Date: 2026-04-23
- Category: Framework development
- Scope: Deliverables required to prove the project management framework works end-to-end and can spawn new self-managing projects.

## Objective

Prove that an LLM Wiki-based project management framework can:

1. manage its own development (self-referential dogfooding)
2. scale to real project complexity
3. spawn new projects where an agent is immediately productive
4. maintain knowledge integrity through ingest/query/lint cycles

## Sequencing Principles

1. Bootstrap first, refine later
2. Each deliverable must produce observable evidence
3. The framework manages its own deliverables (eat your own dogfood)
4. Portability is proven by spawning, not by theorizing

---

### D1 - Bootstrap

Status: Completed
Promise: The framework manages its own creation. Three-layer structure exists, first ingest is done, wiki is navigable.
Depends On: None
Execution Plan: Not needed (single-session bootstrap)

Included:
- project_guidelines.md written
- CLAUDE.md written
- raw/, wiki/ structure scaffolded
- Legacy guidelines and research moved to raw/
- First ingest: research and legacy compiled into wiki pages
- Decisions recorded for key architectural choices
- Spec written for the documentation model
- index.md and log.md created
- This roadmap created

Excluded:
- Lint operation
- Automated tooling
- Multi-project testing

Proof:
- wiki/index.md catalogs all pages
- wiki/log.md records the bootstrap
- Agent can orient by reading index.md alone

Promotion Target:
- wiki/specs/documentation-model.spec.md (done)

Unlocks:
- D2, D3

---

### D2 - Ingest Cycle

Status: Completed
Promise: A new raw source is ingested end-to-end: the agent reads it, creates/updates wiki pages, checks for contradictions, updates index and log. The workflow is repeatable.
Depends On: D1
Execution Plan: Not created yet

Included:
- Ingest a non-trivial new raw source (not the bootstrap material)
- Agent follows the ingest workflow from CLAUDE.md
- Contradictions with existing content are surfaced
- Index and log are updated correctly
- Cross-references between new and existing pages are created

Excluded:
- Automated ingest triggers
- Bulk ingest of multiple sources

Proof:
- New wiki pages exist with correct metadata and source citations
- wiki/index.md includes the new pages
- wiki/log.md records the ingest
- At least one cross-reference to an existing page was created or updated

Promotion Target:
- wiki/specs/documentation-model.spec.md (update ingest as proven)

Unlocks:
- D3, D5

---

### D3 - Lint Operation

Status: Completed
Promise: The agent runs a lint pass over the wiki and finds at least one real issue (contradiction, orphan, stale status, missing cross-reference). The issue is fixed and logged.
Depends On: D1
Execution Plan: Not needed (single-session lint pass)

Included:
- Full wiki scan
- Contradiction detection
- Orphan page detection
- Stale status detection
- Missing cross-reference detection
- Direct fix of issues found
- Log entry for each fix

Excluded:
- Automated lint scheduling
- Lint tooling or scripts

Proof:
- At least one real issue found and fixed
- wiki/log.md records the lint and fixes
- wiki/index.md is consistent after lint

Promotion Target:
- wiki/specs/documentation-model.spec.md (update lint as proven)

Unlocks:
- D5

---

### D4 - Query Produces Durable Knowledge

Status: Draft
Promise: A non-trivial question is asked, the agent answers from wiki content, and the answer is valuable enough to file as a new wiki page.
Depends On: D2
Execution Plan: Not created yet

Included:
- A real question that requires synthesizing multiple wiki pages
- Answer with citations to wiki pages
- New wiki page created from the answer
- Index and log updated

Excluded:
- Query tooling
- Search beyond index.md navigation

Proof:
- New wiki page exists from a query answer
- Page cites multiple existing wiki pages
- wiki/index.md includes the new page

Promotion Target:
- wiki/specs/documentation-model.spec.md (update query as proven)

Unlocks:
- D5

---

### D5 - Framework Spawns a New Project

Status: Draft
Promise: The `init-project` skill creates a new project end-to-end. A fresh agent in the new repo, with only the generated project_guidelines.md and CLAUDE.md, bootstraps a self-managing wiki and is immediately productive.
Depends On: D2, D3, D4
Execution Plan: Not created yet

Included:
- Run `init-project` on a real project
- Skill asks questions, generates tailored guidelines and CLAUDE.md
- Scaffolds raw/ + wiki/ with correct profile (ML_AI, QMD flags)
- Agent in the new project ingests first raw sources
- Agent answers a query about the new project using only wiki content

Excluded:
- Multi-agent coordination
- Scale testing (handled in D6)

Proof:
- New project has a navigable wiki after bootstrap
- Agent can answer a domain question from the wiki
- No human edited wiki/ directly

Promotion Target:
- wiki/specs/documentation-model.spec.md (update portability as proven)

Unlocks:
- D6, D7

---

### D6 - Scale Test

Status: Draft
Promise: The wiki handles 50+ pages across multiple document types without degrading agent navigation. Index remains usable.
Depends On: D5
Execution Plan: Not created yet

Included:
- A project with 50+ wiki pages
- Agent navigates via index.md to answer questions
- Measure: does the agent find the right pages on first try?
- Measure: does index.md fit in context?
- If index exceeds context: implement sub-indexes per type

Excluded:
- Hybrid search (BM25 + vector)
- Performance benchmarking

Proof:
- Agent answers 10 questions correctly using only index navigation
- Index token count measured and recorded
- If sub-indexes needed: implemented and working

Promotion Target:
- wiki/specs/documentation-model.spec.md (update scale characteristics)

Unlocks:
- D7

---

### D7 - Self-Replicating Framework

Status: Draft
Promise: The framework is packaged so that creating a new self-managing project is a single operation: point the agent at a domain, provide initial raw sources, and the agent produces a working wiki.
Depends On: D5, D6
Execution Plan: Not created yet

Included:
- Template CLAUDE.md and project_guidelines.md
- Documented bootstrap procedure (as a checklist)
- Agent can execute the bootstrap checklist without modification
- Tested on at least two distinct project domains

Excluded:
- GUI or CLI tooling
- Framework versioning or updates to spawned projects

Proof:
- Two distinct projects bootstrapped by agents using the framework
- Both projects have navigable wikis
- Neither required human editing of wiki/

Promotion Target:
- wiki/specs/documentation-model.spec.md (framework is proven self-replicating)

Unlocks:
- Future: multi-agent, automated lint, hybrid search, framework distribution
