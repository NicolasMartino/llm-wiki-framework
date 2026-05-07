# Source 001: Askama 0.16.0 package metadata

- URL: https://docs.rs/crate/askama/latest
- Retrieved: 2026-05-07
- Mode: web
- Source Type: primary package documentation

## Relevant Facts

- docs.rs lists `askama 0.16.0` as the latest package page.
- Package description: type-safe, compiled Jinja-like templates for Rust.
- Version list shows `0.16.0` published on 2026-04-29 and `0.15.6` on
  2026-03-24.
- docs.rs links the homepage to Askama's book at `askama.rs` and the repository
  to `askama-rs/askama`.
- The package page says Askama generates type-safe Rust code from templates at
  compile time based on a user-defined context struct.
- Current local toolchain observed during research: `rustc 1.92.0`, `cargo
  1.92.0`.

## D10 Relevance

D10 should use `askama = "0.16"` as the first implementation candidate, then
let local `cargo build` validate the dependency. The current local Rust
toolchain is modern enough that an Askama 0.16 build is unlikely to be blocked
by MSRV locally.

## Notes

The docs.rs `/latest/features` link returned a page labeled 0.15.6 during this
research session, while the main package page labeled latest as 0.16.0. Treat
the main package metadata as current and verify through Cargo during
implementation.
