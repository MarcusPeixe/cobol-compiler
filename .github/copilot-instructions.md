# Copilot Instructions for COBOL Compiler

This file guides AI agents working on the COBOL compiler project. For high-level decisions and conventions, see [AGENTS.md](../AGENTS.md). For current phase status, see [STATUS.md](../STATUS.md).

## Before You Start

**Nix is required.** Cargo is only available inside the Nix development shell:

```sh
nix develop
```

Then run cargo commands directly (no `nix develop --command` prefix needed inside the shell):

```sh
cargo build
cargo test
cargo fmt
cargo clippy --all-targets
```

If you get "cargo: command not found", you're not in the Nix shell. Run `nix develop` first.

## Quick Commands

| Task | Command |
| --- | --- |
| Run all tests | `cargo test` |
| Run a single test file | `cargo test --test parser_golden_tests` |
| Run a single test | `cargo test test_minimal_program` |
| Format code | `cargo fmt` |
| Check for warnings | `cargo clippy --all-targets` |
| Review & accept snapshot changes | `cargo insta test` then follow prompts |

## Architecture & Phases

The compiler is built in phases:

1. **Lexer** (src/lexer/) — Tokenizes fixed-format COBOL. Includes fixed-format reader and COPY preprocessor.
2. **Parser** (src/parser/) — Builds an AST from tokens. Handles context-sensitive words and semantic grouping.
3. **Semantic Analysis** (not started) — Type checking, identifier uniqueness, control flow validation.
4. **Code Generation** (not started) — Backends (C translation or bytecode) chosen later.

Each phase is its own module with:
- `mod.rs` — Public API and module structure
- `tests.rs` — Comprehensive unit tests (declared with `#[cfg(test)] mod tests;` at the bottom of `mod.rs`)
- Integration tests in `tests/` using `insta` snapshots

## Test File Organization

**Test programs live in `cobol-examples/` as separate `.cob` files, never inline in Rust.**

Naming convention: `{PHASE}_test_{TEST_NAME}.cob`

| Phase | Prefix | Example |
| --- | --- | --- |
| Lexer | `lexer_test_` | `lexer_test_string_continuation.cob` |
| Parser | `parser_test_` | `parser_test_minimal_program.cob` |
| Semantic | `semantic_test_` | `semantic_test_move_validation.cob` |
| Codegen | `codegen_test_` | `codegen_test_basic_arithmetic.cob` |

The test function name matches the filename: `test_minimal_program()` uses `parser_test_minimal_program.cob`.

## Golden Testing with insta

"Integration tests" always means golden snapshot testing using the `insta` crate.

### Workflow

1. **Run tests with `cargo insta test`** (not `cargo test`)
2. **Review colored diffs** — insta shows what changed in a readable format
3. **Accept or reject** — `a` to accept, `r` to reject, `i` to ignore. Answer for each changed snapshot.
4. **Commit the snapshots** — `.snap` files in `tests/snapshots/` are version-controlled.

### Output Design

When implementing a new compiler phase, design the output format with integration testing in mind:

- **Maximize information** — Print every node, field, and detail. All divisions, sections, statements, even if empty.
- **Prioritize completeness over pretty printing** — This output is compiler-internal (for debugging), not user-facing.
- **Never abbreviate or count** — Don't say "3 statements"; list each statement individually.
- **Example:** A paragraph with 0 statements still prints the paragraph header; it doesn't disappear.

The golden snapshots catch structural changes reliably because they contain complete information.

### Reviewing Snapshot Changes

When output changes (intentionally or not):

```sh
cargo insta test
# insta shows the diff interactively
# Review each change carefully:
#   - Is this expected behavior change? (Accept with 'a')
#   - Is this a regression? (Reject with 'r')
# After accepting, commit the .snap files:
jj commit -m "Update snapshots for parser output changes"
```

## Documentation & Memory

**AGENTS.md is the single source of truth.** When you discover or establish a convention:

1. **Update AGENTS.md first** — Add the decision to the Conventions or Decisions section with examples
2. **Store in GitHub Copilot Memory** (optional, for cross-session discovery) — Repository-scoped memory at https://github.com/MarcusPeixe/cobol-compiler/settings/copilot/memory
3. **Cite AGENTS.md in memory** — Memory is a convenience layer; AGENTS.md is the source

## Version Control: Jujutsu Only

This repository uses **Jujutsu (`jj`), not Git.** Commands:

```sh
jj new          # Create a new change
jj commit -m ""  # Finalize the current change
jj diff         # Show changes
jj branch       # Manage branches
jj log          # View history
```

**Agents do not commit automatically.** After implementing changes, summarize what you've done in the chat, and let the user commit with `jj commit`.

For help: `jj help` or https://martinvonz.github.io/jj/

## Common Patterns

### Adding a Unit Test to a Module

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_something() {
        let input = "some cobol code";
        let result = parse_it(input);
        assert_eq!(result, expected);
    }
}
```

Declare this module at the bottom of `mod.rs` with `#[cfg(test)] mod tests;`.

### Adding a Golden Integration Test

1. Create `cobol-examples/parser_test_my_feature.cob` with test COBOL code
2. Add a test function to `tests/parser_golden_tests.rs`:
   ```rust
   #[test]
   fn test_my_feature() {
       test_parser_golden("parser_test_my_feature.cob");
   }
   ```
3. Run `cargo insta test` and accept the snapshot
4. Snapshots are auto-generated as `tests/snapshots/parser_golden_tests__parser_test_my_feature.cob.snap`

### Error Handling

- **Library code** (`src/*`): Use `thiserror` for typed errors, return `Result<T, E>`
- **CLI/main** (`src/main.rs`): Use `anyhow` for ergonomic error handling
- **Rendering**: Use `miette` for pretty error display on stderr

Every diagnostic is a `diagnostics::Diagnostic` with a stable code like `cobol::parser::undefined_identifier`.

## Gotchas

- **Lexer has no EOF token** — parsing must handle stream ending naturally
- **Signs are always separate tokens** — `+5` lexes as `+` then `5`; the parser folds signs into literals
- **Fixed format only** — columns 1-6 and 73+ ignored; column 7 is the indicator
- **Comment-entry text ends early** — `Enio B. Basso` ends at the period (lexer spec quirk)
- **Snapshots must be complete** — Every structural node must appear; never abbreviate. Diffs in snapshots catch regressions reliably because they're exhaustive.
