# Typed Documents

- Document Class: Decision
- Status: Accepted
- Date: 2026-04-23
- Category: Documentation model
- Scope: Preserve the typed document system (spec, decision, proposal, roadmap, plan, experiment, eval, checklist, reference) from the legacy framework within the LLM Wiki structure.
- Sources: raw/legacy/legacy-project-guidelines.md, raw/research/llm-wiki-pattern-research.md

## Choice

Keep nine typed document roles from the legacy framework. Each document type
answers a specific question and carries a specific relationship to truth.

| Type | Question | Truth relationship |
| --- | --- | --- |
| Spec | What is validated? | Current truth |
| Decision | What choice and why? | Durable truth |
| Proposal | Should we do this? | Unvalidated direction |
| Roadmap | In what order? | Execution coordination |
| Plan | How to execute? | Tactical steps |
| Experiment | What did we learn? | Evidence |
| Eval | How did it perform? | Measured evidence |
| Checklist | What procedure? | Repeatable operations |
| Reference | What external evidence? | Source synthesis |

## Why

The original Karpathy LLM Wiki uses untyped wiki pages organized by
topic/entity. This works for personal knowledge bases but loses structural
guarantees in a software project context.

Typed documents solve a specific problem: preventing the conflation of
"what is true" with "what we want to be true." The type system makes
it structurally impossible to confuse a proposal with a spec if the
agent follows the schema.

## Alternatives Considered

1. **Untyped wiki pages (pure Karpathy model).** Rejected: no structural
   guarantee that validated truth and proposals are distinguishable. Relies
   entirely on the agent's discipline, which degrades over long sessions.

2. **Fewer types (just spec, proposal, plan).** Rejected: loses useful
   distinctions. Decisions vs specs (durable choice vs current state),
   experiments vs evals (open question vs formal measurement), roadmaps
   vs plans (coordination vs execution) all carry distinct semantics.

## Consequences

- Wiki pages are organized by type in subdirectories (`wiki/specs/`, `wiki/decisions/`, etc.)
- Each type has its own status vocabulary
- The agent must classify every piece of knowledge into the correct type
- Templates exist for each type in `project_guidelines.md`

## Revisit When

- A document type is consistently unused across multiple projects
- Two types are regularly confused or conflated in practice
