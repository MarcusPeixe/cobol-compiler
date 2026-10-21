---
applyTo: "tests/**,src/**/tests.rs"
---

# Testing rules

- Unit tests live in a sibling `tests.rs` next to each `mod.rs`, declared with `#[cfg(test)] mod tests;`.
- Integration tests are single files in `tests/`. `tests/lexer_golden_tests.rs` lexes the 7 files in `cobol-examples/` through the library (not the binary) and compares with `tests/lexer-goldens/<file>.cob.tokens`.
- Golden format: `file:LLL:CCC  TOKEN` lines, a blank line, `DIAGNOSTICS:`, then `WARNING <code> <file:line:col> <message>` lines. Lexer errors fail the test.
- When lexer output changes intentionally, update the goldens and review the diff; do not just overwrite them blindly.
- Run with `nix develop --command cargo test`; finish with `cargo fmt` and `cargo clippy --all-targets` warning-free.
