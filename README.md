# COBOL Compiler

A COBOL compiler written in Rust, built phase by phase. Currently only the first phase is available: the **lexer**, which reads fixed-format COBOL and prints its tokens. The language it follows is described in [documents/EspecificacaoCOBOL.pdf](documents/EspecificacaoCOBOL.pdf).

## Setup

The Rust toolchain is provided by the Nix flake:

```sh
nix develop
```

## Usage

```sh
cargo run -- lex <FILE> [-I <DIR>]... [--debug-lines]
```

| Option | Description |
| --- | --- |
| `-I`, `--include <DIR>` | Extra directory to search for `COPY` books. Can be repeated. The source file's own directory is always searched first. |
| `--debug-lines` | Compile lines with `D` in column 7 instead of skipping them. |

Try it on the included example:

```sh
cargo run -- lex documents/example.cob
```

Each token is printed on its own line as `file:line:column  TOKEN`:

```
documents/example.cob:2:8	IDENTIFIER PROGRAM-ID
documents/example.cob:11:20	STRING "HELLO, "
```

Errors are shown on stderr with the offending source highlighted, and the exit code is non-zero if any occurred.

## Source format

Source files must be in COBOL fixed format:

| Columns | Meaning |
| --- | --- |
| 1-6 | Sequence numbers (ignored) |
| 7 | Indicator: `*` or `/` comment, `-` continuation, `D` debug line |
| 8-72 | Code |
| 73+ | Ignored |

`COPY name.` is expanded before lexing. `COPY ... REPLACING` is not supported yet.

## Tests

```sh
cargo test
```
