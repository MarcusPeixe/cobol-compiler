# STATUS

Volatile project state. Rules and decisions are in [AGENTS.md](AGENTS.md); update this file when a phase starts or finishes.

| Phase | Status |
| --- | --- |
| 1. Lexer (with fixed-format reader and `COPY` preprocessor) | Done (101 unit tests + golden test over the 7 examples) |
| 2. Parser | Done (108 unit tests + 26 file-based golden tests using `insta`; full AST pretty-printing; all `prog001`–`prog007` parse cleanly) |
| 3. Semantic analysis | Done (17 unit tests covering 8 semantic checks; CLI analyze subcommand added; 13 golden tests via `insta` snapshots) |
| 4. Code generation | Not started |

## Deliverables

- `typst-src/lexer-report.typ` (Portuguese Typst report source); its compiled PDF is `documents/lexer-report.pdf`. Conclusion written. Still placeholders for the user: authors, AI tools used, and any test/error outputs not yet pasted.
- **Parser Phase 2:** Complete AST with all divisions/sections/fields/statements. Golden tests use `insta` snapshots over real COBOL files in `cobol-examples/parser_test_*.cob` and seven professor examples. Parsing covers `VALUE(S)` initialization, qualified identifiers, arithmetic statements (`ADD`, `SUBTRACT`, `MULTIPLY`, `DIVIDE`), `PERFORM ... UNTIL`, `COMPUTE`, intrinsic function calls, and abbreviated relational conditions. Recovery reports unsupported or malformed syntax with source spans and continues at later divisions, declarations, paragraphs, and statements; `prog001`–`prog007` all parse with no errors (file I/O, OCCURS/SEARCH, INSPECT, EVALUATE, subscripts are implemented), and `parser_test_unsupported_recovery.cob` covers the recovery path. 26 total golden tests (7 real-world examples + 19 focused tests).
- **Phase 3 — Semantic Analysis:** Complete implementation of 8 semantic checks (identifier uniqueness, move compatibility, undefined references, file association, picture validation, scope rules, type inference, numeric operations). Module: `src/semantic/`. CLI: `cobol-compiler analyze <file> [-I dir]...`. 17 unit tests + 13 golden tests via `insta` snapshots (6 focused `semantic_test_*.cob` programs and 7 existing `prog001`–`prog007` examples in `cobol-examples/`), error recovery enabled. Total test count: 171 tests (125 lib + 7 lexer golden + 26 parser golden + 13 semantic golden).

## Next steps

- Code generation (phase 4): backend decided — MIPS assembly for the MARS simulator (`Mars` in the dev shell). Not started.
- Report: Parser and Semantic Analysis reports need to be written in Typst and compiled to PDF.

## Open questions

- Semantic checks are shallow for the new constructs: `SEARCH`, `INSPECT`, `EVALUATE` and I/O statements only resolve identifiers and file names. They do not yet check types, key fields or OCCURS/subscript rules.
- Remaining parser gaps (not needed by `prog001`–`prog007`): `EVALUATE ... ALSO`, `PERFORM ... AFTER`, `PERFORM THRU` with TIMES/UNTIL/VARYING, `A(i)(a:b)`, `CALL`, `STRING`/`UNSTRING`.
- Phase 4 backend is MIPS assembly for MARS (`Mars` in the dev shell). Next: deepen semantics (types, OCCURS/subscripts, file records), compute storage layout (PIC sizes, group offsets, OCCURS strides), then codegen.
- The semantic golden printer is intentionally concise (e.g. `Move`, `Set`, `Inspect` print no details); full detail lives in the parser goldens.
