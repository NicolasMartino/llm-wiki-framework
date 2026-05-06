# Knowledge Lint Skill

- Document Class: Spec
- Status: Active
- Date: 2026-05-06
- Category: Tooling
- Scope: The `knowledge-lint` skill and `$knowledge lint` namespace entry for scanning and fixing wiki consistency issues.
- Related: wiki/specs/documentation-model.spec.md, wiki/specs/knowledge-query-skill.spec.md, wiki/specs/knowledge-ingest-skill.spec.md, wiki/decisions/knowledge-command-namespace.decision.md, wiki/decisions/single-source-skills.decision.md

## What It Does

The knowledge-lint skill runs a consistency pass over the current project's
wiki. It checks for contradictions, stale claims, orphan pages, missing
cross-references, and index drift, then fixes clear bookkeeping issues
directly.

In Codex, it can trigger from normal requests to lint the wiki, through direct
explicit invocation with `$knowledge-lint`, or through the namespace alias
`$knowledge lint`. In Claude, it can trigger through direct `/knowledge-lint`
invocation or normal language.

## Location

Canonical source: `skills/knowledge-lint/SKILL.md`
Claude generated skill: `.claude/skills/knowledge-lint/SKILL.md`
Codex generated skill: `.codex/skills/knowledge-lint/SKILL.md`
Codex UI metadata source: `skills/knowledge-lint/codex/openai.yaml`

## Workflow

1. Read `wiki/index.md` to orient.
2. Identify candidate pages for contradiction, status, orphan, or cross-link
   checks.
3. Read the minimum relevant pages needed to confirm each issue.
4. Fix clear bookkeeping issues directly in `wiki/`.
5. Ask the user to resolve only genuinely ambiguous contradictions.
6. Update `wiki/index.md` when needed.
7. Append the lint pass to `wiki/log.md`.

## Key Behaviors

- **Index-first navigation** — starts from `wiki/index.md`
- **Direct bookkeeping fixes** — repairs clear issues without unnecessary
  approval loops
- **Contradiction escalation** — asks only when conflicting claims cannot be
  resolved from documented sources
- **Log mutation tracking** — records the lint pass in `wiki/log.md`

## Proven By

- Claude skill file exists at `.claude/skills/knowledge-lint/SKILL.md`
- Codex skill file exists at `.codex/skills/knowledge-lint/SKILL.md`
- Canonical skill source exists at `skills/knowledge-lint/SKILL.md`
- `bash skills/build.sh` renders the Claude and Codex outputs
- Codex UI metadata exists at `.codex/skills/knowledge-lint/agents/openai.yaml`
- Codex global symlink exists at `~/.codex/skills/knowledge-lint`
- Claude global symlink exists at `~/.claude/skills/knowledge-lint`
- Codex dispatcher skill routes `$knowledge lint` to `knowledge-lint`

## Limitations

- Cannot resolve substantive contradictions without a documented source of
  truth or user judgment
