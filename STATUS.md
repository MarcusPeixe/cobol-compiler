# STATUS

Volatile project state. Rules and decisions are in [AGENTS.md](AGENTS.md); update this file when a phase starts or finishes.

| Phase | Status |
| --- | --- |
| 1. Lexer (with fixed-format reader and `COPY` preprocessor) | Done (101 unit tests + golden test over the 7 examples) |
| 2. Parser | Done (108 unit tests + 26 file-based golden tests using `insta`; full AST pretty-printing; all `prog001`–`prog007` parse cleanly) |
| 3. Semantic analysis | Done (20 unit tests covering 8 semantic checks; CLI analyze subcommand added; 17 golden tests via `insta` snapshots) |
| 4. Code generation | C11 backend implemented for `prog001`–`prog007`; additional forms remain |

## Deliverables

- `typst-src/lexer-report.typ` (Portuguese Typst report source); its compiled PDF is `documents/lexer-report.pdf`. Conclusion written. Still placeholders for the user: authors, AI tools used, and any test/error outputs not yet pasted.
- **Parser Phase 2:** Complete AST with all divisions/sections/fields/statements. Golden tests use `insta` snapshots over real COBOL files in `cobol-examples/parser_test_*.cob` and seven professor examples. Parsing covers `VALUE(S)` initialization, qualified identifiers, arithmetic statements (`ADD`, `SUBTRACT`, `MULTIPLY`, `DIVIDE`), `PERFORM ... UNTIL`, `COMPUTE`, intrinsic function calls, and abbreviated relational conditions. Recovery reports unsupported or malformed syntax with source spans and continues at later divisions, declarations, paragraphs, and statements; `prog001`–`prog007` all parse with no errors (file I/O, OCCURS/SEARCH, INSPECT, EVALUATE, subscripts are implemented), and `parser_test_unsupported_recovery.cob` covers the recovery path. 26 total golden tests (7 real-world examples + 19 focused tests).
- **Phase 3 — Semantic Analysis:** Complete implementation of 8 semantic checks (identifier uniqueness, move compatibility, undefined references, file association, picture validation, scope rules, type inference, numeric operations), including qualified data-name resolution and ambiguity diagnostics. Module: `src/semantic/`. CLI: `cobol-compiler analyze <file> [-I dir]...`. 23 unit tests + 17 golden tests via `insta` snapshots (10 focused `semantic_test_*.cob` programs and 7 existing `prog001`–`prog007` examples in `cobol-examples/`), error recovery enabled. Semantic snapshots use the complete parser AST formatter and render symbol kinds and qualifiers.
- **Phase 4 — Code Generation:** `src/codegen/` emits standalone C11 with an embedded runtime; `codegen` writes to stdout or `-o`. All seven examples compile and run in the codegen integration tests, including fresh and reopened `prog007` files. Exact decimal arithmetic, fractional powers, tables, string inspection and fixed-record file I/O cover the examples. See [AGENTS.md](AGENTS.md) for precision limits, the project-specific indexed-file format, and unsupported forms.

## Next steps

- Extend the C backend beyond the seven examples, prioritizing the unsupported forms listed in [AGENTS.md](AGENTS.md).
- Report: Parser and Semantic Analysis reports need to be written in Typst and compiled to PDF.

## Open questions

- Semantic checks still skip: `INSPECT`/`SET` operand types, READ key fields, subscript index values vs `OCCURS` bounds, `READ ... INTO` compatibility, and level-88/`REDEFINES` (not parsed).
- Remaining parser gaps (not needed by `prog001`–`prog007`): `EVALUATE ... ALSO`, `PERFORM ... AFTER`, `PERFORM THRU` with TIMES/UNTIL/VARYING, `A(i)(a:b)`, `CALL`, `STRING`/`UNSTRING`.
- Phase 4 currently handles only a subset of parsed COBOL; unsupported forms produce diagnostics instead of partial generated C.
