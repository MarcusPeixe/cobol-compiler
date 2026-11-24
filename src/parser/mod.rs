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

/// Computer associated with a paragraph in `CONFIGURATION SECTION`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComputerParagraph {
    pub name: Option<String>,
    pub span: Span,
}

/// `CONFIGURATION SECTION` of `ENVIRONMENT DIVISION`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigurationSection {
    pub span: Span,
    pub source_computer: Option<ComputerParagraph>,
    pub object_computer: Option<ComputerParagraph>,
}

/// One `SELECT file-name ASSIGN TO destination` entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileControlEntry {
    pub file_name: String,
    pub destination: Option<String>,
    pub span: Span,
}

/// `INPUT-OUTPUT SECTION` of `ENVIRONMENT DIVISION`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputOutputSection {
    pub span: Span,
    pub file_control: Vec<FileControlEntry>,
}

/// Optional `ENVIRONMENT DIVISION`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvironmentDivision {
    pub span: Span,
    pub configuration_section: Option<ConfigurationSection>,
    pub input_output_section: Option<InputOutputSection>,
}

/// Minimal program AST produced by the current parser phase.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Program {
    pub identification_division: IdentificationDivision,
    pub environment_division: Option<EnvironmentDivision>,
}

#[derive(Debug, Clone)]
struct ValueWithSpan {
    name: String,
    span: Span,
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
        let environment_division = self.parse_environment_division();
        (
            Program {
                identification_division,
                environment_division,
            },
            self.diagnostics,
        )
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

    fn parse_environment_division(&mut self) -> Option<EnvironmentDivision> {
        if !self.check_keyword(Keyword::Environment) {
            return None;
        }

        let start = self.peek_span().expect("ENVIRONMENT token was checked");
        self.advance();

        if !self.check_keyword(Keyword::Division) {
            self.error_current(
                "cobol::parser::environment_expected_division",
                "expected `DIVISION` after `ENVIRONMENT`",
            );
        } else {
            self.advance();
        }

        if !self.check_kind(&TokenKind::Period) {
            self.error_current(
                "cobol::parser::environment_missing_period",
                "expected `.` after `ENVIRONMENT DIVISION`",
            );
        } else {
            self.advance();
        }

        let configuration_section = self.parse_configuration_section();
        let input_output_section = self.parse_input_output_section();

        let end = input_output_section
            .as_ref()
            .map(|section| section.span.end)
            .or_else(|| configuration_section.as_ref().map(|section| section.span.end))
            .or_else(|| self.previous_span().map(|span| span.end))
            .unwrap_or(start.end);

        Some(EnvironmentDivision {
            span: Span::new(start.file, start.start, end),
            configuration_section,
            input_output_section,
        })
    }

    fn parse_configuration_section(&mut self) -> Option<ConfigurationSection> {
        if !self.check_identifier("CONFIGURATION") {
            return None;
        }

        let start = self.peek_span().expect("CONFIGURATION token was checked");
        self.advance();

        if !self.check_keyword(Keyword::Section) {
            self.error_current(
                "cobol::parser::configuration_expected_section",
                "expected `SECTION` after `CONFIGURATION`",
            );
        } else {
            self.advance();
        }

        if !self.check_kind(&TokenKind::Period) {
            self.error_current(
                "cobol::parser::configuration_missing_period",
                "expected `.` after `CONFIGURATION SECTION`",
            );
        } else {
            self.advance();
        }

        let source_computer = self.parse_computer_paragraph("SOURCE-COMPUTER");
        let object_computer = self.parse_computer_paragraph("OBJECT-COMPUTER");

        let end = object_computer
            .as_ref()
            .map(|computer| computer.span.end)
            .or_else(|| source_computer.as_ref().map(|computer| computer.span.end))
            .unwrap_or_else(|| {
                self.previous_span()
                    .map(|span| span.end)
                    .unwrap_or(start.end)
            });

        Some(ConfigurationSection {
            span: Span::new(start.file, start.start, end),
            source_computer,
            object_computer,
        })
    }

    fn parse_computer_paragraph(&mut self, paragraph: &str) -> Option<ComputerParagraph> {
        if !self.check_identifier(paragraph) {
            return None;
        }

        let start = self.peek_span().expect("computer paragraph was checked");
        self.advance();

        if !self.check_kind(&TokenKind::Period) {
            self.error_current(
                "cobol::parser::computer_missing_period",
                format!("expected `.` after `{paragraph}`"),
            );
        } else {
            self.advance();
        }

        let name = self.take_optional_name(&["OBJECT-COMPUTER", "INPUT-OUTPUT"]);
        let end = self
            .previous_span()
            .map(|span| span.end)
            .unwrap_or(start.end);

        Some(ComputerParagraph {
            name,
            span: Span::new(start.file, start.start, end),
        })
    }

    fn parse_input_output_section(&mut self) -> Option<InputOutputSection> {
        if !self.check_identifier("INPUT-OUTPUT") {
            return None;
        }

        let start = self.peek_span().expect("INPUT-OUTPUT token was checked");
        self.advance();

        if !self.check_keyword(Keyword::Section) {
            self.error_current(
                "cobol::parser::input_output_expected_section",
                "expected `SECTION` after `INPUT-OUTPUT`",
            );
        } else {
            self.advance();
        }

        if !self.check_kind(&TokenKind::Period) {
            self.error_current(
                "cobol::parser::input_output_missing_period",
                "expected `.` after `INPUT-OUTPUT SECTION`",
            );
        } else {
            self.advance();
        }

        if !self.check_identifier("FILE-CONTROL") {
            self.error_current(
                "cobol::parser::missing_file_control",
                "expected `FILE-CONTROL` in `INPUT-OUTPUT SECTION`",
            );
        } else {
            self.advance();

            if !self.check_kind(&TokenKind::Period) {
                self.error_current(
                    "cobol::parser::file_control_missing_period",
                    "expected `.` after `FILE-CONTROL`",
                );
            } else {
                self.advance();
            }
        }

        let mut file_control = Vec::new();
        while self.check_identifier("SELECT") {
            if let Some(entry) = self.parse_file_control_entry() {
                file_control.push(entry);
            } else {
                break;
            }
        }

        let end = file_control
            .last()
            .map(|entry| entry.span.end)
            .or_else(|| self.previous_span().map(|span| span.end))
            .unwrap_or(start.end);

        Some(InputOutputSection {
            span: Span::new(start.file, start.start, end),
            file_control,
        })
    }

    fn parse_file_control_entry(&mut self) -> Option<FileControlEntry> {
        let start = self.peek_span().expect("SELECT token was checked");
        self.advance();

        let file_name = self.take_identifier_or_string()?.name;

        if !self.check_identifier("ASSIGN") {
            self.error_current(
                "cobol::parser::missing_assign",
                "expected `ASSIGN TO` after the file name",
            );
        } else {
            self.advance();

            if !self.check_identifier("TO") {
                self.error_current(
                    "cobol::parser::missing_assign_to",
                    "expected `TO` after `ASSIGN`",
                );
            } else {
                self.advance();
            }
        }

        let destination = self.take_identifier_or_string().map(|value| value.name);

        if !self.check_kind(&TokenKind::Period) {
            self.error_current(
                "cobol::parser::select_missing_period",
                "expected `.` after `SELECT` entry",
            );
        } else {
            self.advance();
        }

        let end = self
            .previous_span()
            .map(|span| span.end)
            .unwrap_or(start.end);

        Some(FileControlEntry {
            file_name,
            destination,
            span: Span::new(start.file, start.start, end),
        })
    }

    fn take_optional_name(&mut self, stop_words: &[&str]) -> Option<String> {
        if let Some(Token {
            kind: TokenKind::Identifier(name),
            ..
        }) = self.peek()
        {
            if stop_words.iter().any(|word| name == *word) {
                return None;
            }
            let name = name.clone();
            self.advance();

            if self.check_kind(&TokenKind::Period) {
                self.advance();
            }
            return Some(name);
        }

        if let Some(Token {
            kind: TokenKind::StringLiteral(name),
            ..
        }) = self.peek()
        {
            let name = name.clone();
            self.advance();
            return Some(name);
        }

        None
    }

    fn take_identifier_or_string(&mut self) -> Option<ValueWithSpan> {
        match self.peek()? {
            Token {
                kind: TokenKind::Identifier(name),
                span,
            } => {
                let value = ValueWithSpan {
                    name: name.clone(),
                    span: *span,
                };
                self.advance();
                Some(value)
            }
            Token {
                kind: TokenKind::StringLiteral(name),
                span,
            } => {
                let value = ValueWithSpan {
                    name: name.clone(),
                    span: *span,
                };
                self.advance();
                Some(value)
            }
            _ => {
                self.error_current(
                    "cobol::parser::expected_name",
                    "expected an identifier or string literal",
                );
                None
            }
        }
    }

    fn check_identifier(&self, name: &str) -> bool {
        matches!(
            self.peek(),
            Some(Token {
                kind: TokenKind::Identifier(actual),
                ..
            }) if actual == name
        )
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

pub fn parse_environment_division(
    tokens: &[Token],
) -> (Option<EnvironmentDivision>, Vec<Diagnostic>) {
    let mut parser = Parser::new(tokens);
    let division = parser.parse_environment_division();
    (division, parser.diagnostics)
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

    #[test]
    fn reports_missing_division_keyword() {
        let (_, diagnostics) = parse(&tokens("IDENTIFICATION SECTION. PROGRAM-ID. TEST."));

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "cobol::parser::expected_division");
    }

    #[test]
    fn reports_missing_program_name() {
        let (_, diagnostics) = parse(&tokens("IDENTIFICATION DIVISION. PROGRAM-ID 123."));

        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "cobol::parser::missing_program_name"));
    }

    #[test]
    fn reports_missing_period_after_program_name() {
        let (_, diagnostics) = parse(&tokens("IDENTIFICATION DIVISION. PROGRAM-ID. TEST"));

        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "cobol::parser::missing_period"));
    }

    #[test]
    fn reports_missing_identification_for_empty_token_stream() {
        let (_, diagnostics) = parse(&[]);

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(
            diagnostics[0].code,
            "cobol::parser::missing_identification_division"
        );
    }

    #[test]
    fn parse_identification_division_exposes_program_id_span() {
        let (division, diagnostics) = parse_identification_division(&tokens(
            "IDENTIFICATION DIVISION. PROGRAM-ID. TEST.",
        ));

        assert!(diagnostics.is_empty());
        let program_id = division.program_id.expect("missing PROGRAM-ID AST node");
        assert_eq!(program_id.name.as_deref(), Some("TEST"));
        assert!(!division.span.is_empty());
        assert_eq!(division.span.start, 0);
        assert!(division.span.end > division.span.start);
    }

    #[test]
    fn parses_environment_division_with_configuration_and_input_output_sections() {
        let (program, diagnostics) = parse(&tokens(
            "IDENTIFICATION DIVISION. PROGRAM-ID. GREETING. ENVIRONMENT DIVISION. CONFIGURATION SECTION. SOURCE-COMPUTER. IBM-7094. OBJECT-COMPUTER. IBM-7094. INPUT-OUTPUT SECTION. FILE-CONTROL. SELECT INPUT-FILE ASSIGN TO 'input.txt'. SELECT OUTPUT-FILE ASSIGN TO 'output.txt'.",
        ));

        assert!(diagnostics.is_empty());
        assert_eq!(
            program.identification_division.program_name(),
            Some("GREETING")
        );
        assert!(program.environment_division.is_some());
    }

    #[test]
    fn parses_environment_division_without_configuration_section() {
        let (program, diagnostics) = parse(&tokens(
            "IDENTIFICATION DIVISION. PROGRAM-ID. GREETING. ENVIRONMENT DIVISION. INPUT-OUTPUT SECTION. FILE-CONTROL. SELECT INPUT-FILE ASSIGN TO 'input.txt'.",
        ));

        assert!(diagnostics.is_empty());
        assert_eq!(
            program.identification_division.program_name(),
            Some("GREETING")
        );
        assert!(program.environment_division.is_some());
        assert!(program
            .environment_division
            .as_ref()
            .unwrap()
            .configuration_section
            .is_none());
    }

    #[test]
    fn parses_environment_division_without_input_output_section() {
        let (program, diagnostics) = parse(&tokens(
            "IDENTIFICATION DIVISION. PROGRAM-ID. GREETING. ENVIRONMENT DIVISION. CONFIGURATION SECTION. SOURCE-COMPUTER. IBM-7094. OBJECT-COMPUTER. IBM-7094.",
        ));

        assert!(diagnostics.is_empty());
        assert_eq!(
            program.identification_division.program_name(),
            Some("GREETING")
        );
        assert!(program.environment_division.is_some());
        assert!(program
            .environment_division
            .as_ref()
            .unwrap()
            .input_output_section
            .is_none());
    }

    #[test]
    fn reports_missing_configuration_section() {
        let (_, diagnostics) = parse(&tokens(
            "IDENTIFICATION DIVISION. PROGRAM-ID. GREETING. ENVIRONMENT DIVISION. CONFIGURATION SECTION. OBJECT-COMPUTER. IBM-7094.",
        ));

        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "cobol::parser::missing_computer_paragraph"));
    }

    #[test]
    fn reports_missing_input_output_section() {
        let (_, diagnostics) = parse(&tokens(
            "IDENTIFICATION DIVISION. PROGRAM-ID. GREETING. ENVIRONMENT DIVISION. INPUT-OUTPUT SECTION. SELECT OUTPUT-FILE ASSIGN TO 'output.txt'.",
        ));

        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "cobol::parser::missing_file_name"));
    }

    #[test]
    fn reports_missing_select_file_name() {
        let (_, diagnostics) = parse(&tokens(
            "IDENTIFICATION DIVISION. PROGRAM-ID. GREETING. ENVIRONMENT DIVISION. INPUT-OUTPUT SECTION. FILE-CONTROL. SELECT ASSIGN TO 'output.txt'.",
        ))
;

        assert!(diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "cobol::parser::expected_name"
                || diagnostic.code == "cobol::parser::missing_assign"
        }));
    }

    #[test]
    fn reports_missing_period_in_select_statement() {
        let (_, diagnostics) = parse(&tokens(
            "IDENTIFICATION DIVISION. PROGRAM-ID. GREETING. ENVIRONMENT DIVISION. INPUT-OUTPUT SECTION. FILE-CONTROL. SELECT INPUT-FILE ASSIGN TO 'input.txt' SELECT OUTPUT-FILE ASSIGN TO 'output.txt'.",
        ))
;

        assert!(diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "cobol::parser::select_missing_period"
        }));
    }

    #[test]
    fn reports_missing_environment_division_period() {
        let (_, diagnostics) = parse(&tokens(
            "IDENTIFICATION DIVISION. PROGRAM-ID. GREETING. ENVIRONMENT DIVISION CONFIGURATION SECTION.",
        ));

        assert!(diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "cobol::parser::environment_missing_period"
        }));
    }

    #[test]
    fn reports_missing_environment_division_keyword() {
        let (_, diagnostics) = parse(&tokens(
            "IDENTIFICATION DIVISION. PROGRAM-ID. GREETING. ENVIRONMENT SECTION.",
        ))
;

        assert!(diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "cobol::parser::missing_environment_division"
        }));
    }

    #[test]
    fn reports_missing_configuration_section_keyword() {
        let (_, diagnostics) = parse(&tokens(
            "IDENTIFICATION DIVISION. PROGRAM-ID. GREETING. ENVIRONMENT DIVISION. CONFIGURATION DATA SECTION. SOURCE-COMPUTER. LINUX.",
        ))
;

        assert!(diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "cobol::parser::configuration_expected_section"
        }));
    }

    #[test]
    fn reports_missing_file_control_header() {
        let (_, diagnostics) = parse(&tokens(
            "IDENTIFICATION DIVISION. PROGRAM-ID. GREETING. ENVIRONMENT DIVISION. INPUT-OUTPUT SECTION. SELECT OUTPUT-FILE ASSIGN TO 'output.txt'.",
        ))
;

        assert!(diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "cobol::parser::missing_file_control"
        }));
    }

    #[test]
    fn reports_missing_assign_to_clause() {
        let (_, diagnostics) = parse(&tokens(
            "IDENTIFICATION DIVISION. PROGRAM-ID. GREETING. ENVIRONMENT DIVISION. INPUT-OUTPUT SECTION. FILE-CONTROL. SELECT OUTPUT-FILE 'output.txt'.",
        ))
;

        assert!(diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "cobol::parser::missing_assign"
                || diagnostic.code == "cobol::parser::missing_assign_to"
        }));
    }

    #[test]
    fn reports_missing_select_entry_period() {
        let (_, diagnostics) = parse(&tokens(
            "IDENTIFICATION DIVISION. PROGRAM-ID. GREETING. ENVIRONMENT DIVISION. INPUT-OUTPUT SECTION. FILE-CONTROL. SELECT INPUT-FILE ASSIGN TO 'input.txt'",
        ))
;

        assert!(diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "cobol::parser::select_missing_period"
        }));
    }

    #[test]
    fn parses_configuration_section() {
        let (program, diagnostics) = parse(&tokens(
            "IDENTIFICATION DIVISION. PROGRAM-ID. TEST. \
             ENVIRONMENT DIVISION. CONFIGURATION SECTION. \
             SOURCE-COMPUTER. LINUX. OBJECT-COMPUTER. LINUX.",
        ))
;

        assert!(diagnostics.is_empty());
        let environment = program.environment_division.expect("missing ENVIRONMENT DIVISION");
        let configuration = environment.configuration_section.expect("missing CONFIGURATION SECTION");
        assert_eq!(configuration.source_computer.unwrap().name.as_deref(), Some("LINUX"));
        assert_eq!(configuration.object_computer.unwrap().name.as_deref(), Some("LINUX"));
    }

    #[test]
    fn parses_input_output_file_control() {
        let (program, diagnostics) = parse(&tokens(
            "IDENTIFICATION DIVISION. PROGRAM-ID. TEST. \
             ENVIRONMENT DIVISION. INPUT-OUTPUT SECTION. FILE-CONTROL. \
             SELECT CUSTOMER-FILE ASSIGN TO \"customers.dat\".",
        ))
;

        assert!(diagnostics.is_empty());
        let environment = program.environment_division.expect("missing ENVIRONMENT DIVISION");
        let input_output = environment.input_output_section.expect("missing INPUT-OUTPUT SECTION");
        assert_eq!(input_output.file_control.len(), 1);
        let entry = &input_output.file_control[0];
        assert_eq!(entry.file_name, "CUSTOMER-FILE");
        assert_eq!(entry.destination.as_deref(), Some("customers.dat"));
    }

    #[test]
    fn parses_environment_division_in_isolation() {
        let (division, diagnostics) = parse_environment_division(&tokens(
            "ENVIRONMENT DIVISION. CONFIGURATION SECTION. SOURCE-COMPUTER. LINUX.",
        ))
;

        assert!(diagnostics.is_empty());
        let division = division.expect("missing ENVIRONMENT DIVISION");
        let configuration = division
            .configuration_section
            .expect("missing CONFIGURATION SECTION");
        assert_eq!(
            configuration.source_computer.unwrap().name.as_deref(),
            Some("LINUX")
        );
    }

    #[test]
    fn parses_multiple_file_control_entries() {
        let (program, diagnostics) = parse(&tokens(
            "IDENTIFICATION DIVISION. PROGRAM-ID. TEST. ENVIRONMENT DIVISION. INPUT-OUTPUT SECTION. FILE-CONTROL. SELECT INPUT-FILE ASSIGN TO 'input.txt'. SELECT OUTPUT-FILE ASSIGN TO 'output.txt'.",
        ))
;

        assert!(diagnostics.is_empty());
        let entries = &program
            .environment_division
            .unwrap()
            .input_output_section
            .unwrap()
            .file_control;
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].file_name, "INPUT-FILE");
        assert_eq!(entries[1].file_name, "OUTPUT-FILE");
    }

    #[test]
    fn configuration_section_allows_only_object_computer() {
        let (program, diagnostics) = parse(&tokens(
            "IDENTIFICATION DIVISION. PROGRAM-ID. GREETING. ENVIRONMENT DIVISION. CONFIGURATION SECTION. OBJECT-COMPUTER. IBM-7094.",
        ))
;

        assert!(diagnostics.is_empty());
        let configuration = program
            .environment_division
            .unwrap()
            .configuration_section
            .unwrap();
        assert!(configuration.source_computer.is_none());
        assert_eq!(
            configuration.object_computer.unwrap().name.as_deref(),
            Some("IBM-7094")
        );
    }

    #[test]
    fn input_output_section_accepts_empty_file_control() {
        let (program, diagnostics) = parse(&tokens(
            "IDENTIFICATION DIVISION. PROGRAM-ID. GREETING. ENVIRONMENT DIVISION. INPUT-OUTPUT SECTION. FILE-CONTROL.",
        ))
;

        assert!(diagnostics.is_empty());
        let input_output = program
            .environment_division
            .unwrap()
            .input_output_section
            .unwrap();
        assert!(input_output.file_control.is_empty());
    }

    #[test]
    fn accepts_program_without_environment_division() {
        let (program, diagnostics) = parse(&tokens(
            "IDENTIFICATION DIVISION. PROGRAM-ID. GREETING.",
        ))
;

        assert!(diagnostics.is_empty());
        assert!(program.environment_division.is_none());
    }
}
