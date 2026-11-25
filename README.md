# COBOL Compiler

A COBOL compiler written in Rust, built in phases. The language specification is [misc/EspecificacaoCOBOL.pdf](misc/EspecificacaoCOBOL.pdf).

## Status

| Phase | Status |
| --- | --- |
| Fixed-format source reader, `COPY` preprocessor, and lexer | Complete |
| Parser | Complete |
| Semantic analysis | Complete |
| Code generation | Not started |

The available pipeline is: fixed-format source reader -> `COPY` expansion -> lexer -> parser -> semantic analysis.
Code generation has not been implemented yet.

## Setup

The Rust toolchain is provided by the Nix flake:

```sh
nix develop
```

## Usage

```sh
cargo run -- lex <FILE> [-I <DIR>]... [--debug-lines]
cargo run -- parse <FILE> [-I <DIR>]... [--debug-lines]
cargo run -- analyze <FILE> [-I <DIR>]... [--debug-lines]
```

| Option | Description |
| --- | --- |
| `-I`, `--include <DIR>` | Extra directory to search for `COPY` books. Can be repeated. The source file's own directory is always searched first. |
| `--debug-lines` | Compile lines with `D` in column 7 instead of skipping them. |

The subcommands are:

- `lex`: preprocesses and tokenizes a fixed-format source file, printing one token per line.
- `parse`: parses the source and prints a complete pretty-printed AST.
- `analyze`: parses the source and prints the AST, symbol table, and diagnostics.

Try the lexer on the included example:

```sh
cargo run -- lex misc/example.cob
```

Lex output is printed as `file:line:column  TOKEN`:

```
misc/example.cob:2:8	IDENTIFIER PROGRAM-ID
misc/example.cob:11:20	STRING "HELLO, "
```

Compiler diagnostics are shown on stderr with the offending source highlighted. Commands exit with a non-zero status if an error occurs.

## Parser

The parser builds a complete AST containing all divisions, sections, fields, paragraphs, and statements, including empty nodes. It supports the implemented phase syntax, including `PERFORM ... UNTIL`, `COMPUTE` expressions, intrinsic `FUNCTION` calls, and abbreviated relational conditions.

Golden tests use [insta](https://insta.rs/) snapshots in `tests/parser_golden_tests.rs`. Test source programs follow the `parser_test_*.cob` naming convention in `cobol-examples/`.

## Semantic analysis

`analyze` runs eight semantic checks:

- Identifier uniqueness
- `MOVE` compatibility
- Undefined references for `PERFORM` and `GO TO`
- File association
- Picture validation
- Scope rules
- Type inference
- Numeric operations

The output includes the original AST and a symbol-table summary. Semantic diagnostics use the shared source-spanned diagnostic system, and analysis continues after errors.

Semantic golden tests use [insta](https://insta.rs/) snapshots in `tests/semantic_golden_tests.rs`. Focused fixtures use the `semantic_test_*.cob` naming convention in `cobol-examples/`.

## Source format

Source files must be in COBOL fixed format:

| Columns | Meaning |
| --- | --- |
| 1-6 | Sequence numbers (ignored) |
| 7 | Indicator: `*` or `/` comment, `-` continuation, `D` debug line |
| 8-72 | Code |
| 73+ | Ignored |

`COPY name.` is expanded before lexing. `COPY ... REPLACING` is not supported yet.

Report sources are in `typst-src/`, and compiled PDF reports are in `documents/`. The separate `misc/example.typ` file is a Typst example, not a report source. `typst` is not available in the local Nix shell, so reports must be compiled in an environment that provides it.

Project rules and decisions live in [AGENTS.md](AGENTS.md), while the current phase state and next steps live in [STATUS.md](STATUS.md).

## Tests

```sh
cargo test
cargo clippy --all-targets
cargo fmt --check
```

Before submitting a change, also run `cargo fmt`, `cargo clippy --all-targets` (warning-free), and `cargo test`.

Golden snapshot changes must be reviewed with the `insta` commands:

```sh
cargo insta test
cargo insta review
cargo insta accept
cargo insta reject
```
