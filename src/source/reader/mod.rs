//! Fixed-format reader (columns 1-6 sequence, 7 indicator, 8-72 code, 73-80 ignored).

use super::{FileId, LogicalText, Origin, SourceMap, Span};
use crate::diagnostics::Diagnostic;

const INDICATOR_INDEX: usize = 6;
const AREA_START: usize = 7;
const AREA_END: usize = 72;
const AREA_WIDTH: usize = AREA_END - AREA_START;

#[derive(Debug, Clone, Copy, Default)]
pub struct ReaderOptions {
    /// Treat lines with `D` in column 7 as code instead of skipping them.
    pub debug_lines: bool,
}

/// Converts a fixed-format file into logical text: comments (and debug lines
/// unless enabled) are dropped, literal continuations are joined and columns
/// outside 8-72 are removed. Every line of output ends with `\n`.
pub fn read_fixed_format(
    map: &SourceMap,
    file: FileId,
    options: ReaderOptions,
) -> (LogicalText, Vec<Diagnostic>) {
    let source = &map.file(file).text;
    let mut out = LogicalText::default();
    let mut diagnostics = Vec::new();

    let mut open_quote: Option<char> = None;
    // Padding needed to bring the previous line up to column 72, and where it ended.
    let mut prev_pad = 0;
    let mut prev_end = Origin { file, offset: 0 };
    let mut line_start = 0;

    for raw in source.split_inclusive('\n') {
        let base = line_start;
        line_start += raw.len();
        let line = raw.strip_suffix('\n').unwrap_or(raw);
        let line = line.strip_suffix('\r').unwrap_or(line);

        let chars: Vec<(usize, char)> = line.char_indices().collect();
        let Some(&(indicator_offset, indicator)) = chars.get(INDICATOR_INDEX) else {
            continue;
        };
        let area = &chars[AREA_START.min(chars.len())..AREA_END.min(chars.len())];
        if area.iter().all(|(_, c)| c.is_whitespace()) && indicator != '-' {
            continue;
        }

        let continuation = match indicator {
            '*' | '/' => continue,
            'D' | 'd' if !options.debug_lines => continue,
            'D' | 'd' | ' ' => false,
            '-' => true,
            other => {
                let at = base + indicator_offset;
                diagnostics.push(
                    Diagnostic::error(
                        "cobol::source::invalid_indicator",
                        format!("invalid indicator '{other}' in column 7"),
                    )
                    .with_span(Span::new(file, at, at + other.len_utf8()))
                    .with_label("expected ' ', '*', '/', '-' or 'D'"),
                );
                continue;
            }
        };

        let mut content = area;
        if continuation {
            match (
                open_quote,
                area.iter().position(|(_, c)| !c.is_whitespace()),
            ) {
                (Some(q), Some(p)) if area[p].1 == q => {
                    out.text.pop();
                    out.origins.pop();
                    for _ in 0..prev_pad {
                        out.push_synthetic(' ', prev_end);
                    }
                    content = &area[p + 1..];
                }
                (Some(q), found) => {
                    let (offset, len) = match found {
                        Some(p) => (area[p].0, area[p].1.len_utf8()),
                        None => (indicator_offset, 1),
                    };
                    diagnostics.push(
                        Diagnostic::error(
                            "cobol::source::bad_continuation",
                            "continuation of a literal must begin with a quote",
                        )
                        .with_span(Span::new(file, base + offset, base + offset + len))
                        .with_label(format!("expected {q}")),
                    );
                    open_quote = None;
                }
                (None, _) => open_quote = None,
            }
        } else {
            open_quote = None;
        }

        for &(offset, c) in content {
            out.push_char(c, file, base + offset);
            match open_quote {
                None if c == '\'' || c == '"' => open_quote = Some(c),
                Some(q) if c == q => open_quote = None,
                _ => {}
            }
        }
        prev_pad = AREA_WIDTH.saturating_sub(area.len());
        prev_end = Origin {
            file,
            offset: base + line.len(),
        };
        out.push_synthetic('\n', prev_end);
    }

    (out, diagnostics)
}

#[cfg(test)]
mod tests;
