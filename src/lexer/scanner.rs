use super::token::{Keyword, Token, TokenKind};
use crate::diagnostics::Diagnostic;
use crate::source::LogicalText;

const MAX_IDENTIFIER_LEN: usize = 30;

/// Tokenizes logical text. Lexical errors are collected and lexing continues
/// after each one.
pub fn lex(input: &LogicalText) -> (Vec<Token>, Vec<Diagnostic>) {
    let mut scanner = Scanner {
        input,
        text: &input.text,
        pos: 0,
        tokens: Vec::new(),
        diagnostics: Vec::new(),
    };
    scanner.run();
    (scanner.tokens, scanner.diagnostics)
}

struct Scanner<'a> {
    input: &'a LogicalText,
    text: &'a str,
    pos: usize,
    tokens: Vec<Token>,
    diagnostics: Vec<Diagnostic>,
}

fn is_word_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'-'
}

impl Scanner<'_> {
    fn bytes(&self) -> &[u8] {
        self.text.as_bytes()
    }

    fn peek(&self, offset: usize) -> Option<u8> {
        self.bytes().get(self.pos + offset).copied()
    }

    fn push(&mut self, kind: TokenKind, start: usize, end: usize) {
        let span = self.input.span(start, end);
        self.tokens.push(Token { kind, span });
    }

    fn error(
        &mut self,
        code: &'static str,
        message: String,
        start: usize,
        end: usize,
        label: &str,
    ) {
        let span = self.input.span(start, end);
        self.diagnostics.push(
            Diagnostic::error(code, message)
                .with_span(span)
                .with_label(label),
        );
    }

    fn run(&mut self) {
        while let Some(c) = self.peek(0) {
            let start = self.pos;
            match c {
                b' ' | b'\t' | b'\n' | b'\r' => self.pos += 1,
                b'\'' | b'"' => self.string(c),
                c if c.is_ascii_alphanumeric() => self.word(),
                b'.' => self.symbol(TokenKind::Period, 1),
                b',' => self.symbol(TokenKind::Comma, 1),
                b';' => self.symbol(TokenKind::Semicolon, 1),
                b'(' => self.symbol(TokenKind::LParen, 1),
                b')' => self.symbol(TokenKind::RParen, 1),
                b'+' => self.symbol(TokenKind::Plus, 1),
                b'-' => self.symbol(TokenKind::Minus, 1),
                b'/' => self.symbol(TokenKind::Slash, 1),
                b'=' => self.symbol(TokenKind::Eq, 1),
                b'*' if self.peek(1) == Some(b'*') => self.symbol(TokenKind::Power, 2),
                b'*' => self.symbol(TokenKind::Star, 1),
                b'<' if self.peek(1) == Some(b'=') => self.symbol(TokenKind::LtEq, 2),
                b'<' => self.symbol(TokenKind::Lt, 1),
                b'>' if self.peek(1) == Some(b'=') => self.symbol(TokenKind::GtEq, 2),
                b'>' => self.symbol(TokenKind::Gt, 1),
                _ => {
                    let ch = self.text[start..]
                        .chars()
                        .next()
                        .expect("pos is inside text");
                    self.pos += ch.len_utf8();
                    self.error(
                        "cobol::lexer::illegal_character",
                        format!("illegal character {ch:?}"),
                        start,
                        self.pos,
                        "not part of the COBOL character set",
                    );
                }
            }
        }
    }

    fn symbol(&mut self, kind: TokenKind, len: usize) {
        let start = self.pos;
        self.pos += len;
        self.push(kind, start, self.pos);
    }

    fn string(&mut self, quote: u8) {
        let start = self.pos;
        let mut j = start + 1;
        let mut value = String::new();
        loop {
            match self.bytes().get(j) {
                None | Some(b'\n') => {
                    self.pos = j;
                    self.error(
                        "cobol::lexer::unterminated_string",
                        "unterminated string literal".to_string(),
                        start,
                        j,
                        "literal is missing its closing quote",
                    );
                    return;
                }
                Some(&b) if b == quote => {
                    if self.bytes().get(j + 1) == Some(&quote) {
                        value.push(quote as char);
                        j += 2;
                    } else {
                        j += 1;
                        break;
                    }
                }
                Some(_) => {
                    let ch = self.text[j..].chars().next().expect("j is inside text");
                    value.push(ch);
                    j += ch.len_utf8();
                }
            }
        }
        self.pos = j;
        self.push(TokenKind::StringLiteral(value), start, j);
    }

    fn word(&mut self) {
        let start = self.pos;
        while self.peek(0).is_some_and(is_word_byte) {
            self.pos += 1;
        }
        let word = &self.text[start..self.pos];

        if word.bytes().all(|b| b.is_ascii_digit()) {
            if self.peek(0) == Some(b'.') && self.peek(1).is_some_and(|b| b.is_ascii_digit()) {
                self.pos += 1;
                while self.peek(0).is_some_and(|b| b.is_ascii_digit()) {
                    self.pos += 1;
                }
            }
            let number = self.text[start..self.pos].to_string();
            self.push(TokenKind::NumericLiteral(number), start, self.pos);
            return;
        }

        if word.ends_with('-') {
            self.error(
                "cobol::lexer::trailing_hyphen",
                format!("word '{word}' ends with a hyphen"),
                start,
                self.pos,
                "words cannot end with '-'",
            );
            return;
        }
        if word.len() > MAX_IDENTIFIER_LEN {
            self.error(
                "cobol::lexer::identifier_too_long",
                format!(
                    "word is {} characters long (maximum is {MAX_IDENTIFIER_LEN})",
                    word.len()
                ),
                start,
                self.pos,
                "too long",
            );
            return;
        }

        if let Some(keyword) = Keyword::from_word(word) {
            self.push(TokenKind::Keyword(keyword), start, self.pos);
        } else if word.eq_ignore_ascii_case("PIC") || word.eq_ignore_ascii_case("PICTURE") {
            self.push(TokenKind::Picture, start, self.pos);
            self.picture_string(start);
        } else {
            self.push(
                TokenKind::Identifier(word.to_ascii_uppercase()),
                start,
                self.pos,
            );
        }
    }

    /// Lexes the picture string that follows `PIC`/`PICTURE`, after an optional `IS`.
    fn picture_string(&mut self, pic_start: usize) {
        let pic_end = self.pos;
        self.skip_whitespace();

        let is_start = self.pos;
        let mut j = is_start;
        while self.bytes().get(j).copied().is_some_and(is_word_byte) {
            j += 1;
        }
        if self.text[is_start..j].eq_ignore_ascii_case("IS") {
            self.pos = j;
            self.push(TokenKind::Identifier("IS".to_string()), is_start, j);
            self.skip_whitespace();
        }

        let start = self.pos;
        let mut end = start;
        while self
            .bytes()
            .get(end)
            .is_some_and(|b| !b.is_ascii_whitespace())
        {
            end += 1;
        }
        // A trailing '.' is the sentence terminator, not part of the picture.
        if end > start && self.bytes()[end - 1] == b'.' {
            end -= 1;
        }
        if end == start {
            self.error(
                "cobol::lexer::missing_picture",
                "expected a picture string after PIC".to_string(),
                pic_start,
                pic_end,
                "picture string missing",
            );
            return;
        }
        self.pos = end;
        let picture = self.text[start..end].to_ascii_uppercase();
        self.push(TokenKind::PictureString(picture), start, end);
    }

    fn skip_whitespace(&mut self) {
        while self.peek(0).is_some_and(|b| b.is_ascii_whitespace()) {
            self.pos += 1;
        }
    }
}

#[cfg(test)]
mod tests {
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
}
