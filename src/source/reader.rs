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
mod tests {
    use super::*;

    fn read_with(
        src: &str,
        options: ReaderOptions,
    ) -> (String, Vec<Diagnostic>, SourceMap, LogicalText) {
        let mut map = SourceMap::default();
        let id = map.add_file("t.cob", src.to_string());
        let (text, diags) = read_fixed_format(&map, id, options);
        (text.text.clone(), diags, map, text)
    }

    fn read(src: &str) -> (String, Vec<Diagnostic>) {
        let (t, d, _, _) = read_with(src, ReaderOptions::default());
        (t, d)
    }

    #[test]
    fn strips_sequence_area_and_program_area() {
        let (t, d) = read(
            "000100 DISPLAY 'HI'.                                                    PROG0001\n",
        );
        assert!(d.is_empty());
        assert_eq!(t.trim_end(), "DISPLAY 'HI'.");
        assert!(!t.contains("PROG0001"));
    }

    #[test]
    fn column_72_is_included_and_73_is_not() {
        let mut line = String::from("      ");
        line.push(' ');
        line.push_str(&"A".repeat(64));
        line.push('B'); // column 72
        line.push('C'); // column 73
        let (t, _) = read(&line);
        assert!(t.trim_end().ends_with('B'));
        assert!(!t.contains('C'));
    }

    #[test]
    fn comment_lines_are_dropped() {
        let (t, d) = read("      * a comment\n      / page break\n       STOP RUN.\n");
        assert!(d.is_empty());
        assert_eq!(t.trim(), "STOP RUN.");
    }

    #[test]
    fn debug_lines_skipped_by_default() {
        let (t, _) = read("      D    DISPLAY 'X'.\n       STOP RUN.\n");
        assert!(!t.contains("DISPLAY"));
        assert!(t.contains("STOP RUN."));
    }

    #[test]
    fn debug_lines_kept_with_option_and_lowercase_d() {
        let opts = ReaderOptions { debug_lines: true };
        let (t, ..) = read_with("      D    DISPLAY 'X'.\n      d    DISPLAY 'Y'.\n", opts);
        assert!(t.contains("DISPLAY 'X'."));
        assert!(t.contains("DISPLAY 'Y'."));
    }

    #[test]
    fn short_and_blank_lines_are_ignored() {
        let (t, d) = read("\n   \n123\n       STOP RUN.\n\n");
        assert!(d.is_empty());
        assert_eq!(t.trim(), "STOP RUN.");
    }

    #[test]
    fn crlf_line_endings() {
        let (t, d) = read("       STOP\r\n       RUN.\r\n");
        assert!(d.is_empty());
        assert!(!t.contains('\r'));
        assert_eq!(
            t.lines().map(str::trim).collect::<Vec<_>>(),
            ["STOP", "RUN."]
        );
    }

    #[test]
    fn invalid_indicator_reports_error_and_skips_line() {
        let (t, d) = read("      X    DISPLAY 'X'.\n       STOP RUN.\n");
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].code, "cobol::source::invalid_indicator");
        assert_eq!(d[0].span.unwrap().start, 6);
        assert!(!t.contains("DISPLAY"));
        assert!(t.contains("STOP RUN."));
    }

    #[test]
    fn literal_continuation_pads_first_line_to_column_72() {
        let (t, d) = read("       DISPLAY 'AB\n      -    'CD'.\n");
        assert!(d.is_empty());
        assert_eq!(t, format!("DISPLAY 'AB{}CD'.\n", " ".repeat(54)));
    }

    #[test]
    fn continuation_with_doubled_quote_state() {
        let (t, d) = read("       DISPLAY 'IT''S\n      -    'OK'.\n");
        assert!(d.is_empty());
        assert!(t.contains("IT''S"));
        assert!(t.ends_with("OK'.\n"));
    }

    #[test]
    fn double_quote_continuation() {
        let (t, d) = read("       DISPLAY \"AB\n      -    \"CD\".\n");
        assert!(d.is_empty());
        assert!(t.trim_end().ends_with("CD\"."));
    }

    #[test]
    fn continuation_missing_quote_is_error() {
        let (_, d) = read("       DISPLAY 'AB\n      -    CD'.\n");
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].code, "cobol::source::bad_continuation");
    }

    #[test]
    fn continuation_without_open_literal_is_plain_line() {
        let (t, d) = read("       MOVE A\n      -    TO B.\n");
        assert!(d.is_empty());
        assert_eq!(
            t.lines().map(str::trim).collect::<Vec<_>>(),
            ["MOVE A", "TO B."]
        );
    }

    #[test]
    fn comment_between_continued_lines_is_ignored() {
        let (t, d) = read("       DISPLAY 'AB\n      * note\n      -    'CD'.\n");
        assert!(d.is_empty());
        assert!(t.contains("CD'."));
        assert_eq!(t.matches('\n').count(), 1);
    }

    #[test]
    fn quote_state_resets_between_lines() {
        let (t, d) = read("       DISPLAY 'AB\n       STOP RUN.\n      -    'X'.\n");
        // The unterminated literal does not leak into later lines; the lexer reports it.
        assert!(d.is_empty());
        assert!(t.contains("STOP RUN."));
        assert!(t.contains("'X'."));
    }

    #[test]
    fn origins_point_back_to_original_columns() {
        let src = "000100 MOVE A TO B.\n";
        let (_, d, map, text) = read_with(src, ReaderOptions::default());
        assert!(d.is_empty());
        let m = text.text.find("MOVE").unwrap();
        let span = text.span(m, m + 4);
        assert_eq!(&map.file(span.file).text[span.start..span.end], "MOVE");
        assert_eq!(map.line_col(span.file, span.start), (1, 8));
    }

    #[test]
    fn origins_across_lines_and_multibyte() {
        let src = "       DISPLAY 'AÇÃO'.\n       STOP RUN.\n";
        let (_, _, map, text) = read_with(src, ReaderOptions::default());
        let i = text.text.find("AÇÃO").unwrap();
        let span = text.span(i, i + "AÇÃO".len());
        assert_eq!(&map.file(span.file).text[span.start..span.end], "AÇÃO");
        let j = text.text.find("STOP").unwrap();
        let span = text.span(j, j + 4);
        assert_eq!(map.line_col(span.file, span.start), (2, 8));
    }

    #[test]
    fn empty_input() {
        let (t, d) = read("");
        assert!(t.is_empty());
        assert!(d.is_empty());
    }
}
