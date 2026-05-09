# V1 Fixture Smoke

- Document Class: Checklist
- Status: Completed
- Date: 2026-05-06
- Category: Compatibility, release gate
- Scope: Agent-driven smoke procedure for the committed v1 wiki fixture.
- Sources: wiki/plans/llm-wiki-binary.plan.md, tests/fixtures/wikis/v1/
- Related: wiki/decisions/llm-wiki-binary-distribution.decision.md

## Procedure

1. Copy `tests/fixtures/wikis/v1/` to a temporary project.
2. Add one small raw source under `raw/`.
3. Run the `wiki-ingest` skill against that source.
4. Ask one query that requires reading the fixture spec and decision.
5. Run the `wiki-lint` skill.
6. Confirm `wiki/index.md` and `wiki/log.md` stay coherent after the run.

## Pass Criteria

- The ingest creates or updates typed wiki pages with sources.
- The query cites fixture wiki pages.
- The lint pass reports no unresolved bookkeeping issues or records its fixes.

## Result

Completed on 2026-05-09 in `/private/tmp/llm-wiki-v1-smoke-20260509`.

- Ingest created `wiki/references/runtime-template-smoke.reference.md` from
  `raw/smoke/runtime-template-note.md`.
- Query answer used the fixture spec and decision:
  `wiki/specs/fixture.spec.md` validates the v1 wiki shape, and
  `wiki/decisions/fixture.decision.md` records typed markdown documents as the
  durable fixture choice.
- Lint found and fixed a stale `Updated` date in the temp copy's
  `wiki/index.md`; no unresolved contradictions or orphan pages remained.
