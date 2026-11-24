# AGENTS.md

Guidance for people and AI agents working on this repository. This file is the single source of truth for decisions: when you make a decision or learn something others need, add it here **in the same change** (other documents, such as the report, must cite it rather than redefine it).

Current phase, open questions and next steps live in [STATUS.md](STATUS.md). Read it before starting work.

Scoped rules are in [.github/instructions/](.github/instructions): `tests.instructions.md` (tests and goldens) and `report.instructions.md` (the Typst report).

## Project

A COBOL compiler in Rust, built in phases. Each phase is its own module with comprehensive unit tests. The language spec is [misc/EspecificacaoCOBOL.pdf](misc/EspecificacaoCOBOL.pdf) (Portuguese). For usage see [README.md](README.md).

## Workflow

- Cargo is only available inside the Nix dev shell. Run commands as `nix develop --command cargo <cmd>`, or enter `nix develop` first. `typst` is not installed, so the report cannot be compiled locally.
- Before finishing a change: `cargo fmt`, `cargo clippy --all-targets` (keep it warning-free) and `cargo test`. CI runs fmt check, clippy with `-D warnings` and `cargo test --locked`.
- **Snapshot testing (insta):** Golden tests use the `insta` crate for snapshot assertions. When test output changes, use: `cargo insta test` to view pending snapshots, `cargo insta review` to interactively accept/reject changes one-by-one, `cargo insta accept` to accept all, or `cargo insta reject` to reject all. Run `cargo test` after accepting snapshots to verify tests pass.
- **Documentation and memory:** When documenting decisions, conventions, or preferences in this file (AGENTS.md), also store them in GitHub Copilot Memory (repository-scoped) for retrieval across sessions. AGENTS.md is the single source of truth; memory is a convenience layer for discovery.
- **Version control:** This repository uses [Jujutsu](https://github.com/martinvonz/jj) (`jj`), not Git. Use `jj` commands for all VCS operations: `jj new`, `jj commit`, `jj branch`, `jj log`, `jj diff`, `jj rebase`, etc. When in doubt about syntax, check `jj help` or the [Jujutsu documentation](https://martinvonz.github.io/jj/). Never use `git` commands directly. The `.jj` directory contains the Jujutsu workspace configuration; the `.git` directory remains for compatibility only.
- **Agents do not commit:** After implementing changes, do not create commits. Instead, summarize what you've changed **directly in the chat** (not in `/tmp/` or other temp files) and let the user manually commit using `jj commit`. This preserves user control over versioning and commit messages.
- Never do: add `--format` options to the CLI, treat `cobol-examples/` as spec-conformant (they target GnuCOBOL), or hand-edit goldens without checking the diff against the lexer's real output; create summaries in temporary files like `/tmp/`.
- Every new module needs unit tests. Each module is a directory with `mod.rs` for the code and a sibling `tests.rs` for its tests, declared at the bottom of `mod.rs` with `#[cfg(test)] mod tests;`. Single-purpose submodules (`lexer/scanner/`, `lexer/token/`, `source/reader/`) follow the same pattern.
- Integration tests live in `tests/`; lexer regression goldens are under `tests/lexer-goldens/` and are exercised automatically by `cargo test`. Golden output contains the token listing followed by a plain `DIAGNOSTICS:` section with deterministic warning records; errors fail the integration test.
- Library crate (`src/lib.rs`) plus a thin binary (`src/main.rs`). New phases get their own module and their own CLI subcommand (`cobol-compiler lex <file>`, and so on).

## Layout

```
src/main.rs                 CLI (clap); anyhow is used only here
src/lib.rs                  module declarations
src/diagnostics/            shared Severity (I/W/E/S), Diagnostic and miette integration
src/source/                 SourceMap, Span, LogicalText
src/source/reader/          fixed-format reader
src/preprocess/             COPY expansion (runs before the lexer)
src/lexer/                  module root, re-exports the public API
src/lexer/token/            token types
src/lexer/scanner/          lexing
tests/                      integration tests
tests/lexer-goldens/        expected token output for professor example programs
```

Each directory above holds `mod.rs` plus `tests.rs` (except `src/lexer/`, which only re-exports).

```
misc/                        language spec, COBOL examples, and example.typ
documents/                   compiled PDF reports
typst-src/                   Typst report source files (except example.typ)
cobol-examples/             the professor's example programs (git subtree); they target GnuCOBOL, not strictly the spec
```

Pipeline: source file -> fixed-format reader -> `COPY` preprocessor -> lexer -> tokens.

## Conventions

- **Errors:** `thiserror` for typed errors in library code, `anyhow` only in `main.rs`, `miette` for rendering. Every compiler message is a `diagnostics::Diagnostic` with a stable code like `cobol::lexer::illegal_character`.
- **Error recovery:** phases collect all diagnostics and keep going rather than stopping at the first one.
- **Spans:** `Span` is a byte range in an original file (`FileId` into `SourceMap`). Text from copybooks keeps spans pointing into the copybook. `LogicalText` carries a per-byte origin map so spans survive column stripping and `COPY` expansion.
- **Integration tests & golden testing:** "Integration tests" always refers to golden testing using the `insta` crate. All new compiler phases should include golden tests. See existing test files in `tests/` for patterns.
- **Test program placement:** Never write compiler test programs inline in Rust source code. Always create separate `.cob` files in `cobol-examples/` with stage-specific prefixes: `lexer_test_`, `parser_test_`, `semantic_test_`, or `codegen_test_`. This makes tests maintainable and allows programs to be reused as examples.
- **Output formatting for integration tests:** When implementing a new compiler phase, design the output format with integration testing in mind. Maximize information (all nodes, fields, all details) to aid inspection and debugging. The output is compiler-internal, not user-facing, so prioritize completeness over pretty printing.
- **Snapshot testing:** Golden tests use `insta` for snapshot assertions. When output changes intentionally, use `cargo insta test` to review and accept new snapshots.
- **AST pretty-printing:** Print every node in the AST tree (all divisions, sections, fields, statements), including empty ones. Never count statements; always list them individually. This ensures snapshots catch structural changes.
- **Test file naming:** Test programs follow the convention `{PHASE}_test_{TEST_NAME}.cob` where `PHASE` is `lexer`, `parser`, `semantic`, or `codegen` and `TEST_NAME` matches the test function name (e.g., `parser_test_minimal_program.cob` for `test_minimal_program()`).

## Decisions

Lexer and source handling:

- Fixed format only (no free format). Columns 1-6 and 73+ are ignored, column 7 is the indicator.
- Comment lines (`*`, `/`) are dropped. `D` lines are skipped unless `--debug-lines` is given. Continuation lines (`-`) are joined only for string literals, and the previous line is padded to column 72.
- Case-insensitive: keywords and identifiers are uppercased in tokens. The original text stays available through the span.
- Only the 14 keywords from spec section 2.2 are reserved. Everything else (`PROGRAM-ID`, `VALUE`, `TO`, `AND`, `END-IF`...) is an identifier. The parser decides what they mean. `PIC` and `PICTURE` are special-cased.
- Identifiers: 1-30 characters of A-Z, 0-9 and `-`, not ending with a hyphen. Longer or hyphen-terminated words are errors.
- `_` is also accepted inside identifiers (but cannot start one) with a lexer warning `cobol::lexer::underscore_in_identifier`. This deviates from the spec; it exists because the professor's examples in `cobol-examples/` use names like `PROG008_STRING`.
- Comment-entry paragraphs: inside the IDENTIFICATION DIVISION, a paragraph named `AUTHOR`, `INSTALLATION`, `DATE-WRITTEN`, `DATE-COMPILED`, `SECURITY` or `REMARKS` has free-text contents. The lexer emits the name, `.`, one `CommentEntry` token (whitespace collapsed) and `.`. The text ends at the first `.` followed by whitespace or end of input. These words are ordinary identifiers elsewhere.
- Numeric literals are unsigned. `+` and `-` are always separate tokens, and the parser folds signs into literals.
- A `.` followed by a non-digit is a period token, so `1.` lexes as `1` then `.`. A `.` inside digits (`45.67`) is a decimal point.
- After `PIC`/`PICTURE` (and an optional `IS`, emitted as an identifier) the lexer emits one `PictureString` token, such as `S9(3)V99`. A trailing `.` is not part of it.
- Symbols: `. , ; ( ) + - * / ** = < > <= >=`.
- Tokens carry no Area A / Area B information. The parser can derive columns from spans and `SourceMap::line_col` if it needs them. Area warnings, if wanted, belong in a later pass, not the lexer.
- There is no end-of-file token.

`COPY` preprocessor:

- Runs before the lexer, as a separate module.
- Supports `COPY name.`, `COPY 'name'.`, `COPY name OF|IN lib.` (the library is ignored) and nested copybooks. Cycles are reported as errors.
- `COPY ... REPLACING`, `REPLACE` and compiler directives are not supported (`REPLACING` reports an error).
- Copybooks are searched in the source file's directory, then `-I` directories, trying the name as written, lowercase and uppercase, with extensions `""`, `.cpy`, `.cbl`, `.cob`, `.copy`. Names containing path separators or `..` are rejected.

CLI:

- `cobol-compiler lex <file> [-I dir]... [--debug-lines]`. Output is plain text, one token per line: `file:line:col<TAB>TOKEN`. There is no `--format` option. Diagnostics go to stderr, and the exit code is non-zero if any error occurred.
- `cobol-compiler parse <file> [-I dir]... [--debug-lines]`. Output is a pretty-printed AST. Diagnostics go to stderr.
- `cobol-compiler analyze <file> [-I dir]... [--debug-lines]`. Output is a pretty-printed AST followed by symbol table and diagnostics. Exit code is non-zero if semantic errors occur.

Semantic analysis:

- 8 semantic checks: identifier uniqueness, move compatibility, undefined references (PERFORM/GO TO), file association, picture validation, scope rules, type inference, numeric operations.
- Scopes: Global (DATA DIVISION variables and PROCEDURE DIVISION paragraph names), procedure-local (not yet implemented; for future phases).
- Data types: Numeric (PIC 9, S9, V, P), Alphabetic (PIC A), Alphanumeric (PIC X, N, or unknown).
- Error recovery: Collect all diagnostics and continue checking (like lexer and parser).
- Output: Annotated AST (original AST + symbol table). Per-node type annotations deferred to future phases.
- Module: `src/semantic/` with submodules: `analysis.rs` (main analyzer), `symbol_table.rs` (symbol management), `type_system.rs` (type inference and compatibility), `errors.rs` (diagnostic codes), `tests.rs` (unit tests).
- Golden tests: 13 semantic golden tests via `insta` snapshots (file: `tests/semantic_golden_tests.rs`); 6 focused programs use the `semantic_test_*.cob` prefix, and 7 additional tests exercise `prog001_divisoes.cob` through `prog007_loja.cob` in `cobol-examples/`.

## Gotchas

- **Use `jj` only** for all version control operations — this repository uses Jujutsu, not Git. Using `git` commands directly may cause inconsistencies between the working copy and the VCS state.
- **AST snapshots must be complete:** Every node (division, section, field, statement) must appear in snapshots, even empty ones. Never abbreviate or summarize; use `insta accept` to sync snapshots with actual output.
- The lexer emits no EOF token, and `+`/`-` are always separate tokens.
- Lexer warnings do not fail lexing (exit code 0); lexer errors do and also fail the golden test.
- A literal continuation pads the previous line to column 72; an unterminated string stops at end of line.
- Comment-entry text ends at the first `.` followed by whitespace, so `Enio B. Basso` would end early.
- Changing lexer output requires reviewing and updating `tests/lexer-goldens/` (there is no automatic regenerate mode yet).
- Diagnostic codes in use: `cobol::source::{invalid_indicator, bad_continuation}`, `cobol::lexer::{illegal_character, unterminated_string, trailing_hyphen, identifier_too_long, underscore_in_identifier, missing_picture}`, `cobol::preprocess::{copybook_error, recursive_copy, missing_name, missing_period, replacing_unsupported}`, `cobol::semantic::{identifier_duplicate, move_incompatible, undefined_identifier, undefined_paragraph, file_not_declared, invalid_picture, type_mismatch, numeric_operand_required}`.
