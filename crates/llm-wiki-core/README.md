# llm-wiki-core

The library shared by `llm-wiki` and `poman`: how the project's wiki pages are
named, read and typed, and how both binaries serve MCP.

- `types` splits a wiki filename (`[slug].type.md` or `[index]-[slug].type.md`)
  into its parts.
- `page` reads a page's title and metadata fields from its text, each field
  with its line and the form it was written in, and gives two views of them:
  the wiki's, which llm-wiki's search reads, and the bullet block under the
  title, which poman reads.
- `types::format` defines the forms a field's value may take, and
  `types::poman` poman's own types, the deadline so far.
- `names` holds the slug rule, a title's rule, and how close a mistyped name
  is to a known one.
- `mcp` is the MCP plumbing both binaries' servers use: the stdio transport,
  the JSON-RPC envelope and the dispatch every server shares.
