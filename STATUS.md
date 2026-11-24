# STATUS

Volatile project state. Rules and decisions are in [AGENTS.md](AGENTS.md); update this file when a phase starts or finishes.

| Phase | Status |
| --- | --- |
| 1. Lexer (with fixed-format reader and `COPY` preprocessor) | Done (101 unit tests + golden test over the 7 examples) |
| 2. Parser | Done (108 unit tests + 18 file-based golden tests using `insta`; full AST pretty-printing) |
| 3. Semantic analysis | Done (17 unit tests covering 8 semantic checks; CLI analyze subcommand added; 13 golden tests via `insta` snapshots) |
| 4. Code generation | Not started |

## Deliverables

- `typst-src/lexer-report.typ` (Portuguese Typst report source); its compiled PDF is `documents/lexer-report.pdf`. Conclusion written. Still placeholders for the user: authors, AI tools used, and any test/error outputs not yet pasted.
- **Parser Phase 2:** Complete AST with all divisions/sections/fields/statements. Golden tests use `insta` snapshots over real COBOL files in `cobol-examples/parser_test_*.cob`. 18 total golden tests (7 real-world examples + 11 inline-extracted tests). All 133 tests pass.
- **Phase 3 — Semantic Analysis:** Complete implementation of 8 semantic checks (identifier uniqueness, move compatibility, undefined references, file association, picture validation, scope rules, type inference, numeric operations). Module: `src/semantic/`. CLI: `cobol-compiler analyze <file> [-I dir]...`. 17 unit tests + 13 golden tests via `insta` snapshots (6 focused `semantic_test_*.cob` programs and 7 existing `prog001`–`prog007` examples in `cobol-examples/`), error recovery enabled. Total test count: 163 tests (125 lib + 7 lexer golden + 18 parser golden + 13 semantic golden).

## Next steps

- Code generation (phase 4): The spec allows two backends (translate to C, or emit bytecode or machine code). No choice has been made — ask before starting.
- Report: Parser and Semantic Analysis reports need to be written in Typst and compiled to PDF.

## Open questions

(None currently.)
