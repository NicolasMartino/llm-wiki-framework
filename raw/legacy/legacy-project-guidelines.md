# Project Guidelines Next - Software / AI Engineering

- Document Class: Proposal
- Status: Draft
- Date: 2026-04-23
- Category: Documentation model
- Scope: Proposed next version of the project documentation and execution guidelines.

## Purpose

This file proposes the next documentation model for LLM Wiki.

The goal is to keep the existing separation between current truth, future
intent, and execution support, while adding `*.roadmap.md` as a first-class
process document for orchestrating deliverables and plans.

## Core Rule

Document tested truth, not intended truth.

The repository should always make it clear whether a statement is:

1. validated reality
2. a future direction under consideration
3. an execution plan
4. an experiment or evaluation result
5. a durable decision

## Documentation Lanes

Primary lanes:

1. `docs/current/` - validated, current truth
2. `docs/next/` - proposed or unvalidated future direction
3. `docs/process/` - execution support, orchestration, plans, experiments, and checklists

Support lanes:

1. `src/` - application and model code
2. `tests/` - automated verification
3. `models/` - model artifacts, configs, lineage, and training notes
4. `data/` - datasets, source definitions, schemas, and pipeline definitions
5. `notebooks/` - exploratory analysis and prototyping
6. `scripts/` - utilities and repeatable automation
7. `infra/` - local and cloud infrastructure definitions
8. `evals/` - evaluation suites, benchmark harnesses, and reports

## Document Roles

Use the document type by role, not by convenience.

| Document type | Core question | Purpose |
| --- | --- | --- |
| `*.proposal.md` | Should we do this? What future direction are we considering? | Future change, product direction, architecture direction, or redesign that is not validated yet |
| `*.roadmap.md` | In what order will we pursue accepted or intended deliverables, and which plans execute each one? | Ordered orchestration of deliverables, dependencies, proof gates, and execution plans |
| `*.plan.md` | How do we execute one deliverable or bounded workstream? | Tactical execution for work that needs sequencing, ownership, and verification |
| `*.experiment.md` | What uncertain question are we testing? | Investigation record with setup, observations, measurements, and conclusion |
| `*.eval.md` | How did a model or system candidate perform against an evaluation suite? | Formal evaluation report with metrics, failure analysis, and recommendation |
| `*.spec.md` | What became validated truth? | Current accepted baseline facts, interfaces, schemas, behavior, and measured properties |
| `*.decision.md` | What lasting choice did we make, and why? | Accepted durable architectural, product, model, data, or process decision |
| `*.checklist.md` | What repeatable procedure must be followed? | Operational, release, deployment, data, model, or incident procedure |
| `*.reference.md` | What external evidence or raw comparison material supports later synthesis? | Source notes for papers, APIs, vendor docs, benchmarks, and external examples |

Short version:

1. Proposal - direction
2. Roadmap - ordering
3. Plan - execution
4. Experiment - uncertainty
5. Eval - measured candidate performance
6. Spec - validated truth
7. Decision - durable choice
8. Checklist - repeatable procedure
9. Reference - raw evidence

## Roadmaps

Roadmaps are first-class process documents.

Use a roadmap when the project needs to coordinate multiple deliverables,
dependencies, and execution plans across a stage, milestone, or product slice.

Roadmaps live in:

```text
docs/process/roadmaps/
```

Roadmaps use:

```text
*.roadmap.md
```

A roadmap should define:

1. ordered deliverables
2. dependency logic between deliverables
3. status of each deliverable
4. linked proposals, plans, experiments, and specs
5. proof gates required to complete each deliverable
6. promotion targets for validated outcomes
7. intentionally excluded scope

A roadmap should not contain detailed implementation steps. Those belong in
`*.plan.md` files.

A roadmap does not validate reality. It coordinates intended work. Validated
outcomes must still be promoted into `docs/current/`.

## Deliverables

A deliverable is the smallest externally observable result that moves the
project forward.

Deliverables usually live as sections inside a roadmap. A complex deliverable
may have its own execution plan.

Each deliverable should answer:

1. **Promise** - what becomes true for a user, developer, operator, or evaluator?
2. **Included** - what is inside this deliverable?
3. **Excluded** - what is deliberately out of scope?
4. **Depends On** - what must already be true?
5. **Execution Plan** - which `*.plan.md` owns the detailed work?
6. **Proof** - what command, screenshot, test, demo, or measurement proves completion?
7. **Promotion Target** - what current spec or decision should be updated after validation?
8. **Unlocks** - what later deliverables become possible?

The deliverable rule:

> If the result cannot be observed or verified, the work unit is too vague.

## Roadmap Versus Plan

Roadmaps orchestrate multiple deliverables.

Plans execute one deliverable or bounded workstream.

Example:

```text
docs/process/roadmaps/local-learning.roadmap.md
  D1 - Gallery proves core workflow
      Execution plan: docs/process/planning/gallery-ui-creation.plan.md
  D2 - Real app renders cache-first fixtures
      Execution plan: docs/process/planning/cache-first-client-shell.plan.md
  D3 - Fake API drives one chat run
      Execution plan: docs/process/planning/fake-api-orchestration.plan.md
```

The roadmap answers:

1. what comes first
2. what depends on what
3. what proof completes each deliverable
4. which plan owns detailed execution

The plan answers:

1. what files, modules, systems, or workflows change
2. what steps will be taken
3. what verification will be run
4. what evidence will be recorded
5. when the plan can be closed

## Promotion Rule

1. Define future direction in `docs/next/` as a proposal when the direction is not yet accepted or validated.
2. Add or update a roadmap when the work needs ordering across deliverables.
3. Add a plan when a deliverable is multi-step, risky, cross-subsystem, or needs explicit sequencing.
4. Run implementation, prototypes, experiments, or evaluations in narrow increments.
5. Record evidence: tests, screenshots, logs, measurements, eval results, or experiment outcomes.
6. Promote only validated durable outcomes into `docs/current/`.
7. Record durable choices in `docs/current/decisions/`.
8. Close, archive, or update the proposal, roadmap item, plan, experiment, or eval after the outcome is known.

If an idea fails, the current truth does not change. Record the failure in an
experiment, eval, or plan outcome.

## Change Classification Rules

Before starting meaningful work, decide where the resulting information belongs:

1. If it changes validated reality, update `docs/current/`.
2. If it locks in a lasting choice, add or update a decision doc.
3. If it describes future direction, use `docs/next/`.
4. If it orders multiple deliverables or plans, use a roadmap.
5. If it executes one deliverable or bounded workstream, use a plan.
6. If it changes a repeatable procedure, add or update a checklist.
7. If it answers an uncertain question, record an experiment.
8. If it measures a model or system candidate, record an eval.
9. If it adds model, data, infra, or eval assets, update the relevant support folder.

Choose the lane by the role of the information.

## Naming And Metadata

Suggested filename patterns:

1. `[index]-[slug].proposal.md`
2. `[slug].roadmap.md`
3. `[slug].plan.md`
4. `[index]-[slug].experiment.md`
5. `[index]-[slug].eval.md`
6. `[index]-[slug].spec.md`
7. `[index]-[slug].decision.md`
8. `[slug].checklist.md`
9. `[slug].reference.md`

Suggested metadata block for all non-README documents:

```md
- Document Class: [Proposal / Roadmap / Plan / Experiment / Eval / Spec / Decision / Checklist / Reference]
- Status: [STATUS]
- Date: YYYY-MM-DD
- Category: [SHORT LABEL]
- Scope: [ONE SENTENCE]
```

Useful optional fields:

1. `Owner`
2. `Stage`
3. `Related Proposals`
4. `Related Roadmap`
5. `Related Plans`
6. `Related Subsystems`
7. `Model Version`
8. `Dataset Version`
9. `Promotion Targets`
10. `Supersedes`
11. `Superseded By`

## Status Vocabulary

| Document class | Typical statuses |
| --- | --- |
| Proposals | `Proposed`, `Deferred`, `Rejected`, `Accepted`, `Archived`, `Superseded` |
| Roadmaps | `Draft`, `Active`, `Paused`, `Completed`, `Archived`, `Superseded` |
| Plans | `Draft`, `Active`, `Blocked`, `Completed`, `Archived`, `Superseded` |
| Experiments | `Planned`, `Running`, `Recorded`, `Archived` |
| Evals | `Planned`, `Baseline`, `Candidate`, `Accepted`, `Rejected`, `Superseded` |
| Specs | `Active`, `Superseded` |
| Decisions | `Accepted`, `Superseded` |
| Checklists | `Active`, `Historical`, `Superseded` |
| References | `Draft`, `Sourced draft`, `Archived` |

## Repository Structure

```text
./
  README.md
  project_guidelines.md
  project_guildelines_next.md
  src/
    README.md
  tests/
  models/
    README.md
  data/
    README.md
  notebooks/
  evals/
    README.md
  scripts/
    README.md
  infra/
    README.md
  docs/
    README.md
    current/
      README.md
      spec/
        README.md
        *.spec.md
      decisions/
        README.md
        *.decision.md
    next/
      README.md
      *.proposal.md
      references/
        README.md
        *.reference.md
      archive/
    process/
      README.md
      roadmaps/
        README.md
        *.roadmap.md
        archive/
      planning/
        README.md
        *.plan.md
        archive/
      checklists/
        README.md
        *.checklist.md
      experiments/
        README.md
        *.experiment.md
```

## Folder Responsibilities

| Path | Responsibility |
| --- | --- |
| `README.md` | Top-level orientation and current focus |
| `project_guidelines.md` | Accepted project documentation and execution rules |
| `project_guildelines_next.md` | Draft next version of the project guidelines |
| `docs/` | Main documentation home |
| `docs/current/` | Latest validated baseline and confirmed facts |
| `docs/current/spec/` | Current specifications needed to reproduce or understand the accepted state |
| `docs/current/decisions/` | Accepted durable decisions |
| `docs/next/` | Proposed or unvalidated future direction |
| `docs/next/references/` | Raw evidence for research-heavy comparisons |
| `docs/process/` | Execution support, not long-term truth |
| `docs/process/roadmaps/` | Ordered deliverable orchestration across plans |
| `docs/process/planning/` | Tactical plans for bounded workstreams or deliverables |
| `docs/process/checklists/` | Repeatable procedures |
| `docs/process/experiments/` | Test, benchmark, investigation, and learning records |
| `src/` | Application code, services, APIs, frontend, workers, and pipelines |
| `tests/` | Automated tests |
| `models/` | Model artifacts, configs, training notes, and lineage |
| `data/` | Data sources, schemas, versioning, and data pipeline definitions |
| `notebooks/` | Exploratory work, not production source of truth |
| `evals/` | Evaluation methodology, benchmark harnesses, and reports |
| `scripts/` | Utilities and automation |
| `infra/` | Infrastructure as code and operational configuration |

## Folder README Responsibilities

| README | Intended job |
| --- | --- |
| `README.md` | Explain what the project is, how to set up the dev environment, and where to start |
| `docs/README.md` | Explain the documentation model and route readers to the right lane |
| `docs/current/README.md` | State the active validated stage or baseline |
| `docs/current/spec/README.md` | Give the reading order for current specs |
| `docs/current/decisions/README.md` | Explain what counts as a durable decision |
| `docs/next/README.md` | Explain what belongs in future or unvalidated docs |
| `docs/process/README.md` | Explain that this lane supports execution but does not replace baseline truth |
| `docs/process/roadmaps/README.md` | Explain how roadmaps orchestrate deliverables and plans |
| `docs/process/planning/README.md` | Explain when a plan is required |
| `docs/process/checklists/README.md` | Explain when procedures should become checklists |
| `docs/process/experiments/README.md` | Explain how to record tests and investigations |
| `src/README.md` | Explain code organization, entry points, and module boundaries |
| `models/README.md` | Explain model lineage, how to train, and how to reproduce results |
| `data/README.md` | Explain data sources, schemas, versioning, and access controls |
| `evals/README.md` | Explain evaluation methodology, metrics, and how to run eval suites |
| `infra/README.md` | Explain deployment topology, environments, and how to operate the system |

## Proposal Template

Use proposals for future direction or unvalidated changes.

A proposal should answer:

1. What problem are we solving?
2. What direction or change is proposed?
3. What user, developer, operational, or model benefit should it create?
4. What risks or tradeoffs does it introduce?
5. What alternatives remain alive?
6. What evidence is required before promotion?
7. What roadmap or plan will execute it if accepted?

Proposals belong in `docs/next/`.

## Roadmap Template

Use roadmaps for ordered deliverable orchestration.

A roadmap should answer:

1. What stage, milestone, or product slice does this roadmap coordinate?
2. What objective is it pursuing?
3. What principles constrain the sequence?
4. What deliverables are planned, in order?
5. What does each deliverable depend on?
6. Which plan executes each deliverable?
7. What proof completes each deliverable?
8. What does each deliverable unlock?
9. What outcomes should be promoted into current docs?

Recommended deliverable section:

```md
### D1 - [Deliverable Name]

Status: [Draft / Active / Blocked / Completed / Archived]
Promise: [Externally observable result]
Depends On: [None / D0 / external prerequisite]
Execution Plan: [path or "Not created yet"]

Included:
- ...

Excluded:
- ...

Proof:
- ...

Promotion Target:
- ...

Unlocks:
- ...
```

Roadmaps belong in `docs/process/roadmaps/`.

## Plan Template

Use plans for bounded execution.

A plan should answer:

1. What deliverable or workstream does this execute?
2. What files, modules, systems, or workflows are in scope?
3. What is explicitly out of scope?
4. What steps will be taken?
5. What verification gates must pass?
6. What evidence will be recorded?
7. What docs should be updated when done?
8. What closes the plan?

Plans belong in `docs/process/planning/`.

## Experiment Template

Use experiments when the project is trying to answer an uncertain question.

Every experiment should capture:

1. Objective
2. Hypothesis
3. Setup
4. Procedure
5. Observations
6. Measurements
7. Conclusion
8. Next action

Experiments belong in `docs/process/experiments/`.

## Eval Template

Use evals when assessing a model or system candidate against a defined suite.

Every eval should capture:

1. model or system version
2. eval suite and dataset versions
3. metrics and variance where applicable
4. comparison to baseline
5. failure analysis
6. resource profile
7. recommendation

Evals belong in `evals/` unless the project later creates a more specific
current/proposed eval lane.

## Current Spec Template

Current specs should answer:

1. What is currently accepted and validated?
2. What architecture, API, data flow, schema, or behavior exists now?
3. What is deliberately not yet validated?
4. What commands, tests, measurements, or artifacts prove the claim?
5. What limitations remain?

Specs belong in `docs/current/spec/`.

## Decision Template

Use decisions for lasting choices.

Every decision should answer:

1. What choice was made?
2. Why was it made?
3. What alternatives were considered?
4. What consequences or tradeoffs are accepted?
5. What evidence or plan supports the decision?
6. What would cause the decision to be revisited?

Decisions belong in `docs/current/decisions/`.

## Stage Or Lifecycle Model

The project should progress through stages, but stages should be expressed as
deliverables with proof gates.

Recommended stage rules:

1. each stage answers one narrow question
2. each stage has measurable exit criteria
3. each stage has a roadmap when multiple deliverables must be coordinated
4. later stages are not written into current docs before validation
5. model performance claims require recorded evaluation evidence

For LLM Wiki, the current validated stage remains:

```text
Stage 1 - project scaffold and local learning baseline
```

## Deliverables-First Execution

To reduce scope gravity, work should be framed as deliverables instead of
subsystems.

Prefer:

```text
Gallery proves the full mobile chat/research/cache workflow.
```

Avoid:

```text
Build shared UI, cache, API, auth, model orchestration, and retrieval.
```

Good deliverables are:

1. externally observable
2. small enough to verify in one sitting or one short workstream
3. tied to a proof command, screenshot, test, demo, or measurement
4. explicit about exclusions
5. connected to a roadmap and, when needed, a plan

Before starting work, ask:

> What is the smallest observable thing this task delivers, and what future work are we explicitly refusing to do inside it?

## Definition Of Done

Work is done only when:

1. the implementation exists
2. relevant automated tests or checks pass
3. required screenshots, demos, logs, measurements, experiments, or evals are recorded
4. validated baseline docs match reality
5. durable decisions are recorded when needed
6. future-only ideas remain in proposals or roadmaps
7. procedures are updated when repeatable operations changed
8. model, data, prompt, and eval changes include appropriate evidence
9. any plan used for the work has been closed or updated
10. the roadmap deliverable status is updated if the work completes or changes it

## Common Failure Modes To Prevent

1. writing future intent into current docs
2. using roadmaps as proof of progress
3. letting plans become the only source of truth
4. creating architecture work without an observable deliverable
5. leaving durable decisions only in chat
6. running experiments without recording setup and outcome
7. changing APIs or schemas without updating current specs after validation
8. claiming model performance without eval conditions
9. silently changing prompts, data, or configs without versioning
10. expanding scope inside a deliverable without updating the roadmap
11. creating placeholder documents that imply progress that does not exist

## Suggested Reading Order

For a new contributor:

1. `project_guidelines.md` - accepted documentation and execution rules
2. `project_guildelines_next.md` - proposed next documentation model, if under review
3. `README.md` - what the project is and how to set up
4. `docs/current/README.md` - what is true now
5. current specs - accepted baseline facts
6. current decisions - durable choices
7. active proposals in `docs/next/` - future direction
8. active roadmaps in `docs/process/roadmaps/` - deliverable order and orchestration
9. active plans in `docs/process/planning/` - execution details
10. recent experiments and evals - what was tried, measured, and learned
11. `models/README.md`, `data/README.md`, `evals/README.md`, and `infra/README.md`

## Bottom Line

This model keeps the project deliverables-first without losing documentation
discipline:

1. proposals define direction
2. roadmaps order deliverables and coordinate plans
3. plans execute bounded work
4. experiments and evals produce evidence
5. specs and decisions record validated truth

Roadmaps are the new coordination layer, but they do not replace validation.
The project advances when deliverables are proven and their durable outcomes
are promoted into current documentation.
