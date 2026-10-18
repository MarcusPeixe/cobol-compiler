use super::*;
use crate::lexer::token::Keyword as K;
use crate::lexer::token::TokenKind as T;
use crate::source::{FileId, ReaderOptions, SourceMap, read_fixed_format};

fn logical(src: &str) -> LogicalText {
    let mut t = LogicalText::default();
    for (i, c) in src.char_indices() {
        t.push_char(c, FileId(0), i);
    }
    t
}

fn kinds(src: &str) -> Vec<TokenKind> {
    let (tokens, diags) = lex(&logical(src));
    assert!(diags.is_empty(), "unexpected diagnostics: {diags:?}");
    tokens.into_iter().map(|t| t.kind).collect()
}

fn errors(src: &str) -> Vec<Diagnostic> {
    lex(&logical(src)).1
}

fn ident(s: &str) -> TokenKind {
    T::Identifier(s.to_string())
}

fn num(s: &str) -> TokenKind {
    T::NumericLiteral(s.to_string())
}

fn string(s: &str) -> TokenKind {
    T::StringLiteral(s.to_string())
}

#[test]
fn empty_and_whitespace_only() {
    assert!(kinds("").is_empty());
    assert!(kinds("  \n\t \r\n").is_empty());
}

#[test]
fn all_keywords() {
    for k in Keyword::ALL {
        assert_eq!(kinds(k.as_str()), [T::Keyword(k)]);
    }
}

#[test]
fn keywords_are_case_insensitive() {
    assert_eq!(
        kinds("display Display DISPLAY"),
        vec![T::Keyword(K::Display); 3]
    );
}

#[test]
fn keyword_prefix_is_an_identifier() {
    assert_eq!(
        kinds("DISPLAYS MOVE-X"),
        [ident("DISPLAYS"), ident("MOVE-X")]
    );
}

#[test]
fn identifiers_are_uppercased() {
    assert_eq!(
        kinds("ws-total Total1"),
        [ident("WS-TOTAL"), ident("TOTAL1")]
    );
}

#[test]
fn identifier_with_interior_hyphens() {
    assert_eq!(kinds("A-B-C A--B"), [ident("A-B-C"), ident("A--B")]);
}

#[test]
fn identifier_may_start_with_digit_when_it_has_letters() {
    assert_eq!(kinds("1A 2-B"), [ident("1A"), ident("2-B")]);
}

#[test]
fn identifier_of_30_characters_is_accepted() {
    let name = "A".repeat(30);
    assert_eq!(kinds(&name), [ident(&name)]);
}

#[test]
fn identifier_longer_than_30_is_an_error() {
    let d = errors(&"A".repeat(31));
    assert_eq!(d.len(), 1);
    assert_eq!(d[0].code, "cobol::lexer::identifier_too_long");
}

#[test]
fn identifier_ending_in_hyphen_is_an_error() {
    let d = errors("ABC- DEF");
    assert_eq!(d.len(), 1);
    assert_eq!(d[0].code, "cobol::lexer::trailing_hyphen");
}

#[test]
fn leading_hyphen_is_a_minus_operator() {
    assert_eq!(kinds("-ABC"), [T::Minus, ident("ABC")]);
}

#[test]
fn integers_and_decimals() {
    assert_eq!(
        kinds("0 123 45.67 007"),
        [num("0"), num("123"), num("45.67"), num("007")]
    );
}

#[test]
fn signs_are_separate_tokens() {
    assert_eq!(
        kinds("-123.45 +7"),
        [T::Minus, num("123.45"), T::Plus, num("7")]
    );
}

#[test]
fn trailing_period_is_a_sentence_terminator_not_a_decimal_point() {
    assert_eq!(
        kinds("MOVE 1 TO X."),
        [
            T::Keyword(K::Move),
            num("1"),
            ident("TO"),
            ident("X"),
            T::Period
        ]
    );
    assert_eq!(kinds("1."), [num("1"), T::Period]);
    assert_eq!(kinds("1.\n2"), [num("1"), T::Period, num("2")]);
}

#[test]
fn leading_period_is_not_part_of_a_number() {
    assert_eq!(kinds(".5"), [T::Period, num("5")]);
}

#[test]
fn second_decimal_point_ends_the_number() {
    assert_eq!(kinds("1.2.3"), [num("1.2"), T::Period, num("3")]);
}

#[test]
fn string_literals_with_both_quotes() {
    assert_eq!(kinds("'abc' \"def\""), [string("abc"), string("def")]);
}

#[test]
fn empty_string_literal() {
    assert_eq!(kinds("''"), [string("")]);
}

#[test]
fn doubled_quotes_are_escapes() {
    assert_eq!(
        kinds("'IT''S' \"SAY \"\"HI\"\"\""),
        [string("IT'S"), string("SAY \"HI\"")]
    );
}

#[test]
fn other_quote_inside_literal_is_plain() {
    assert_eq!(kinds("'say \"hi\"'"), [string("say \"hi\"")]);
}

#[test]
fn string_preserves_case_symbols_and_unicode() {
    assert_eq!(
        kinds("'Olá, Mundo! ção . ( ) *'"),
        [string("Olá, Mundo! ção . ( ) *")]
    );
}

#[test]
fn unterminated_string_stops_at_end_of_line_and_lexing_continues() {
    let (tokens, diags) = lex(&logical("DISPLAY 'oops\nSTOP RUN."));
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].code, "cobol::lexer::unterminated_string");
    let kinds: Vec<_> = tokens.into_iter().map(|t| t.kind).collect();
    assert_eq!(
        kinds,
        [
            T::Keyword(K::Display),
            T::Keyword(K::Stop),
            T::Keyword(K::Run),
            T::Period
        ]
    );
}

#[test]
fn unterminated_string_at_end_of_input() {
    let d = errors("'abc");
    assert_eq!(d.len(), 1);
    assert_eq!(d[0].span.unwrap().len(), 4);
}

#[test]
fn punctuation() {
    assert_eq!(
        kinds(". , ; ( )"),
        [T::Period, T::Comma, T::Semicolon, T::LParen, T::RParen]
    );
}

#[test]
fn operators() {
    assert_eq!(
        kinds("+ - * / ** = < > <= >="),
        [
            T::Plus,
            T::Minus,
            T::Star,
            T::Slash,
            T::Power,
            T::Eq,
            T::Lt,
            T::Gt,
            T::LtEq,
            T::GtEq
        ]
    );
}

#[test]
fn operators_without_whitespace() {
    assert_eq!(
        kinds("A<=B**2"),
        [ident("A"), T::LtEq, ident("B"), T::Power, num("2")]
    );
    assert_eq!(kinds("(A)"), [T::LParen, ident("A"), T::RParen]);
    assert_eq!(kinds("A>B"), [ident("A"), T::Gt, ident("B")]);
}

#[test]
fn spaced_minus_is_an_operator() {
    assert_eq!(kinds("A - B"), [ident("A"), T::Minus, ident("B")]);
}

#[test]
fn three_stars_are_power_then_star() {
    assert_eq!(kinds("***"), [T::Power, T::Star]);
}

#[test]
fn illegal_characters_are_reported_and_skipped() {
    let (tokens, diags) = lex(&logical("A @ B # ç"));
    assert_eq!(diags.len(), 3);
    assert!(
        diags
            .iter()
            .all(|d| d.code == "cobol::lexer::illegal_character")
    );
    assert_eq!(tokens.len(), 2);
    assert!(diags[2].message.contains('ç'));
}

#[test]
fn underscore_is_illegal() {
    assert_eq!(errors("A_B").len(), 1);
}

#[test]
fn picture_clause() {
    assert_eq!(
        kinds("PIC 9(5)V99."),
        [T::Picture, T::PictureString("9(5)V99".into()), T::Period]
    );
}

#[test]
fn picture_keyword_spellings_and_case() {
    assert_eq!(
        kinds("picture x(10)"),
        [T::Picture, T::PictureString("X(10)".into())]
    );
    assert_eq!(
        kinds("Pic s9(3)"),
        [T::Picture, T::PictureString("S9(3)".into())]
    );
}

#[test]
fn picture_with_is() {
    assert_eq!(
        kinds("PIC IS 999"),
        [T::Picture, ident("IS"), T::PictureString("999".into())]
    );
}

#[test]
fn picture_with_embedded_decimal_point_and_terminator() {
    assert_eq!(
        kinds("PIC S9(3).99."),
        [T::Picture, T::PictureString("S9(3).99".into()), T::Period]
    );
    assert_eq!(
        kinds("PIC 9.99 VALUE 1.5."),
        [
            T::Picture,
            T::PictureString("9.99".into()),
            ident("VALUE"),
            num("1.5"),
            T::Period
        ]
    );
}

#[test]
fn picture_on_next_line_and_at_end_of_input() {
    assert_eq!(
        kinds("PIC\n   X(3)."),
        [T::Picture, T::PictureString("X(3)".into()), T::Period]
    );
    assert_eq!(
        kinds("PIC X."),
        [T::Picture, T::PictureString("X".into()), T::Period]
    );
}

#[test]
fn picture_with_special_characters() {
    assert_eq!(
        kinds("PIC $ZZ,ZZ9.99CR"),
        [T::Picture, T::PictureString("$ZZ,ZZ9.99CR".into())]
    );
    assert_eq!(
        kinds("PIC -9(3)"),
        [T::Picture, T::PictureString("-9(3)".into())]
    );
}

#[test]
fn missing_picture_string() {
    let d = errors("PIC .");
    assert_eq!(d.len(), 1);
    assert_eq!(d[0].code, "cobol::lexer::missing_picture");
    assert_eq!(errors("PIC").len(), 1);
    assert_eq!(errors("PIC IS").len(), 1);
}

#[test]
fn missing_picture_still_lexes_the_period() {
    let (tokens, _) = lex(&logical("PIC ."));
    let kinds: Vec<_> = tokens.into_iter().map(|t| t.kind).collect();
    assert_eq!(kinds, [T::Picture, T::Period]);
}

#[test]
fn pic_inside_a_longer_word_is_an_identifier() {
    assert_eq!(kinds("PIC-X PICTURES"), [ident("PIC-X"), ident("PICTURES")]);
}

#[test]
fn errors_do_not_stop_lexing() {
    let (tokens, diags) = lex(&logical("MOVE @ 1 TO X- .\nDISPLAY 'x"));
    assert_eq!(diags.len(), 3);
    let kinds: Vec<_> = tokens.into_iter().map(|t| t.kind).collect();
    assert_eq!(
        kinds,
        [
            T::Keyword(K::Move),
            num("1"),
            ident("TO"),
            T::Period,
            T::Keyword(K::Display)
        ]
    );
}

#[test]
fn spans_cover_the_token_text() {
    let src = "MOVE 'AB' TO TOTAL.";
    let (tokens, _) = lex(&logical(src));
    let slices: Vec<_> = tokens
        .iter()
        .map(|t| &src[t.span.start..t.span.end])
        .collect();
    assert_eq!(slices, ["MOVE", "'AB'", "TO", "TOTAL", "."]);
}

#[test]
fn spans_for_picture_and_operators() {
    let src = "PIC 9(3) <= **";
    let (tokens, _) = lex(&logical(src));
    let slices: Vec<_> = tokens
        .iter()
        .map(|t| &src[t.span.start..t.span.end])
        .collect();
    assert_eq!(slices, ["PIC", "9(3)", "<=", "**"]);
}

fn lex_fixed(src: &str) -> (Vec<Token>, Vec<Diagnostic>, SourceMap) {
    let mut map = SourceMap::default();
    let id = map.add_file("t.cob", src.to_string());
    let (text, mut diags) = read_fixed_format(&map, id, ReaderOptions::default());
    let (tokens, more) = lex(&text);
    diags.extend(more);
    (tokens, diags, map)
}

#[test]
fn end_to_end_program_with_original_positions() {
    let src = "\
000100 IDENTIFICATION DIVISION.                                          ID
000200 PROGRAM-ID. HELLO.
000300* a comment
000400 PROCEDURE DIVISION.
000500     DISPLAY 'HELLO, WORLD'.
000600     STOP RUN.
";
    let (tokens, diags, map) = lex_fixed(src);
    assert!(diags.is_empty(), "{diags:?}");
    let kinds: Vec<_> = tokens.iter().map(|t| t.kind.clone()).collect();
    assert_eq!(
        kinds,
        [
            T::Keyword(K::Identification),
            T::Keyword(K::Division),
            T::Period,
            ident("PROGRAM-ID"),
            T::Period,
            ident("HELLO"),
            T::Period,
            T::Keyword(K::Procedure),
            T::Keyword(K::Division),
            T::Period,
            T::Keyword(K::Display),
            string("HELLO, WORLD"),
            T::Period,
            T::Keyword(K::Stop),
            T::Keyword(K::Run),
            T::Period,
        ]
    );
    let display = tokens
        .iter()
        .find(|t| t.kind == T::Keyword(K::Display))
        .unwrap();
    assert_eq!(map.line_col(display.span.file, display.span.start), (5, 12));
    let hello = tokens.iter().find(|t| t.kind == ident("HELLO")).unwrap();
    assert_eq!(map.line_col(hello.span.file, hello.span.start), (2, 20));
}

#[test]
fn end_to_end_continued_literal() {
    let (tokens, diags, _) = lex_fixed("       DISPLAY 'AB\n      -    'CD'.\n");
    assert!(diags.is_empty());
    let TokenKind::StringLiteral(s) = &tokens[1].kind else {
        panic!("expected string")
    };
    assert!(s.starts_with("AB") && s.ends_with("CD"));
    assert_eq!(s.len(), 2 + 54 + 2);
    assert_eq!(tokens[2].kind, T::Period);
}

#[test]
fn end_to_end_text_after_column_72_is_ignored() {
    let line = format!("       STOP RUN.{}IGNORED TEXT\n", " ".repeat(56));
    let (tokens, diags, _) = lex_fixed(&line);
    assert!(diags.is_empty());
    assert_eq!(tokens.len(), 3);
}

#[test]
fn end_to_end_data_division_entry() {
    let (tokens, diags, _) = lex_fixed("       01  WS-AMOUNT  PIC S9(5)V99  VALUE -12.50.\n");
    assert!(diags.is_empty());
    let kinds: Vec<_> = tokens.into_iter().map(|t| t.kind).collect();
    assert_eq!(
        kinds,
        [
            num("01"),
            ident("WS-AMOUNT"),
            T::Picture,
            T::PictureString("S9(5)V99".into()),
            ident("VALUE"),
            T::Minus,
            num("12.50"),
            T::Period
        ]
    );
}
