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
    let (t, d) =
        read("000100 DISPLAY 'HI'.                                                    PROG0001\n");
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
