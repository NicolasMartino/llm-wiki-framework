# poman

The project manager for projects kept with `llm-wiki`. It ships beside
`llm-wiki`: each release has an archive of its own for poman,
`poman-<target>.tar.xz`, next to `llm-wiki-rs-<target>.tar.xz`, and a
`poman-installer.sh` next to `llm-wiki-rs-installer.sh`. Both installers put
their binary in Cargo's bin folder; if you unpack the archives by hand, unpack
both into one folder. `llm-wiki install` then installs poman with itself, and
refuses without it. If you installed llm-wiki with `cargo install llm-wiki-rs`,
also run `cargo install poman`.

- `poman new deadline "<title>"` writes `wiki/deadlines/<slug>.deadline.md`
  from its flags (`--status`, `--deadline`, `--duration`, `--importance`,
  `--blocked-by`, `--track`, `--who`, `--slug`), asking on a terminal for a
  mandatory field left out; it never overwrites a file.
- `poman check` holds every deadline file under `wiki/` to its type, each
  `Blocked by` path to a deadline file, and `poman.toml` to its one key,
  `landing-branch`; it warns on near misses.
- Both take `--json`. Exit codes: 0 done, 1 output failed, 2 usage, 3 the
  check found an error, 4 `poman new` refused, 5 not in a Git repository or
  a file it could not write.
- `poman mcp` serves the two commands as the MCP tools `poman_check` and
  `poman_new_deadline` over stdio.
