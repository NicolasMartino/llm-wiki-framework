# poman

The project manager for projects kept with `llm-wiki`. It ships beside
`llm-wiki`: each release has an archive of its own for poman,
`poman-<target>.tar.xz`, next to `llm-wiki-rs-<target>.tar.xz`, and a
`poman-installer.sh` next to `llm-wiki-rs-installer.sh`. Both installers put
their binary in Cargo's bin folder; if you unpack the archives by hand, unpack
both into one folder. `llm-wiki install` then installs poman with itself, and
refuses without it. If you installed llm-wiki with `cargo install llm-wiki-rs`,
also run `cargo install poman`.
