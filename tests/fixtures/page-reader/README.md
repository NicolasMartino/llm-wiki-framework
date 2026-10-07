# Page Reader Corpus

A frozen copy of this repository's `wiki/` at commit `6bd0d4f`, and in
`forms/` one small page per form and edge of a page's metadata: each case of
the page reader's first seven unit tests, front matter with blank lines,
indented continuations and an indented YAML list, front matter with no title
after it or never closed, blocks before and after the title, bullets, bare and
bold keys, the prose line read as a field, pages with no title, and pages with
search ignore markers.

`search::page_reading_regression` reads every page here, masked as search
masks it, and records each page's title and fields in the `frozen_pages`
snapshot. That snapshot was taken with the reader search used before it moved
into `llm-wiki-core`, and the reader in that crate must still match it. The
live `wiki/` changes with every wiki change, so it could not hold that
snapshot still.

Leave the copy as it is: a page changed or added here changes the snapshot,
which then no longer proves what the old reader gave.

`just audit-legacy` leaves this copy out: it is the wiki as it stood, legacy
wording included, and the live `wiki/` is where that wording is audited.
