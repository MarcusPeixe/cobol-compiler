---
applyTo: "misc/*.typ,typst-src/*.typ"
---

# Typst report rules

- The report is written in Portuguese and follows the requirements in `misc/instructions.md`.
- Follow the `lambda-notes` template used in `misc/example.typ` (`#callout`, `#quote`, `#figure(table(...))`, fenced code blocks, `=`/`==` headings).
- `typst` is not installed locally, so the document cannot be compiled here; say so instead of claiming it renders.
- Cite decisions from `AGENTS.md` instead of redefining them.
- Leave user-owned placeholders (authors, AI tools used, pasted test/error outputs) untouched unless asked.
- The file is large (2000+ lines of pasted outputs): use `grep -n` and narrow `view` ranges.
- Repository URL: https://github.com/MarcusPeixe/cobol-compiler/tree/lexer
