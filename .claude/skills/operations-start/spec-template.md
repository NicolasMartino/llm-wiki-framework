# The spec

The coordinator writes this per task, then pastes the kind's base text under it
unchanged (see `SKILL.md`). Every `<…>` here is filled from the issue and the
checked facts; nothing in the base text is. A brief says what the worker must
not get wrong, never how to write the code: the worker finds the solution. A
list of files or impacts is a first guess, and the spec says so.

## The shape

```
Goal: <one sentence: do issue #<n>, what comes out (a PR that closes #<n>, a comment on #<n>)>

Files:
- <the paths or areas the worker may change, from the issue; say it is a first list and give the search that rechecks it>
- Read anything else; ask before editing any other file.

Out of scope:
- <neighbouring issues that own nearby work, by number and what they do>
- <work on hold, by name>

Context (established by the coordinator on develop <sha>; verify, don't rediscover):
- Read first: `gh issue view <n> --comments`, its roadmap entry `<wiki/roadmaps/….roadmap.md, D<n>>` and, for code or tooling, its plan `<wiki/plans/….plan.md>`, then <any comment the issue rests on, with its link>.
- <facts already found and checked: where things live, what an earlier attempt did, owner decisions with their date>

Done when:
- `git log --oneline -1` in the worktree showed <sha> at the start.
- <each check the issue's "Done when" asks for, as a command and its expected result>
```

Name the plan whose work this PR does, and its roadmap entry, in Context
whenever there is one: the `plan-status.txt` part has the worker set their
status in this PR. Say whether this PR finishes everything the roadmap entry
includes; if not, the entry stays Active. A spec with no plan says so, and
names the roadmap entry whose status the PR sets, if any. A PR that writes a
plan leaves that plan Draft. A plan Active on `develop` from before
2026-10-06 is named as an exception: this PR completes it.

`<sha>` is the commit the worktree really starts from, the one passed as
`--base-branch`. The base text's "Done when, as well as the spec's" lines, its
report and the shared rules follow the spec, so the spec repeats none of them.

## What each kind's spec fills

### A code change

- Goal: `Do issue #<n>: <the behaviour llm-wiki gains or loses>, as its plan <wiki/plans/….plan.md> says, and open one draft PR that closes #<n>.`
- Files: `<the modules, tests and spec pages the issue and plan name; a first list, recheck it>`; `templates/…` only if the work is meant to change what every project gets.
- Context: the spec or decision it rests on, `<wiki/specs/… or wiki/decisions/…>`.
- Done when: `<the plan's and the issue's checks, each as a command with its expected result>`. The full gates add-on adds `just verify` and CI.

### Tooling

- Goal: `Do issue #<n>: <what the tooling does once this merges>, and open one draft PR that closes #<n>.`
- Files: `<the tools/, .github/ or justfile paths the issue names; a first list, recheck it>`.
- Context: `<facts already found: the failing command and its output>`.
- Done when: `<the issue's checks, each as a command with its expected output>`.

### Settling a design with the owner (no code)

- Goal: `Do issue #<n> with the owner: settle <the design question>, and post the result as one comment on #<n> once the owner approves it. No code.`
- Files: none; the comment is the output.
- Context: `<the comment or note the question comes from, with its link>`; the owner's words so far, verbatim `<quotes, with date>`; what the decisions already say `<pages and passages>`; the questions to settle `<the list>`; `Number your questions to the owner from <the next Roman numeral>.`
- Done when: `<each piece of work it decides is named as a roadmap entry to add, in the order to build them>`.

### A wiki PR (a plan included)

- Goal: `Do issue #<n>: <what the wiki says once this merges>, and open one draft PR that closes #<n>.` For a plan: `Write the plan for <deliverable> (<roadmap, D<n>>) as wiki/plans/<slug>.plan.md, Status Draft, for the owner to approve.`
- Files: `<the wiki pages the issue names; a first list>`; the base adds the search that rechecks it.
- Done when: `<the issue's checks>`.

### An investigation

- Goal: `Do issue #<n>: answer <the question>, and post the answer as one comment on #<n>.`
- Files: `Read anything: <the sources to read, with the commands to read them>.`
- Out of scope: `<neighbouring issues; work on hold>`.
- Context: what is already known `<facts and failures already found, with links>`; `Time box: <hours>.`

### A fix round

- Goal: `Fix round on PR #<n> (<what the PR does>): address <the blind review's or the owner's review comment, with its link>, and push to the PR's branch.`
- Files: the PR's files, plus any the review names.
- Context: `The review already ran: skip the review ask.`; for each P1 and P2 (or each point of the owner's review), one line on what it asks; the P3s to fix or to name for filing.
- Done when: each point fixed or answered in a PR comment; the kind's gates on the new head.
