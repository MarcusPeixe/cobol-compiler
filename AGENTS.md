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
- Integration tests live in `tests/`; insta snapshots are under `tests/snapshots/`. Lexer goldens contain the token listing followed by a plain `DIAGNOSTICS:` section with deterministic warning records; errors fail the integration test.
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
src/parser/                 AST, parsing and complete AST formatter
src/semantic/               checks, symbol table and storage layout
src/codegen/                standalone C11 generation and embedded runtime
tests/                      integration tests
tests/snapshots/            insta golden output
```

Each directory above holds `mod.rs` plus `tests.rs` (except `src/lexer/`, which only re-exports).

```
misc/                        language spec, COBOL examples, and example.typ
documents/                   compiled PDF reports
typst-src/                   Typst report source files (except example.typ)
cobol-examples/             the professor's example programs (git subtree); they target GnuCOBOL, not strictly the spec
```

Pipeline: source file -> fixed-format reader -> `COPY` preprocessor -> lexer -> parser -> semantic analysis -> standalone C11.

## Conventions

- **Errors:** `thiserror` for typed errors in library code, `anyhow` only in `main.rs`, `miette` for rendering. Every compiler message is a `diagnostics::Diagnostic` with a stable code like `cobol::lexer::illegal_character`.
- **Error recovery:** phases collect all diagnostics and keep going rather than stopping at the first one.
- **Spans:** `Span` is a byte range in an original file (`FileId` into `SourceMap`). Text from copybooks keeps spans pointing into the copybook. `LogicalText` carries a per-byte origin map so spans survive column stripping and `COPY` expansion.
- **Integration tests & golden testing:** "Integration tests" always refers to golden testing using the `insta` crate. All new compiler phases should include golden tests. See existing test files in `tests/` for patterns.
- **File edits:** make source changes with the `edit`/`create` tools, not scripts (Python, `sed`) run through bash. The tools are faster and safer and avoid a permission prompt per command; use bash only to run builds and tests.
- **Fixing example failures:** if a `cobol-examples/prog00X` program produces an error diagnostic, treat it as a missing feature. Implement valid COBOL even when it goes beyond the spec (in the parser, AST, semantic analyzer and `src/parser/printer.rs`) instead of accepting the error in a snapshot. Allow-errors goldens (`test_parser_golden_with_errors`) are only for fixtures that deliberately test recovery.
- Changing `src/parser/printer.rs` or the AST changes every parser golden: review each `.snap.new` diff before `cargo insta accept`.
- **Test program placement:** Never write compiler test programs inline in Rust source code. Always create separate `.cob` files in `cobol-examples/` with stage-specific prefixes: `lexer_test_`, `parser_test_`, `semantic_test_`, or `codegen_test_`. This makes tests maintainable and allows programs to be reused as examples.
- **Output formatting for integration tests:** When implementing a new compiler phase, design the output format with integration testing in mind. Maximize information (all nodes, fields, all details) to aid inspection and debugging. The output is compiler-internal, not user-facing, so prioritize completeness over pretty printing.
- **Snapshot testing:** Golden tests use `insta` for snapshot assertions. When output changes intentionally, use `cargo insta test` to review and accept new snapshots.
- **Parser error snapshots:** Parser goldens normally fail when parser errors occur. Allow errors only in fixtures intentionally exercising unsupported syntax, and snapshot their diagnostics as well as the recovered AST.
- **AST pretty-printing:** Print every node in the AST tree (all divisions, sections, fields, statements), including empty ones. Never count statements; always list them individually. This ensures snapshots catch structural changes.
- **Test file naming:** Test programs follow the convention `{PHASE}_test_{TEST_NAME}.cob` where `PHASE` is `lexer`, `parser`, `semantic`, or `codegen` and `TEST_NAME` matches the test function name (e.g., `parser_test_minimal_program.cob` for `test_minimal_program()`).

## Decisions

Lexer and source handling:

- Fixed format only (no free format). Columns 1-6 and 73+ are ignored, column 7 is the indicator.
- Comment lines (`*`, `/`) are dropped. `D` lines are skipped unless `--debug-lines` is given. Continuation lines (`-`) are joined for string literals (the previous line is padded to column 72) and for words split across lines (no quote after the `-`: the text glues directly onto the previous line, e.g. `T` + `-AB-X` becomes `TAB-X`).
- Case-insensitive: keywords and identifiers are uppercased in tokens. The original text stays available through the span.
- Only the 14 keywords from spec section 2.2 are reserved. Everything else (`PROGRAM-ID`, `VALUE`, `TO`, `AND`, `END-IF`...) is an identifier. The parser decides what they mean. `PIC` and `PICTURE` are special-cased.
- Identifiers: 1-30 characters of A-Z, 0-9 and `-`, not ending with a hyphen. Longer or hyphen-terminated words are errors.
- `_` is also accepted inside identifiers (but cannot start one) with a lexer warning `cobol::lexer::underscore_in_identifier`. This deviates from the spec; it exists because the professor's examples in `cobol-examples/` use names like `PROG008_STRING`.
- Comment-entry paragraphs: inside the IDENTIFICATION DIVISION, a paragraph named `AUTHOR`, `INSTALLATION`, `DATE-WRITTEN`, `DATE-COMPILED`, `SECURITY` or `REMARKS` has free-text contents. The lexer emits the name, `.`, one `CommentEntry` token (whitespace collapsed) and `.`. The text ends at the first `.` followed by whitespace or end of input. These words are ordinary identifiers elsewhere.
- Numeric literals are unsigned. `+` and `-` are always separate tokens, and the parser folds signs into literals.
- A `.` followed by a non-digit is a period token, so `1.` lexes as `1` then `.`. A `.` inside digits (`45.67`) is a decimal point.
- After `PIC`/`PICTURE` (and an optional `IS`, emitted as an identifier) the lexer emits one `PictureString` token, such as `S9(3)V99`. A trailing `.` is not part of it.
- Symbols: `. , ; : ( ) + - * / ** = < > <= >=` (`:` is used by reference modification).
- Tokens carry no Area A / Area B information. The parser can derive columns from spans and `SourceMap::line_col` if it needs them. Area warnings, if wanted, belong in a later pass, not the lexer.
- There is no end-of-file token.

`COPY` preprocessor:

- Runs before the lexer, as a separate module.
- Supports `COPY name.`, `COPY 'name'.`, `COPY name OF|IN lib.` (the library is ignored) and nested copybooks. Cycles are reported as errors.
- `COPY ... REPLACING`, `REPLACE` and compiler directives are not supported (`REPLACING` reports an error).
- Copybooks are searched in the source file's directory, then `-I` directories, trying the name as written, lowercase and uppercase, with extensions `""`, `.cpy`, `.cbl`, `.cob`, `.copy`. Names containing path separators or `..` are rejected.

Parser:

- Working-storage entries accept `VALUE` and `VALUES` followed by numeric/string literals or common figurative constants (`ZERO(S)`, `ZEROES`, `SPACE(S)`, `HIGH-VALUE(S)`, `LOW-VALUE(S)`, `QUOTE(S)`, `NULL(S)`).
- Arithmetic statements support `ADD ... TO|GIVING`, `SUBTRACT ... FROM [GIVING ...]`, `MULTIPLY ... BY [GIVING ...]`, and `DIVIDE ... BY ... [GIVING ...] [REMAINDER ...]`, including `ON SIZE ERROR` statement bodies.
- Procedure statements support `PERFORM ... UNTIL`, `COMPUTE target = expression`, intrinsic `FUNCTION name(arguments)` expressions, and abbreviated relational conditions such as `item = "A" OR "B"`.
- Qualified data references using `OF` or `IN` are retained in the identifier expression.
- **Error recovery:** Unsupported or malformed syntax produces a source-spanned `cobol::parser::unsupported_syntax` or `cobol::parser::parse_error` diagnostic. Recovery synchronizes at later divisions, `WORKING-STORAGE`, statement starts, sentence periods, and control-flow boundaries so later declarations and statements can still be parsed. Skipped syntax is diagnosed, not represented as successfully parsed AST nodes. All `prog001`–`prog007` examples parse without errors; the recovery path is exercised by `parser_test_unsupported_recovery.cob` (uses `CALL ... USING` and `UNSTRING`, which are unsupported; update the fixture if either gets implemented).
- Beyond the spec, valid COBOL used by the examples is implemented: `FILE-CONTROL`/`SELECT` (ASSIGN, ORGANIZATION, ACCESS MODE, RECORD/ALTERNATE KEY, FILE STATUS), FD with `BLOCK`/`RECORD CONTAINS` and record layouts, `OCCURS n [TIMES]`, `INDEXED BY`, subscripts `NAME(i[, j])`, figurative constants as expressions, `DISPLAY ... WITH NO ADVANCING`, and the statements `SET`, `SEARCH` (AT END/WHEN), `INSPECT` (TALLYING/REPLACING/CONVERTING with BEFORE/AFTER), `EVALUATE` (subject value or `TRUE`/`FALSE`, stacked `WHEN`s, `THRU` ranges, `ANY`, `WHEN OTHER`; `EvaluateChoice` in the AST), `SEARCH ALL`, `INITIALIZE`, reference modification `NAME(start:[length])` (`Expression::RefMod`), `WRITE ... BEFORE|AFTER ADVANCING n LINES|PAGE`, `OPEN`, `CLOSE`, `READ`, `WRITE`, `START` with AT END / INVALID KEY handlers. The AST retains `WITH DUPLICATES` on alternate keys and `WITH TEST BEFORE|AFTER` on applicable `PERFORM` forms. `PERFORM` supports the plain form, `THRU`/`THROUGH`, `n TIMES`, `UNTIL`, `VARYING ... FROM ... BY ... UNTIL` (paragraph or inline) and inline bodies with `END-PERFORM`. Not supported: `EVALUATE ... ALSO`, `PERFORM ... AFTER` (nested varying), `PERFORM THRU` combined with TIMES/UNTIL/VARYING, `A(i)(a:b)`.
- The AST dump lives in `src/parser/printer.rs` (`format_program`), shared by the CLI and the parser golden tests.
- Edited pictures (`Z`, `*`, `$`, `+`, `-`, `B`, `0`, `/`, `,`, `.`) are accepted by the semantic picture check; `Z` makes the item numeric.

Code generation:

- **Target:** `src/codegen/` lowers the analyzed AST directly to one standalone C11 translation unit, embedding `runtime.c`; no separate runtime or math library is needed. `generate(&AnalyzedProgram)` returns C or source-spanned `cobol::codegen::unsupported` diagnostics. The seven examples are supported, but this is not yet a complete COBOL backend.
- **Storage and decimal arithmetic:** fixed-width DISPLAY bytes use the semantic layout. Signed negatives use `p` through `y` as overpunched last digits; signed DISPLAY prints a leading sign. Basic arithmetic uses exact decimal rational intermediates, truncating to the receiving picture. `ON SIZE ERROR` retains the affected destination on overflow or division by zero. Fractional powers use integer root comparisons at the receiving scale; intermediate precision is limited to 2,048 decimal digits, and invalid domains or insufficient precision terminate explicitly.
- **Pictures and control flow:** supported numeric editing uses `9`, leading `Z` or `*`, one decimal point or `V`, and at most one fixed leading or trailing sign. Paragraph `PERFORM` and `PERFORM ... UNTIL` (including `WITH TEST AFTER`), `IF`, value-subject `EVALUATE`, `SEARCH`, and the example `INSPECT` forms are lowered directly; unimplemented variants are diagnosed.
- **Files:** sequential and indexed files contain raw fixed-length records, without delimiters or indexes in the file. Indexed lookup scans records; primary and alternate keys must be unique. This project-specific format is not GnuCOBOL-compatible. Indexed files require DYNAMIC or RANDOM access; keyed `READ` and `START ... KEY EQUAL` with DYNAMIC access are supported, not indexed sequential reads, duplicate alternate keys, or indexed `OPEN EXTEND`. Missing INPUT/I-O files return status 35 when `ENOENT` is available; otherwise open failures return 30. Unhandled file errors terminate explicitly.
- **Other limitations:** `GO TO`, `PERFORM THRU`/`TIMES`/`VARYING`/inline, `SEARCH ALL`/`VARYING`, `INITIALIZE`, reference modification, `EVALUATE TRUE/FALSE`, `WRITE ADVANCING`, PICTURE `P` or complex editing, nested exponentiation, text-to-numeric MOVE, computed numeric-to-text MOVE, and intrinsic functions other than `UPPER-CASE` are not generated.
- **Tests:** `tests/codegen_golden_tests.rs` snapshots complete generated C and byte-escaped output, compiles with `cc -std=c11 -pedantic-errors -Wall -Wextra -Werror`, and executes all seven examples in isolated directories. `cc` is required; tests do not skip when it is missing. The `prog007` fixture reopens new product files in I-O mode and reopens its key file for OUTPUT before persisting the ID.

CLI:

- `cobol-compiler lex <file> [-I dir]... [--debug-lines]`. Output is plain text, one token per line: `file:line:col<TAB>TOKEN`. There is no `--format` option. Diagnostics go to stderr, and the exit code is non-zero if any error occurred.
- `cobol-compiler parse <file> [-I dir]... [--debug-lines]`. Output is a pretty-printed AST. Diagnostics go to stderr.
- `cobol-compiler analyze <file> [-I dir]... [--debug-lines]`. Output is a pretty-printed AST followed by symbol table and diagnostics. Exit code is non-zero if semantic errors occur.
- `cobol-compiler codegen <file> [-I dir]... [--debug-lines] [-o file]`. C is written to stdout unless `-o` is provided. Warnings go to stderr; any compiler error suppresses C output and leaves the output file untouched. Output cannot refer to the input file, including through a hard link.

Semantic analysis:

- 8 semantic checks: identifier uniqueness, move compatibility, undefined references (PERFORM/GO TO), file association, picture validation, scope rules, type inference, numeric operations.
- Scopes: Global (DATA DIVISION variables and PROCEDURE DIVISION paragraph names), procedure-local (not yet implemented; for future phases).
- Data types: Numeric (PIC 9, S9, V, P), Alphabetic (PIC A), Alphanumeric (PIC X, N, or unknown).
- Qualified data references resolve their `OF`/`IN` chain. The same data name may be declared under distinct containing groups; references that remain ambiguous must be qualified. Index names, paragraphs, files, and data items have distinct symbol kinds.
- Error recovery: Collect all diagnostics and continue checking (like lexer and parser).
- Output: The semantic golden dump uses the complete parser AST formatter, followed by a symbol table that includes symbol kinds and data-name qualifiers, storage layout, and diagnostics. Per-node type annotations are deferred to future phases.
- Deeper checks: subscripts only on items inside an `OCCURS` table with the matching number of subscripts; `SEARCH` needs an `OCCURS` table; `WRITE` needs an 01 record of an FD (and `FROM` must be move-compatible); `VALUE` must match the PICTURE category and fit; group items cannot have `PICTURE`, elementary items need one; `OCCURS` count must be a positive integer; `EVALUATE` choices must be comparable with the subject.
- Storage layout (`src/semantic/layout/`, `AnalyzedProgram::layout`): every item is DISPLAY, one byte per picture position (`S`, `V`, `P` take no space; edited symbols take one byte). Working-storage items are laid out contiguously from offset 0; groups sum their children; `OCCURS` multiplies size, and the stride equals the element size. Each FD's records share one buffer at offset 0 (`record_size` = largest). `analyze` prints it under `STORAGE LAYOUT:` (`format_layout`); semantic goldens include it.
- Module: `src/semantic/` with submodules: `analysis.rs` (main analyzer), `layout/` (picture sizes and storage layout), `symbol_table.rs` (symbol management), `type_system.rs` (type inference and compatibility), `errors.rs` (diagnostic codes), `tests.rs` (unit tests).
- Golden tests: 17 semantic golden tests via `insta` snapshots (file: `tests/semantic_golden_tests.rs`); 10 focused programs use the `semantic_test_*.cob` prefix, and 7 additional tests exercise `prog001_divisoes.cob` through `prog007_loja.cob` in `cobol-examples/`.

## Gotchas

- **Use `jj` only** for all version control operations — this repository uses Jujutsu, not Git. Using `git` commands directly may cause inconsistencies between the working copy and the VCS state.
- **AST snapshots must be complete:** Every node (division, section, field, statement) must appear in snapshots, even empty ones. Never abbreviate or summarize; use `insta accept` to sync snapshots with actual output.
- The lexer emits no EOF token, and `+`/`-` are always separate tokens.
- Lexer warnings do not fail lexing (exit code 0); lexer errors do and also fail the golden test.
- A literal continuation pads the previous line to column 72; an unterminated string stops at end of line.
- Comment-entry text ends at the first `.` followed by whitespace, so `Enio B. Basso` would end early.
- Changing lexer output requires reviewing the corresponding insta snapshots in `tests/snapshots/`.
- Diagnostic codes in use: `cobol::source::{invalid_indicator, bad_continuation}`, `cobol::lexer::{illegal_character, unterminated_string, trailing_hyphen, identifier_too_long, underscore_in_identifier, missing_picture}`, `cobol::preprocess::{copybook_error, recursive_copy, missing_name, missing_period, replacing_unsupported}`, `cobol::parser::{parse_error, unsupported_syntax}`, `cobol::codegen::unsupported`, `cobol::semantic::{identifier_duplicate, ambiguous_identifier, move_incompatible, undefined_identifier, undefined_paragraph, file_not_declared, invalid_picture, type_mismatch, numeric_operand_required, subscript_on_non_table, subscript_count, invalid_occurs, value_type_mismatch, value_too_long, group_with_picture, missing_picture, not_a_record, search_requires_table}`.
