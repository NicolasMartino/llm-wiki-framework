# V1 Fixture Smoke

- Document Class: Checklist
- Status: Active
- Date: 2026-05-06
- Category: Compatibility, release gate
- Scope: Agent-driven smoke procedure for the committed v1 wiki fixture.
- Sources: wiki/plans/llm-wiki-binary.plan.md, tests/fixtures/wikis/v1/
- Related: wiki/decisions/llm-wiki-binary-distribution.decision.md

## Procedure

1. Copy `tests/fixtures/wikis/v1/` to a temporary project.
2. Add one small raw source under `raw/`.
3. Run the `knowledge-ingest` skill against that source.
4. Ask one query that requires reading the fixture spec and decision.
5. Run the `knowledge-lint` skill.
6. Confirm `wiki/index.md` and `wiki/log.md` stay coherent after the run.

## Pass Criteria

- The ingest creates or updates typed wiki pages with sources.
- The query cites fixture wiki pages.
- The lint pass reports no unresolved bookkeeping issues or records its fixes.
