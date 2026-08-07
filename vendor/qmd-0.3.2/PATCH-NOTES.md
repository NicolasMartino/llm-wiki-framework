# Vendored `qmd` 0.3.2 — patch notes

This is a vendored copy of the published `qmd` 0.3.2 crate, redirected from the
root workspace via `[patch.crates-io] qmd = { path = "vendor/qmd-0.3.2" }`.

## What was changed

This vendor carries **load-bearing source changes**, not just a manifest tweak.
Do **not** replace it with the unmodified crates.io `qmd = "0.3.2"` — the build
depends on the added API below (`src/search/gguf_runtime.rs` calls
`qmd::RuntimeOptions::auto()` / `qmd::RuntimeOptions::cpu()`), which upstream
0.3.2 does not expose.

The diff against pristine crates.io `qmd` 0.3.2 is:

1. **`Cargo.toml.orig` / `Cargo.toml` — `llama-cpp-2` default features disabled.**
   Upstream declares `llama-cpp-2.workspace = true`; the vendor pins
   `llama-cpp-2 = { workspace = true, default-features = false }` so the build does
   not compile/bundle llama.cpp's default backend set.

2. **`src/llm.rs` — a `RuntimeBackend` / `RuntimeOptions` runtime API.** Adds an
   `Auto` vs forced-`Cpu` backend selector (zeroes GPU layers, disables KQV/op
   offload, and selects the llama.cpp CPU device) plus `with_runtime_options` and
   `reload_with_runtime_options` constructors on the embedding, generation, and
   rerank engines. This is the CPU-fallback hardening llm-wiki relies on when GPU
   execution is unavailable or unstable.

3. **`src/lib.rs` — re-exports** `RuntimeBackend` and `RuntimeOptions` so the new
   API is reachable as `qmd::RuntimeOptions` / `qmd::RuntimeBackend`.

If a future upstream release ships an equivalent runtime-backend API and the
default-feature pin, the patch and this vendor directory can be removed and the
call sites in `src/search/gguf_runtime.rs` updated to match.

## License

`qmd` is dual-licensed Apache-2.0 OR MIT (see `README.md`). The upstream
`LICENSE-APACHE` / `LICENSE-MIT` text files are **not** shipped in the published
crate tarball, so the canonical texts were re-added here (`LICENSE-APACHE`,
`LICENSE-MIT`) to satisfy redistribution requirements.

## Native build dependencies

Building this crate (via `llama-cpp-2`'s C/C++ bindings) requires a working C/C++
toolchain plus `clang`, `cmake`, and `libclang` on the build host. These are not
declared in Cargo metadata; install them through the system package manager
before `cargo build`.
