# Testbed Eval

- Document Class: Eval
- Status: Active
- Date: 2026-05-12
- Category: Search infrastructure
- Scope: Ten-query fixture eval for autonomous model comparison.
- Sources: raw/fixture.md

| ID | Split | Query | Purpose | Applicable modes | Draft expected target pages |
| --- | --- | --- | --- | --- | --- |
| C1 | Calibration | `who owns wiki updates in this framework` | Conceptual ownership query | all | `wiki/decisions/agent-boundary.decision.md` |
| C2 | Calibration | `D9 natural-language search candidate comparison` | Mixed roadmap/query shape | all | `wiki/roadmaps/fixture.roadmap.md`, `wiki/proposals/search-lab-bench.proposal.md` |
| C3 | Calibration | `model artifact hashes and license evidence` | Reference retrieval | all | `wiki/references/model-card-notes.reference.md` |
| C4 | Calibration | `postgres connection pool shard size` | No expected match | all | none |
| H1 | Hold-out | `why humans curate raw sources` | Decision rationale | all | `wiki/decisions/agent-boundary.decision.md` |
| H2 | Hold-out | `path redaction before raw bundle export` | Plan detail | all | `wiki/plans/calibration.plan.md`, `wiki/checklists/release.checklist.md` |
| H3 | Hold-out | `documentation metadata contract status fields` | Spec concept | all | `wiki/specs/documentation-contracts.spec.md` |
| H4 | Hold-out | `release checklist timing fields` | Checklist retrieval | all | `wiki/checklists/release.checklist.md` |
| H5 | Hold-out | `GPU shader occupancy tuning` | No expected match | all | none |
| H6 | Hold-out | `browser automation screenshot plugin` | No expected match | all | none |
