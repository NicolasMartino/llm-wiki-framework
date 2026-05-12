# Agent Owns Wiki

- Document Class: Decision
- Status: Accepted
- Date: 2026-04-23
- Category: Ownership model
- Scope: The agent has full control of all wiki/ content. Humans curate raw/ and make judgment calls.
- Sources: raw/research/llm-wiki-pattern-research.md

## Choice

The agent owns `wiki/` entirely. The agent is allowed to edit any file under
`wiki/`; humans are not allowed to edit `wiki/` directly and curate `raw/`
instead. The agent writes, updates, cross-links, archives, and maintains all
wiki content.

## Why

The Karpathy insight: "The tedious part of maintaining a knowledge base is
not the reading or the thinking - it's the bookkeeping." The agent handles
bookkeeping (cross-references, index updates, contradiction checks, status
tracking) at near-zero cost. Humans handle curation (what sources matter)
and judgment (what decisions to make).

The legacy model assumed human authorship of all documentation. This created
maintenance burden: stale docs, missing cross-references, orphan pages,
inconsistent statuses.

## Alternatives Considered

1. **Human writes docs, agent assists.** Rejected: creates ambiguous
   ownership. Who updates the index? Who checks for contradictions? The
   bookkeeping falls through cracks.

2. **Agent suggests, human approves every change.** Rejected: adds friction
   that defeats the purpose. The wiki would lag behind reality the same way
   human-maintained docs do.

## Consequences

- Humans interact with the project through raw source curation and queries
- The agent is responsible for wiki accuracy and consistency
- Wiki pages cite raw sources for provenance
- Humans can override agent decisions by adding raw sources that correct them
- The agent must run lint to catch its own errors

## Revisit When

- Agent-generated content quality degrades below useful thresholds
- Regulatory or compliance requirements demand human sign-off on documentation
