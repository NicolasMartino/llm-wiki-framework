# The facts helper's prompt

`/operations-start` gives this prompt, unchanged, to the read-only
`investigator` subagent (Sonnet), with `<n>` set to the issue. It gathers
facts; it does not pick the kind, write the spec or change anything. The
coordinator checks each fact it returns before the fact goes into the spec's
Context.

```
Gather the facts for a worker spec on issue #<n> of NicolasMartino/llm-wiki-framework. Read only: change no file, post nothing, start nothing. Do not suggest the kind of task, a design or a fix.

Return, as short grouped bullet lists, each fact with its source (a link, or the command that showed it):
1. The issue: its title, Status column on the board, labels, milestone, and its "What", "Out of scope" and "Done when" in a line each. (`gh issue view <n> --comments`; the column from `gh issue view <n> --json projectItems`)
2. Its roadmap entry and its plan, if any: the file, the entry or plan's Status, and the plan's Done when (read wiki/ pages with `llm_wiki_read`). Its parent issue and siblings: numbers, titles, states.
3. The comments the issue rests on: owner answers, analyses, reviews, each with its date and link, and the owner's exact words where they decide something.
4. Linked PRs: number, title, state (open, draft, merged, closed), head SHA, and whether the newest review verdict (a comment whose heading contains PASS or FAIL; CHANGES REQUESTED counts as FAIL, and a heading with both is no pass) is a PASS naming that head.
5. The files, areas and wiki pages the issue names, and where each lives today on master (`git grep -n`, `ls`; for wiki/ and raw/, `llm_wiki_search` and `llm_wiki_read`). Say which named paths no longer exist.
6. What merged since the issue was written that touches those files or that topic: `git log --oneline --since=<issue date> -- <paths>`, with PR numbers.
7. Owner decisions that apply, each with its date and where it is written (issue comment, wiki decision page).
8. Blockers: open issues it waits on, and anything on hold that it touches.

Say plainly what you could not find or confirm. 40 lines at most.
```
