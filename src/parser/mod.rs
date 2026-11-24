//! Phase 2: syntactic analysis.

use crate::diagnostics::Diagnostic;
use crate::lexer::{Keyword, Token, TokenKind};
use crate::source::{FileId, Span};

/// The name declared by a COBOL `PROGRAM-ID` paragraph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgramId {
    pub name: Option<String>,
    pub span: Span,
}

/// AST node for `IDENTIFICATION DIVISION`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentificationDivision {
    pub span: Span,
    pub program_id: Option<ProgramId>,
}

impl IdentificationDivision {
    pub fn program_name(&self) -> Option<&str> {
        self.program_id
            .as_ref()
            .and_then(|program_id| program_id.name.as_deref())
    }
}

/// Minimal program AST produced by the current parser phase.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Program {
    pub identification_division: IdentificationDivision,
}

pub struct Parser {
    tokens: Vec<Token>,
    index: usize,
    diagnostics: Vec<Diagnostic>,
}

impl Parser {
    pub fn new(tokens: &[Token]) -> Self {
        Parser {
            tokens: tokens.to_vec(),
            index: 0,
            diagnostics: Vec::new(),
        }
    }

    pub fn parse(mut self) -> (Program, Vec<Diagnostic>) {
        let identification_division = self.parse_identification_division();
        (Program { identification_division }, self.diagnostics)
    }

    fn parse_identification_division(&mut self) -> IdentificationDivision {
        let start = self.peek_span().unwrap_or_else(|| Span::new(FileId(0), 0, 0));

        if !self.check_keyword(Keyword::Identification) {
            self.error_current(
                "cobol::parser::missing_identification_division",
                "expected `IDENTIFICATION DIVISION`",
            );
            return IdentificationDivision {
                span: start,
                program_id: None,
            };
        }
        self.advance();

        if !self.check_keyword(Keyword::Division) {
            self.error_current(
                "cobol::parser::expected_division",
                "expected `DIVISION` after `IDENTIFICATION`",
            );
        } else {
            self.advance();
        }

        if !self.check_kind(&TokenKind::Period) {
            self.error_current(
                "cobol::parser::missing_period",
                "expected `.` after `IDENTIFICATION DIVISION`",
            );
        } else {
            self.advance();
        }

        let program_id = self.parse_program_id();
        let end = program_id
            .as_ref()
            .map(|program_id| program_id.span.end)
            .unwrap_or_else(|| {
                self.previous_span()
                    .map(|span| span.end)
                    .unwrap_or(start.end)
            });

        IdentificationDivision {
            span: Span::new(start.file, start.start, end),
            program_id,
        }
    }

    fn parse_program_id(&mut self) -> Option<ProgramId> {
        match self.peek() {
            Some(Token {
                kind: TokenKind::Identifier(name),
                span,
            }) if name == "PROGRAM-ID" => {
                let start = *span;
                self.advance();

                let name = if let Some(Token {
                    kind: TokenKind::Identifier(name),
                    ..
                }) = self.peek()
                {
                    let name = name.clone();
                    self.advance();

                    if !self.check_kind(&TokenKind::Period) {
                        self.error_current(
                            "cobol::parser::missing_period",
                            "expected `.` after the program name",
                        );
                    } else {
                        self.advance();
                    }

                    Some(name)
                } else if self.check_kind(&TokenKind::Period) {
                    self.advance();
                    None
                } else {
                    self.error_current(
                        "cobol::parser::missing_program_name",
                        "expected a program name or `.` after `PROGRAM-ID`",
                    );
                    None
                };

                let end = self
                    .previous_span()
                    .map(|span| span.end)
                    .unwrap_or(start.end);
                Some(ProgramId {
                    name,
                    span: Span::new(start.file, start.start, end),
                })
            }
            Some(_) => {
                self.error_current(
                    "cobol::parser::missing_program_id",
                    "expected `PROGRAM-ID.` after `IDENTIFICATION DIVISION.`",
                );
                None
            }
            None => {
                self.diagnostics.push(Diagnostic::error(
                    "cobol::parser::missing_program_id",
                    "expected `PROGRAM-ID.` after `IDENTIFICATION DIVISION.`",
                ));
                None
            }
        }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.index)
    }

    fn peek_span(&self) -> Option<Span> {
        self.peek().map(|token| token.span)
    }

    fn previous_span(&self) -> Option<Span> {
        self.index
            .checked_sub(1)
            .and_then(|index| self.tokens.get(index))
            .map(|token| token.span)
    }

    fn advance(&mut self) {
        if self.index < self.tokens.len() {
            self.index += 1;
        }
    }

    fn check_keyword(&self, keyword: Keyword) -> bool {
        matches!(self.peek(), Some(Token { kind: TokenKind::Keyword(actual), .. }) if *actual == keyword)
    }

    fn check_kind(&self, kind: &TokenKind) -> bool {
        matches!(self.peek(), Some(token) if &token.kind == kind)
    }

    fn error_current(&mut self, code: &'static str, message: impl Into<String>) {
        let mut diagnostic = Diagnostic::error(code, message);
        if let Some(token) = self.peek() {
            diagnostic = diagnostic
                .with_span(token.span)
                .with_label("unexpected token here");
        }
        self.diagnostics.push(diagnostic);
    }
}

/// Parses the currently specified COBOL prefix, beginning with
/// `IDENTIFICATION DIVISION`.
pub fn parse(tokens: &[Token]) -> (Program, Vec<Diagnostic>) {
    Parser::new(tokens).parse()
}

/// Parses only the `IDENTIFICATION DIVISION` from the token stream.
pub fn parse_identification_division(
    tokens: &[Token],
) -> (IdentificationDivision, Vec<Diagnostic>) {
    let mut parser = Parser::new(tokens);
    let division = parser.parse_identification_division();
    (
        division,
        parser.diagnostics,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn logical(src: &str) -> crate::source::LogicalText {
        let mut text = crate::source::LogicalText::default();
        for (offset, c) in src.char_indices() {
            text.push_char(c, FileId(0), offset);
        }
        text
    }

    fn tokens(src: &str) -> Vec<Token> {
        let (tokens, diagnostics) = crate::lexer::lex(&logical(src));
        assert!(diagnostics.is_empty(), "unexpected lexer diagnostics: {diagnostics:?}");
        tokens
    }

    #[test]
    fn parses_identification_division_and_program_name() {
        let (program, diagnostics) = parse(&tokens(
            "IDENTIFICATION DIVISION. PROGRAM-ID. GREETING.",
        ));

        assert!(diagnostics.is_empty());
        assert_eq!(
            program.identification_division.program_name(),
            Some("GREETING")
        );
    }

    #[test]
    fn accepts_program_id_without_name() {
        let (program, diagnostics) = parse(&tokens("IDENTIFICATION DIVISION. PROGRAM-ID."));

        assert!(diagnostics.is_empty());
        assert_eq!(
            program
                .identification_division
                .program_id
                .as_ref()
                .and_then(|program_id| program_id.name.as_ref()),
            None
        );
    }

    #[test]
    fn reports_missing_identification_division() {
        let (_, diagnostics) = parse(&tokens("ENVIRONMENT DIVISION."));

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(
            diagnostics[0].code,
            "cobol::parser::missing_identification_division"
        );
    }

    #[test]
    fn reports_missing_program_id() {
        let (_, diagnostics) = parse(&tokens("IDENTIFICATION DIVISION."));

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "cobol::parser::missing_program_id");
    }

    #[test]
    fn reports_missing_period() {
        let (_, diagnostics) = parse(&tokens("IDENTIFICATION DIVISION PROGRAM-ID. TEST."));

        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "cobol::parser::missing_period"));
    }

    #[test]
    fn parser_is_case_insensitive() {
        let (program, diagnostics) = parse(&tokens(
            "identification division. program-id. sample.",
        ));

        assert!(diagnostics.is_empty());
        assert_eq!(
            program.identification_division.program_name(),
            Some("SAMPLE")
        );
    }
}
