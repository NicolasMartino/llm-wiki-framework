# llm-wiki-core

The library shared by `llm-wiki` and `poman`: how the project's wiki pages are
named, read and typed.

- `types` splits a wiki filename (`[slug].type.md` or `[index]-[slug].type.md`)
  into its parts.
- `page` reads a page's title and metadata fields from its text, each field
  with its line and the form it was written in, and gives two views of them:
  the wiki's, which llm-wiki's search reads, and the bullet block under the
  title, which poman reads.
