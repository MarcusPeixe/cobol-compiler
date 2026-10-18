//! Textual `COPY name.` expansion, run on logical text before lexing.
//!
//! Supported: `COPY name.`, `COPY 'name'.`, `COPY name OF|IN library.` (library
//! ignored) and nested copybooks with cycle detection. `REPLACING` is reported
//! as unsupported.

use std::path::PathBuf;

use crate::diagnostics::Diagnostic;
use crate::source::{LogicalText, Origin, ReaderOptions, SourceMap, read_fixed_format};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Copybook {
    /// Unique identity used for cycle detection.
    pub id: String,
    /// Name shown in diagnostics.
    pub name: String,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LoadError {
    #[error("copybook '{0}' not found")]
    NotFound(String),
    #[error("invalid copybook name '{0}'")]
    InvalidName(String),
    #[error("cannot read copybook '{name}': {reason}")]
    Io { name: String, reason: String },
}

pub trait CopybookLoader {
    fn load(&self, name: &str) -> Result<Copybook, LoadError>;
}

/// Looks copybooks up in a list of directories, trying common extensions and
/// the name as written, lowercased and uppercased.
#[derive(Debug, Clone, Default)]
pub struct FsLoader {
    pub search_dirs: Vec<PathBuf>,
}

const EXTENSIONS: [&str; 5] = ["", ".cpy", ".cbl", ".cob", ".copy"];

impl CopybookLoader for FsLoader {
    fn load(&self, name: &str) -> Result<Copybook, LoadError> {
        let valid = !name.is_empty()
            && name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'.')
            && !name.contains("..");
        if !valid {
            return Err(LoadError::InvalidName(name.to_string()));
        }
        let variants = [name.to_string(), name.to_lowercase(), name.to_uppercase()];
        for dir in &self.search_dirs {
            for variant in &variants {
                for ext in EXTENSIONS {
                    let path = dir.join(format!("{variant}{ext}"));
                    if !path.is_file() {
                        continue;
                    }
                    let text = std::fs::read_to_string(&path).map_err(|e| LoadError::Io {
                        name: path.display().to_string(),
                        reason: e.to_string(),
                    })?;
                    let id = path.canonicalize().unwrap_or_else(|_| path.clone());
                    return Ok(Copybook {
                        id: id.display().to_string(),
                        name: path.display().to_string(),
                        text,
                    });
                }
            }
        }
        Err(LoadError::NotFound(name.to_string()))
    }
}

/// Reads the root file (already in `map`) and expands every `COPY` statement.
pub fn preprocess(
    map: &mut SourceMap,
    root: crate::source::FileId,
    options: ReaderOptions,
    loader: &dyn CopybookLoader,
) -> (LogicalText, Vec<Diagnostic>) {
    let (text, mut diagnostics) = read_fixed_format(map, root, options);
    let mut stack = vec![map.file(root).name.clone()];
    let mut expander = Expander {
        map,
        options,
        loader,
        diagnostics: &mut diagnostics,
    };
    let out = expander.expand(&text, &mut stack);
    (out, diagnostics)
}

struct Expander<'a> {
    map: &'a mut SourceMap,
    options: ReaderOptions,
    loader: &'a dyn CopybookLoader,
    diagnostics: &'a mut Vec<Diagnostic>,
}

fn is_word_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'-' || b == b'_'
}

fn skip_ws(b: &[u8], mut i: usize) -> usize {
    while i < b.len() && b[i].is_ascii_whitespace() {
        i += 1;
    }
    i
}

fn word_at(text: &str, i: usize) -> &str {
    let b = text.as_bytes();
    let mut j = i;
    while j < b.len() && is_word_byte(b[j]) {
        j += 1;
    }
    &text[i..j]
}

impl Expander<'_> {
    fn expand(&mut self, input: &LogicalText, stack: &mut Vec<String>) -> LogicalText {
        let text = input.text.as_str();
        let b = text.as_bytes();
        let mut out = LogicalText::default();
        let mut quote: Option<u8> = None;
        let mut i = 0;
        while i < b.len() {
            let c = b[i];
            if let Some(q) = quote {
                if c == q || c == b'\n' {
                    quote = None;
                }
            } else if c == b'\'' || c == b'"' {
                quote = Some(c);
            } else if is_word_byte(c) {
                let word = word_at(text, i);
                let end = i + word.len();
                if word.eq_ignore_ascii_case("COPY") {
                    i = self.copy_statement(input, i, &mut out, stack);
                } else {
                    out.extend_from(input, i..end);
                    i = end;
                }
                continue;
            }
            let len = text[i..].chars().next().map_or(1, char::len_utf8);
            out.extend_from(input, i..i + len);
            i += len;
        }
        out
    }

    /// Handles a `COPY` starting at `start`; returns the index after the statement.
    fn copy_statement(
        &mut self,
        input: &LogicalText,
        start: usize,
        out: &mut LogicalText,
        stack: &mut Vec<String>,
    ) -> usize {
        let text = input.text.as_str();
        let b = text.as_bytes();
        let mut i = skip_ws(b, start + 4);

        let name = match b.get(i) {
            Some(&q @ (b'\'' | b'"')) => {
                let close = text[i + 1..].find([q as char, '\n']).map(|p| i + 1 + p);
                match close {
                    Some(p) if b[p] == q => {
                        let n = text[i + 1..p].to_string();
                        i = p + 1;
                        n
                    }
                    _ => String::new(),
                }
            }
            _ => {
                let w = word_at(text, i);
                i += w.len();
                w.to_string()
            }
        };
        if name.is_empty() {
            self.diagnostics.push(
                Diagnostic::error(
                    "cobol::preprocess::missing_name",
                    "COPY requires a copybook name",
                )
                .with_span(input.span(start, start + 4))
                .with_label("expected a copybook name after COPY"),
            );
            return i.max(start + 4);
        }

        i = skip_ws(b, i);
        let qualifier = word_at(text, i);
        if qualifier.eq_ignore_ascii_case("OF") || qualifier.eq_ignore_ascii_case("IN") {
            i = skip_ws(b, i + qualifier.len());
            i += word_at(text, i).len();
            i = skip_ws(b, i);
        }

        if word_at(text, i).eq_ignore_ascii_case("REPLACING") {
            let end = text[i..].find('.').map_or(b.len(), |p| i + p + 1);
            self.diagnostics.push(
                Diagnostic::error(
                    "cobol::preprocess::replacing_unsupported",
                    "COPY ... REPLACING is not supported",
                )
                .with_span(input.span(start, end))
                .with_label("statement skipped"),
            );
            return end;
        }

        if b.get(i) != Some(&b'.') {
            self.diagnostics.push(
                Diagnostic::error(
                    "cobol::preprocess::missing_period",
                    "COPY statement must end with '.'",
                )
                .with_span(input.span(start, i.max(start + 4)))
                .with_label("expected '.' after the copybook name"),
            );
            return i;
        }
        let end = i + 1;
        let stmt_span = input.span(start, end);

        let copybook = match self.loader.load(&name) {
            Ok(c) => c,
            Err(e) => {
                let mut d = Diagnostic::error("cobol::preprocess::copybook_error", e.to_string())
                    .with_span(stmt_span)
                    .with_label("while expanding this COPY");
                if matches!(e, LoadError::NotFound(_)) {
                    d = d.with_help("add a search directory with -I <dir>");
                }
                self.diagnostics.push(d);
                return end;
            }
        };
        if stack.contains(&copybook.id) {
            self.diagnostics.push(
                Diagnostic::error(
                    "cobol::preprocess::recursive_copy",
                    format!("recursive COPY of '{}'", copybook.name),
                )
                .with_span(stmt_span)
                .with_label("copybook includes itself"),
            );
            return end;
        }

        let file = self.map.add_file(copybook.name, copybook.text);
        let (logical, diags) = read_fixed_format(self.map, file, self.options);
        self.diagnostics.extend(diags);
        stack.push(copybook.id);
        let expanded = self.expand(&logical, stack);
        stack.pop();

        let origin: Origin = input.origins[start];
        out.push_synthetic('\n', origin);
        out.extend_from(&expanded, 0..expanded.text.len());
        out.push_synthetic('\n', origin);
        end
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    struct MemLoader(HashMap<&'static str, &'static str>);

    impl CopybookLoader for MemLoader {
        fn load(&self, name: &str) -> Result<Copybook, LoadError> {
            let key = name.to_uppercase();
            self.0
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(&key))
                .map(|(k, v)| Copybook {
                    id: k.to_string(),
                    name: format!("{k}.cpy"),
                    text: v.to_string(),
                })
                .ok_or_else(|| LoadError::NotFound(name.to_string()))
        }
    }

    fn run(
        src: &str,
        books: &[(&'static str, &'static str)],
    ) -> (String, Vec<Diagnostic>, SourceMap, LogicalText) {
        let mut map = SourceMap::default();
        let root = map.add_file("main.cob", src.to_string());
        let loader = MemLoader(books.iter().copied().collect());
        let (text, diags) = preprocess(&mut map, root, ReaderOptions::default(), &loader);
        (text.text.clone(), diags, map, text)
    }

    fn words(s: &str) -> Vec<&str> {
        s.split_whitespace().collect()
    }

    #[test]
    fn no_copy_is_identity() {
        let (t, d, ..) = run("       STOP RUN.\n", &[]);
        assert!(d.is_empty());
        assert_eq!(words(&t), ["STOP", "RUN."]);
    }

    #[test]
    fn simple_copy_is_expanded() {
        let (t, d, ..) = run(
            "       DATA DIVISION.\n       COPY REC.\n       PROCEDURE DIVISION.\n",
            &[("REC", "       01 A PIC X.\n")],
        );
        assert!(d.is_empty(), "{d:?}");
        assert_eq!(
            words(&t),
            [
                "DATA",
                "DIVISION.",
                "01",
                "A",
                "PIC",
                "X.",
                "PROCEDURE",
                "DIVISION."
            ]
        );
    }

    #[test]
    fn copy_is_case_insensitive_and_accepts_quoted_names_and_library() {
        for stmt in [
            "copy rec.",
            "COPY 'REC'.",
            "COPY \"rec\".",
            "COPY REC OF LIB.",
            "COPY REC IN LIB .",
        ] {
            let (t, d, ..) = run(&format!("       {stmt}\n"), &[("REC", "       01 A.\n")]);
            assert!(d.is_empty(), "{stmt}: {d:?}");
            assert_eq!(words(&t), ["01", "A."], "{stmt}");
        }
    }

    #[test]
    fn copy_spanning_lines() {
        let (t, d, ..) = run(
            "       COPY\n          REC\n          .\n",
            &[("REC", "       01 A.\n")],
        );
        assert!(d.is_empty());
        assert_eq!(words(&t), ["01", "A."]);
    }

    #[test]
    fn nested_copy() {
        let (t, d, ..) = run(
            "       COPY A.\n",
            &[
                ("A", "       01 X.\n       COPY B.\n"),
                ("B", "       01 Y.\n"),
            ],
        );
        assert!(d.is_empty(), "{d:?}");
        assert_eq!(words(&t), ["01", "X.", "01", "Y."]);
    }

    #[test]
    fn same_copybook_twice_is_allowed() {
        let (t, d, ..) = run(
            "       COPY A.\n       COPY A.\n",
            &[("A", "       01 X.\n")],
        );
        assert!(d.is_empty());
        assert_eq!(words(&t), ["01", "X.", "01", "X."]);
    }

    #[test]
    fn copy_inside_literal_and_as_part_of_word_is_ignored() {
        let (t, d, ..) = run(
            "       DISPLAY 'COPY REC.' COPYRIGHT-X.\n",
            &[("REC", "       01 A.\n")],
        );
        assert!(d.is_empty());
        assert_eq!(words(&t), ["DISPLAY", "'COPY", "REC.'", "COPYRIGHT-X."]);
    }

    #[test]
    fn copy_in_comment_is_ignored() {
        let (t, d, ..) = run(
            "      * COPY REC.\n       STOP RUN.\n",
            &[("REC", "       01 A.\n")],
        );
        assert!(d.is_empty());
        assert_eq!(words(&t), ["STOP", "RUN."]);
    }

    #[test]
    fn missing_copybook() {
        let (_, d, ..) = run("       COPY NOPE.\n", &[]);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].code, "cobol::preprocess::copybook_error");
        assert!(d[0].message.contains("NOPE"));
        assert!(d[0].help.is_some());
    }

    #[test]
    fn recursive_copy_is_detected() {
        let (_, d, ..) = run(
            "       COPY A.\n",
            &[("A", "       COPY B.\n"), ("B", "       COPY A.\n")],
        );
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].code, "cobol::preprocess::recursive_copy");
    }

    #[test]
    fn self_recursive_copy_is_detected() {
        let (_, d, ..) = run("       COPY A.\n", &[("A", "       COPY A.\n")]);
        assert_eq!(
            d.iter()
                .filter(|d| d.code == "cobol::preprocess::recursive_copy")
                .count(),
            1
        );
    }

    #[test]
    fn missing_name() {
        let (_, d, ..) = run("       COPY.\n", &[]);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].code, "cobol::preprocess::missing_name");
    }

    #[test]
    fn missing_period() {
        let (t, d, ..) = run(
            "       COPY REC\n       STOP RUN.\n",
            &[("REC", "       01 A.\n")],
        );
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].code, "cobol::preprocess::missing_period");
        assert!(t.contains("STOP"));
    }

    #[test]
    fn replacing_is_unsupported_and_skipped() {
        let (t, d, ..) = run(
            "       COPY REC REPLACING ==A== BY ==B==.\n       STOP RUN.\n",
            &[("REC", "       01 A.\n")],
        );
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].code, "cobol::preprocess::replacing_unsupported");
        assert_eq!(words(&t), ["STOP", "RUN."]);
    }

    #[test]
    fn spans_of_copied_text_point_into_the_copybook() {
        let (_, d, map, text) = run(
            "       COPY REC.\n",
            &[("REC", "000100 01 FIELD-A PIC X.\n")],
        );
        assert!(d.is_empty());
        let i = text.text.find("FIELD-A").unwrap();
        let span = text.span(i, i + 7);
        assert_eq!(map.file(span.file).name, "REC.cpy");
        assert_eq!(&map.file(span.file).text[span.start..span.end], "FIELD-A");
    }

    #[test]
    fn errors_inside_copybooks_are_reported() {
        let (_, d, map, _) = run("       COPY REC.\n", &[("REC", "      X BAD\n")]);
        assert_eq!(d.len(), 1);
        assert_eq!(map.file(d[0].span.unwrap().file).name, "REC.cpy");
    }

    #[test]
    fn fs_loader_rejects_path_traversal() {
        let loader = FsLoader {
            search_dirs: vec![PathBuf::from(".")],
        };
        for name in ["../x", "a/b", "", "a\\b"] {
            assert!(
                matches!(loader.load(name), Err(LoadError::InvalidName(_))),
                "{name}"
            );
        }
    }

    #[test]
    fn fs_loader_finds_files_with_extensions_and_case() {
        let dir = std::env::temp_dir().join(format!("cobol-fs-loader-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("rec.cpy"), "       01 A.\n").unwrap();
        let loader = FsLoader {
            search_dirs: vec![dir.clone()],
        };
        let book = loader.load("REC").unwrap();
        assert_eq!(book.text, "       01 A.\n");
        assert!(matches!(loader.load("OTHER"), Err(LoadError::NotFound(_))));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
