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
mod tests;
