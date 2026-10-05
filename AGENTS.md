# AGENTS.md

Guidance for people and AI agents working on this repository. Keep this file up to date: when you make a decision or learn something others need, add it here.

## Project

A COBOL compiler in Rust, built in phases. Each phase is its own module with comprehensive unit tests. The language spec is [documents/EspecificacaoCOBOL.pdf](documents/EspecificacaoCOBOL.pdf) (Portuguese). For usage see [README.md](README.md).

| Phase | Status |
| --- | --- |
| 1. Lexer (with fixed-format reader and `COPY` preprocessor) | Done |
| 2. Parser | Not started |
| 3. Semantic analysis | Not started |
| 4. Code generation | Not started |

## Workflow

- Cargo is only available inside the Nix dev shell. Run commands as `nix develop --command cargo <cmd>`, or enter `nix develop` first.
- Before finishing a change: `cargo fmt`, `cargo clippy --all-targets` (keep it warning-free) and `cargo test`.
- Every new module needs unit tests in a `#[cfg(test)] mod tests` block in the same file.
- Library crate (`src/lib.rs`) plus a thin binary (`src/main.rs`). New phases get their own module and their own CLI subcommand (`cobol-compiler lex <file>`, and so on).

## Layout

```
src/main.rs         CLI (clap); anyhow is used only here
src/lib.rs          module declarations
src/diagnostics.rs  shared Severity (I/W/E/S), Diagnostic and miette integration
src/source/         SourceMap, Span, LogicalText and the fixed-format reader
src/preprocess.rs   COPY expansion (runs before the lexer)
src/lexer/          token.rs (token types), scanner.rs (lexing)
documents/          spec PDF and example.cob
```

Pipeline: source file -> fixed-format reader -> `COPY` preprocessor -> lexer -> tokens.

## Conventions

- **Errors:** `thiserror` for typed errors in library code, `anyhow` only in `main.rs`, `miette` for rendering. Every compiler message is a `diagnostics::Diagnostic` with a stable code like `cobol::lexer::illegal_character`.
- **Error recovery:** phases collect all diagnostics and keep going rather than stopping at the first one.
- **Spans:** `Span` is a byte range in an original file (`FileId` into `SourceMap`). Text from copybooks keeps spans pointing into the copybook. `LogicalText` carries a per-byte origin map so spans survive column stripping and `COPY` expansion.

## Decisions

Lexer and source handling:

- Fixed format only (no free format). Columns 1-6 and 73+ are ignored, column 7 is the indicator.
- Comment lines (`*`, `/`) are dropped. `D` lines are skipped unless `--debug-lines` is given. Continuation lines (`-`) are joined only for string literals, and the previous line is padded to column 72.
- Case-insensitive: keywords and identifiers are uppercased in tokens. The original text stays available through the span.
- Only the 14 keywords from spec section 2.2 are reserved. Everything else (`PROGRAM-ID`, `VALUE`, `TO`, `AND`, `END-IF`...) is an identifier. The parser decides what they mean. `PIC` and `PICTURE` are special-cased.
- Identifiers: 1-30 characters of A-Z, 0-9 and `-`, not ending with a hyphen. Longer or hyphen-terminated words are errors.
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

## Notes for the next phases

- The lexer output is a `Vec<Token>` plus diagnostics. The parser should handle context-sensitive words (`PROGRAM-ID`, `PIC`, `VALUE`, `OF`, level numbers, paragraph names) itself.
- Semantic checks are listed in spec section 4 (MOVE compatibility, unique identifiers, `PERFORM`/`GO TO` targets, file association).
- The spec allows two backends (translate to C, or emit bytecode or machine code). No choice has been made yet, so ask before starting phase 4.
