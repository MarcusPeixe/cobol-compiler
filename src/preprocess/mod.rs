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
mod tests;
