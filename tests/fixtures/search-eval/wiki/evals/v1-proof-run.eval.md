# V1 Proof Run

- Document Class: Eval
- Status: Accepted
- Date: 2026-05-09
- Category: Framework proof
- Scope: Evidence for the D4-D7 framework proof milestones after D11 and the skill-projection template-engine follow-on.
- Sources: wiki/roadmaps/framework-v1.roadmap.md, wiki/decisions/skill-projection-template-engine.decision.md, wiki/plans/skill-projection-template-engine.plan.md, wiki/checklists/v1-fixture-smoke.checklist.md, /private/tmp/llm-wiki-proof-web-product, /private/tmp/llm-wiki-proof-ml-research, /private/tmp/llm-wiki-v1-smoke-20260509
- Related: wiki/roadmaps/framework-v1.roadmap.md, wiki/specs/documentation-model.spec.md

## Result

Accepted.

The framework has now produced durable query knowledge, spawned new projects,
passed a scale-oriented index check, and bootstrapped two distinct project
shapes through `llm-wiki init`.

## D4 Evidence

The query "what's left to do?" synthesized roadmap, proposal, and checklist
state into implementation work. The resulting durable knowledge is recorded in:

- `wiki/decisions/skill-projection-template-engine.decision.md`
- `wiki/plans/skill-projection-template-engine.plan.md`
- `wiki/checklists/v1-fixture-smoke.checklist.md`
- this eval

## D5 Evidence

`llm-wiki init` spawned a real temp project at
`/private/tmp/llm-wiki-proof-web-product` with:

- canonical `AGENTS.md`
- `CLAUDE.md` compatibility shim
- `project_guidelines.md`
- `.llm_wiki/init.toml`
- `raw/initial/...` source copy
- typed wiki pages created by ingest:
  `wiki/references/initial-product-source.reference.md` and
  `wiki/plans/bootstrap-ingest.plan.md`

The temp project can answer the domain question "what should the first web
product wiki track?" from its wiki: product decisions, user-facing behavior,
API work, frontend work, and implementation plans.

## D6 Evidence

After this eval, the project wiki has 50 markdown pages across specs,
decisions, proposals, roadmaps, plans, evals, checklists, references, and
archive. The root index remains small enough for index-first navigation:
`wiki/index.md` is 1,033 words after this eval entry.

Ten index-first lookup checks were satisfied from the current index and linked
pages:

1. distribution model: `wiki/decisions/llm-wiki-binary-distribution.decision.md`
2. managed runtime: `wiki/decisions/binary-path-bootstrap.decision.md`
3. search backend: `wiki/decisions/search-backend-selection.decision.md`
4. composable init: `wiki/decisions/composable-project-init.decision.md`
5. skill projection template engine:
   `wiki/decisions/skill-projection-template-engine.decision.md`
6. D10 execution: `wiki/plans/composable-project-init.plan.md`
7. D11 execution: `wiki/plans/project-and-skill-rename.plan.md`
8. V1 fixture smoke: `wiki/checklists/v1-fixture-smoke.checklist.md`
9. qmd-rs search evaluation: `wiki/evals/search-backend-selection.eval.md`
10. framework ordering: `wiki/roadmaps/framework-v1.roadmap.md`

## D7 Evidence

Two distinct temp projects were bootstrapped:

- `/private/tmp/llm-wiki-proof-web-product`, using blueprint `web-product`
  with packs `api`, `frontend`, and `ops-lite`.
- `/private/tmp/llm-wiki-proof-ml-research`, using blueprint `ml-research`
  with packs `ml`, `data`, and `research`.

Both projects have navigable wiki indexes, copied initial raw sources, typed
wiki pages created from those sources, and log entries recording the ingest.
No direct human edits to `wiki/` were required; the agent performed the wiki
bookkeeping.

## Residual Risk

The proof projects live under `/private/tmp`, so they are evidence for the
current session rather than committed fixtures. A future release gate should
either preserve proof fixtures or add an automated smoke test that repeats the
two-blueprint bootstrap and ingest sequence.
