# STATUS

Volatile project state. Rules and decisions are in [AGENTS.md](AGENTS.md); update this file when a phase starts or finishes.

| Phase | Status |
| --- | --- |
| 1. Lexer (with fixed-format reader and `COPY` preprocessor) | Done (101 unit tests + golden test over the 7 examples) |
| 2. Parser | Not started |
| 3. Semantic analysis | Not started |
| 4. Code generation | Not started |

## Deliverables

- `documents/lexer-report.typ` (Portuguese Typst report). Conclusion written. Still placeholders for the user: authors, AI tools used, and any test/error outputs not yet pasted. Not compiled locally.

## Next steps

- Parser: the lexer output is a `Vec<Token>` plus diagnostics. The parser handles context-sensitive words itself (`PROGRAM-ID`, `PIC`, `VALUE`, `OF`, level numbers, paragraph names) and folds signs into numeric literals.
- Semantic checks are listed in spec section 4 (MOVE compatibility, unique identifiers, `PERFORM`/`GO TO` targets, file association).
- The spec allows two backends (translate to C, or emit bytecode or machine code). No choice has been made, so ask before starting phase 4.

## Open questions

- Adopt `insta` for snapshot testing once parser/AST outputs get large (recommended, not decided).
