use crate::diagnostics::Diagnostic;
use crate::lexer::{Keyword, Token, TokenKind};
use crate::source::Span;

use super::ast::*;
use super::errors::ParseError;

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    diagnostics: Vec<Diagnostic>,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            pos: 0,
            diagnostics: Vec::new(),
        }
    }

    fn parse_function_call(&mut self, start: Span) -> Result<Expression, ParseError> {
        self.advance();
        let (name, _) = self.expect_identifier()?;
        if !self
            .current_token()
            .is_some_and(|token| matches!(token.kind, TokenKind::LParen))
        {
            return Err(ParseError::InvalidStatement);
        }
        self.advance();

        let mut arguments = Vec::new();
        while !self
            .current_token()
            .is_none_or(|token| matches!(token.kind, TokenKind::RParen))
        {
            arguments.push(self.parse_expression()?);
            if self
                .current_token()
                .is_some_and(|token| matches!(token.kind, TokenKind::Comma))
            {
                self.advance();
            } else {
                break;
            }
        }

        let end = if let Some(token) = self.current_token()
            && matches!(token.kind, TokenKind::RParen)
        {
            let end = token.span.end;
            self.advance();
            end
        } else {
            return Err(ParseError::InvalidStatement);
        };
        Ok(Expression::FunctionCall {
            name,
            arguments,
            span: Span::new(start.file, start.start, end),
        })
    }

    fn is_expression_start(&self) -> bool {
        self.current_token().is_some_and(|token| {
            matches!(
                token.kind,
                TokenKind::Identifier(_)
                    | TokenKind::NumericLiteral(_)
                    | TokenKind::StringLiteral(_)
                    | TokenKind::LParen
                    | TokenKind::Plus
                    | TokenKind::Minus
            )
        })
    }

    fn current_token(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn advance(&mut self) {
        self.pos += 1;
    }

    fn expect_keyword(&mut self, keyword: Keyword) -> Result<Token, ParseError> {
        if let Some(token) = self.current_token()
            && matches!(&token.kind, TokenKind::Keyword(k) if *k == keyword)
        {
            let result = token.clone();
            self.advance();
            return Ok(result);
        }
        Err(ParseError::unexpected_token(
            keyword.as_str(),
            &self
                .current_token()
                .map(|t| t.kind.clone())
                .unwrap_or(TokenKind::Period),
        ))
    }

    fn expect_period(&mut self) -> Result<Span, ParseError> {
        if let Some(token) = self.current_token()
            && matches!(token.kind, TokenKind::Period)
        {
            let span = token.span;
            self.advance();
            return Ok(span);
        }
        Err(ParseError::MissingPeriod {
            context: "statement".to_string(),
        })
    }

    fn expect_identifier(&mut self) -> Result<(String, Span), ParseError> {
        if let Some(token) = self.current_token()
            && let TokenKind::Identifier(name) = &token.kind
        {
            let result = (name.clone(), token.span);
            self.advance();
            return Ok(result);
        }
        Err(ParseError::unexpected_token(
            "identifier",
            &self
                .current_token()
                .map(|t| t.kind.clone())
                .unwrap_or(TokenKind::Period),
        ))
    }

    fn consume_identifier(&mut self) -> Option<(String, Span)> {
        if let Some(token) = self.current_token()
            && let TokenKind::Identifier(name) = &token.kind
        {
            let result = (name.clone(), token.span);
            self.advance();
            return Some(result);
        }
        None
    }

    fn consume_comment_entry(&mut self) -> Option<String> {
        if let Some(token) = self.current_token()
            && let TokenKind::CommentEntry(text) = &token.kind
        {
            let result = text.clone();
            self.advance();
            return Some(result);
        }
        None
    }

    pub fn parse(mut self) -> (Program, Vec<Diagnostic>) {
        match self.parse_program() {
            Ok(program) => (program, self.diagnostics),
            Err(e) => {
                self.report_parse_error(self.pos, e.to_string());
                // Return minimal program
                (
                    Program {
                        identification_division: IdentificationDivision {
                            span: Span::new(crate::source::FileId(0), 0, 0),
                            program_id: "UNKNOWN".to_string(),
                            author: None,
                            installation: None,
                            date_written: None,
                            date_compiled: None,
                            security: None,
                            remarks: None,
                        },
                        environment_division: None,
                        data_division: None,
                        procedure_division: None,
                    },
                    self.diagnostics,
                )
            }
        }
    }

    fn parse_program(&mut self) -> Result<Program, ParseError> {
        let identification_division = self.parse_identification_division()?;
        let environment_start = self.pos;
        let environment_division = self.try_parse_environment_division();
        if environment_division.is_some() {
            self.recover_to_next_division(true, "ENVIRONMENT DIVISION");
        } else if self.pos != environment_start {
            self.report_unsupported(environment_start, "malformed ENVIRONMENT DIVISION");
            self.recover_to_next_division(true, "ENVIRONMENT DIVISION");
        }

        let data_start = self.pos;
        let data_division = self.try_parse_data_division();
        if data_division.is_some() {
            self.recover_to_next_division(false, "DATA DIVISION");
        } else if self.pos != data_start {
            self.report_unsupported(data_start, "malformed DATA DIVISION");
            self.recover_to_next_division(false, "DATA DIVISION");
        }

        let procedure_start = self.pos;
        let procedure_division = self.try_parse_procedure_division();
        if procedure_division.is_none() && self.pos != procedure_start {
            self.report_unsupported(self.pos, "malformed PROCEDURE DIVISION");
            self.recover_to_end_program();
        }
        self.try_parse_end_program();
        if self.current_token().is_some() {
            self.report_unsupported(self.pos, "trailing top-level syntax");
            while self.current_token().is_some() {
                self.advance();
            }
        }

        Ok(Program {
            identification_division,
            environment_division,
            data_division,
            procedure_division,
        })
    }

    fn report_parse_error(&mut self, position: usize, message: impl Into<String>) {
        let diagnostic = Diagnostic::error("cobol::parser::parse_error", message);
        self.diagnostics.push(
            self.tokens
                .get(position)
                .map_or(diagnostic.clone(), |token| diagnostic.with_span(token.span)),
        );
    }

    fn report_unsupported(&mut self, position: usize, context: &str) {
        let Some(token) = self.tokens.get(position) else {
            return;
        };
        self.diagnostics.push(
            Diagnostic::error(
                "cobol::parser::unsupported_syntax",
                format!("unsupported or unparsed {context}: {}", token.kind),
            )
            .with_span(token.span)
            .with_label("skipped while recovering"),
        );
    }

    fn recover_to_next_division(&mut self, allow_data: bool, context: &str) {
        if self.at_next_major_division(allow_data) {
            return;
        }
        self.report_unsupported(self.pos, context);
        while self.current_token().is_some() && !self.at_next_major_division(allow_data) {
            self.advance();
        }
    }

    fn at_next_major_division(&self, allow_data: bool) -> bool {
        self.current_token().is_none()
            || (allow_data && self.is_keyword_at(Keyword::Data))
            || self.is_keyword_at(Keyword::Procedure)
            || self.is_end_program_start()
    }

    fn recover_to_working_storage(&mut self, report: bool) {
        if self.current_token().is_none()
            || self.is_identifier_at("WORKING-STORAGE")
            || self.is_keyword_at(Keyword::Procedure)
            || self.is_end_program_start()
        {
            return;
        }

        if report {
            self.report_unsupported(self.pos, "DATA DIVISION syntax before WORKING-STORAGE");
        }
        while self.current_token().is_some()
            && !self.is_identifier_at("WORKING-STORAGE")
            && !self.is_keyword_at(Keyword::Procedure)
            && !self.is_end_program_start()
        {
            self.advance();
        }
    }

    fn is_end_program_start(&self) -> bool {
        self.is_identifier_at("END")
            && self.tokens.get(self.pos + 1).is_some_and(
                |token| matches!(&token.kind, TokenKind::Identifier(name) if name == "PROGRAM"),
            )
    }

    fn try_parse_end_program(&mut self) {
        if !self.is_end_program_start() {
            return;
        }
        let start = self.pos;
        self.advance();
        self.advance();
        if self.is_identifier_at_next() {
            self.advance();
        } else {
            self.report_parse_error(start, "expected program name after END PROGRAM");
        }
        if self
            .current_token()
            .is_some_and(|token| matches!(token.kind, TokenKind::Period))
        {
            self.advance();
        } else {
            self.report_parse_error(start, "expected period after END PROGRAM");
        }
    }

    fn recover_to_end_program(&mut self) {
        while self.current_token().is_some() && !self.is_end_program_start() {
            self.advance();
        }
    }

    fn parse_identification_division(&mut self) -> Result<IdentificationDivision, ParseError> {
        let div_start = self.expect_keyword(Keyword::Identification)?;
        self.expect_keyword(Keyword::Division)?;
        self.expect_period()?;

        let (program_id_word, _) = self
            .consume_identifier()
            .ok_or(ParseError::MissingProgramId)?;

        let program_id = if program_id_word == "PROGRAM-ID" {
            self.expect_period()?;
            let (name, _) = self.expect_identifier()?;
            self.expect_period()?;
            name
        } else {
            self.expect_period()?;
            program_id_word
        };

        let mut author = None;
        let mut installation = None;
        let mut date_written = None;
        let mut date_compiled = None;
        let mut security = None;
        let mut remarks = None;

        // Parse optional comment entries
        while let Some((word, _)) = self.consume_identifier() {
            match word.as_str() {
                "AUTHOR" => {
                    self.expect_period()?;
                    author = self.consume_comment_entry();
                    self.expect_period()?;
                }
                "INSTALLATION" => {
                    self.expect_period()?;
                    installation = self.consume_comment_entry();
                    self.expect_period()?;
                }
                "DATE-WRITTEN" => {
                    self.expect_period()?;
                    date_written = self.consume_comment_entry();
                    self.expect_period()?;
                }
                "DATE-COMPILED" => {
                    self.expect_period()?;
                    date_compiled = self.consume_comment_entry();
                    self.expect_period()?;
                }
                "SECURITY" => {
                    self.expect_period()?;
                    security = self.consume_comment_entry();
                    self.expect_period()?;
                }
                "REMARKS" => {
                    self.expect_period()?;
                    remarks = self.consume_comment_entry();
                    self.expect_period()?;
                }
                _ => {
                    break;
                }
            }
        }

        Ok(IdentificationDivision {
            span: div_start.span,
            program_id,
            author,
            installation,
            date_written,
            date_compiled,
            security,
            remarks,
        })
    }

    fn try_parse_environment_division(&mut self) -> Option<EnvironmentDivision> {
        if !self.is_keyword_at(Keyword::Environment) {
            return None;
        }

        let div_start = self.current_token()?.span;
        self.advance();
        self.expect_keyword(Keyword::Division).ok()?;
        self.expect_period().ok()?;

        let configuration_start = self.pos;
        let configuration_section = self.try_parse_configuration_section();
        if configuration_section.is_none() && self.pos != configuration_start {
            self.report_unsupported(configuration_start, "malformed CONFIGURATION SECTION");
        }
        let input_output_start = self.pos;
        let input_output_section = self.try_parse_input_output_section();
        if input_output_section.is_none() && self.pos != input_output_start {
            self.report_unsupported(input_output_start, "malformed INPUT-OUTPUT SECTION");
        }

        Some(EnvironmentDivision {
            span: div_start,
            configuration_section,
            input_output_section,
        })
    }

    fn try_parse_configuration_section(&mut self) -> Option<ConfigurationSection> {
        if !self.is_identifier_at("CONFIGURATION") {
            return None;
        }

        let span = self.current_token()?.span;
        self.advance();
        self.expect_keyword(Keyword::Section).ok()?;
        self.expect_period().ok()?;

        Some(ConfigurationSection { span })
    }

    fn try_parse_input_output_section(&mut self) -> Option<InputOutputSection> {
        if !self.is_identifier_at("INPUT-OUTPUT") {
            return None;
        }

        let span = self.current_token()?.span;
        self.advance();
        self.expect_keyword(Keyword::Section).ok()?;
        self.expect_period().ok()?;

        let mut file_controls = Vec::new();
        if self.is_identifier_at("FILE-CONTROL") {
            self.advance();
            self.expect_period().ok()?;
            while self.is_identifier_at("SELECT") {
                file_controls.push(self.parse_file_control()?);
            }
        }

        Some(InputOutputSection {
            span,
            file_controls,
        })
    }

    fn parse_file_control(&mut self) -> Option<FileControl> {
        let span = self.current_token()?.span;
        self.advance();
        let (file_name, _) = self.consume_identifier()?;
        let mut entry = FileControl {
            span,
            file_name,
            assign_to: None,
            organization: None,
            access_mode: None,
            record_key: None,
            alternate_keys: Vec::new(),
            file_status: None,
        };

        loop {
            self.current_token()?;
            if self.consume_identifier_matching("ASSIGN") {
                self.consume_identifier_matching("TO");
                entry.assign_to = self.parse_primary_expression().ok();
            } else if self.consume_identifier_matching("ORGANIZATION") {
                self.consume_identifier_matching("IS");
                entry.organization = self.consume_identifier().map(|(word, _)| word);
            } else if self.consume_identifier_matching("ACCESS") {
                self.consume_identifier_matching("MODE");
                self.consume_identifier_matching("IS");
                entry.access_mode = self.consume_identifier().map(|(word, _)| word);
            } else if self.consume_identifier_matching("RECORD") {
                self.consume_identifier_matching("KEY");
                self.consume_identifier_matching("IS");
                entry.record_key = self.parse_primary_expression().ok();
            } else if self.consume_identifier_matching("ALTERNATE") {
                self.consume_identifier_matching("RECORD");
                self.consume_identifier_matching("KEY");
                self.consume_identifier_matching("IS");
                if let Ok(key) = self.parse_primary_expression() {
                    entry.alternate_keys.push(key);
                }
                if self.consume_identifier_matching("WITH") {
                    self.consume_identifier_matching("DUPLICATES");
                }
            } else if self.consume_identifier_matching("FILE") {
                self.consume_identifier_matching("STATUS");
                self.consume_identifier_matching("IS");
                entry.file_status = self.parse_primary_expression().ok();
            } else {
                break;
            }
        }
        self.expect_period().ok()?;
        Some(entry)
    }

    fn try_parse_data_division(&mut self) -> Option<DataDivision> {
        if !self.is_keyword_at(Keyword::Data) {
            return None;
        }

        let div_start = self.current_token()?.span;
        self.advance();
        self.expect_keyword(Keyword::Division).ok()?;
        self.expect_period().ok()?;

        let file_section_start = self.pos;
        let file_section = self.try_parse_file_section();
        let file_section_failed = file_section.is_none() && self.pos != file_section_start;
        if file_section_failed {
            self.report_unsupported(file_section_start, "malformed FILE SECTION");
        }
        self.recover_to_working_storage(!file_section_failed);
        let working_storage_start = self.pos;
        let working_storage_section = self.try_parse_working_storage_section();
        if working_storage_section.is_none() && self.pos != working_storage_start {
            self.report_unsupported(working_storage_start, "malformed WORKING-STORAGE SECTION");
        }

        Some(DataDivision {
            span: div_start,
            file_section,
            working_storage_section,
        })
    }

    fn try_parse_file_section(&mut self) -> Option<FileSection> {
        if !self.is_identifier_at("FILE") {
            return None;
        }

        let span = self.current_token()?.span;
        self.advance();
        self.expect_keyword(Keyword::Section).ok()?;
        self.expect_period().ok()?;

        let mut file_descriptions = Vec::new();
        while self.is_identifier_at("FD") {
            self.advance();
            if let Ok((file_name, name_span)) = self.expect_identifier() {
                let clauses = self.parse_file_clauses();
                self.expect_period().ok()?;
                let records = self.parse_data_items();
                file_descriptions.push(FileDescription {
                    span: name_span,
                    file_name,
                    clauses,
                    records,
                });
            }
        }

        Some(FileSection {
            span,
            file_descriptions,
        })
    }

    fn parse_file_clauses(&mut self) -> Vec<FileClause> {
        let mut clauses = Vec::new();
        loop {
            let is_block = self.is_identifier_at("BLOCK");
            if !is_block && !self.is_identifier_at("RECORD") {
                break;
            }
            let span = self.tokens[self.pos].span;
            self.advance();
            self.consume_identifier_matching("CONTAINS");
            let mut size = String::new();
            while let Some(TokenKind::NumericLiteral(number)) =
                self.current_token().map(|token| &token.kind)
            {
                size.push_str(number);
                self.advance();
                if self.consume_identifier_matching("TO") {
                    size.push_str(" TO ");
                }
            }
            for unit in ["RECORDS", "RECORD", "CHARACTERS"] {
                if self.consume_identifier_matching(unit) {
                    break;
                }
            }
            clauses.push(if is_block {
                FileClause::BlockContains(size, span)
            } else {
                FileClause::RecordContains(size, span)
            });
        }
        clauses
    }

    fn try_parse_working_storage_section(&mut self) -> Option<WorkingStorageSection> {
        if !self.is_identifier_at("WORKING-STORAGE") {
            return None;
        }

        let span = self.current_token()?.span;
        self.advance();
        self.expect_keyword(Keyword::Section).ok()?;
        self.expect_period().ok()?;

        let data_items = self.parse_data_items();

        Some(WorkingStorageSection { span, data_items })
    }

    fn parse_data_items(&mut self) -> Vec<DataItem> {
        let mut items = Vec::new();
        while let Some(item) = self.try_parse_data_item(0) {
            items.push(item);
        }
        items
    }

    fn try_parse_data_item(&mut self, parent_level: u8) -> Option<DataItem> {
        if !self.is_level_number_at() {
            return None;
        }

        // Peek at the level number to decide if this item belongs to us
        let level = self.peek_level_number()?;
        if level <= parent_level || (level == 77 && parent_level != 0) {
            return None;
        }

        // Now actually consume the level number
        self.parse_level_number()?;

        let span = self.current_token()?.span;
        let (name, _) = self.consume_identifier()?;

        let mut picture = None;
        let mut value = None;
        let mut occurs = None;
        let mut indexed_by = Vec::new();

        while let Some(token) = self.current_token() {
            match token.kind {
                TokenKind::Picture => {
                    self.advance();
                    if let Some(next) = self.current_token()
                        && matches!(next.kind, TokenKind::Identifier(_))
                    {
                        self.advance();
                    }
                    if let Some(next) = self.current_token()
                        && let TokenKind::PictureString(ref s) = next.kind
                    {
                        picture = Some(s.clone());
                        self.advance();
                    }
                }
                TokenKind::Identifier(ref id) if id == "VALUE" || id == "VALUES" => {
                    self.advance();
                    if let Some(lit) = self.parse_literal() {
                        value = Some(lit);
                    }
                }
                TokenKind::Identifier(ref id) if id == "OCCURS" => {
                    self.advance();
                    if let Some(TokenKind::NumericLiteral(count)) =
                        self.current_token().map(|token| token.kind.clone())
                    {
                        occurs = Some(count);
                        self.advance();
                    }
                    self.consume_identifier_matching("TIMES");
                }
                TokenKind::Identifier(ref id) if id == "INDEXED" => {
                    self.advance();
                    self.consume_identifier_matching("BY");
                    while self.is_identifier_at_next()
                        && !self.is_identifier_at("VALUE")
                        && !self.is_identifier_at("VALUES")
                        && !self.is_identifier_at("OCCURS")
                    {
                        if let Some((index_name, _)) = self.consume_identifier() {
                            indexed_by.push(index_name);
                        }
                    }
                }
                TokenKind::Period => {
                    self.advance();
                    break;
                }
                _ => break,
            }
        }

        let mut children = Vec::new();
        while let Some(item) = self.try_parse_data_item(level) {
            children.push(item);
        }

        Some(DataItem {
            span,
            level,
            name,
            picture,
            value,
            occurs,
            indexed_by,
            children,
        })
    }

    fn parse_level_number(&mut self) -> Option<u8> {
        if let Some(token) = self.current_token()
            && let TokenKind::NumericLiteral(s) = &token.kind
            && let Ok(level) = s.parse::<u8>()
            && level <= 99
        {
            self.advance();
            return Some(level);
        }
        None
    }

    fn peek_level_number(&self) -> Option<u8> {
        if let Some(token) = self.current_token()
            && let TokenKind::NumericLiteral(s) = &token.kind
            && let Ok(level) = s.parse::<u8>()
            && level <= 99
        {
            return Some(level);
        }
        None
    }

    fn parse_literal(&mut self) -> Option<Literal> {
        if let Some(token) = self.current_token() {
            let result = match &token.kind {
                TokenKind::NumericLiteral(s) => Some(Literal::Numeric(s.clone(), token.span)),
                TokenKind::StringLiteral(s) => Some(Literal::String(s.clone(), token.span)),
                TokenKind::Identifier(name) if is_figurative_constant(name) => {
                    Some(Literal::FigurativeConstant(name.clone(), token.span))
                }
                _ => None,
            };
            if result.is_some() {
                self.advance();
            }
            result
        } else {
            None
        }
    }

    fn try_parse_procedure_division(&mut self) -> Option<ProcedureDivision> {
        if !self.is_keyword_at(Keyword::Procedure) {
            return None;
        }

        let div_start = self.current_token()?.span;
        self.advance();
        self.expect_keyword(Keyword::Division).ok()?;
        self.expect_period().ok()?;

        let mut paragraphs = Vec::new();
        while let Some(para) = self.try_parse_paragraph() {
            paragraphs.push(para);
        }

        Some(ProcedureDivision {
            span: div_start,
            paragraphs,
        })
    }

    fn try_parse_paragraph(&mut self) -> Option<Paragraph> {
        if !self.is_paragraph_start() {
            return None;
        }

        let (name, span) = self.consume_identifier()?;
        self.expect_period().ok()?;

        let mut statements = Vec::new();
        loop {
            if self.is_paragraph_start()
                || self.is_keyword_at(Keyword::Procedure)
                || self.is_end_program_start()
            {
                break;
            }
            if self
                .current_token()
                .is_some_and(|token| matches!(token.kind, TokenKind::Period | TokenKind::Semicolon))
            {
                self.advance();
                continue;
            }

            let before = self.pos;
            if let Some(statement) = self.parse_statement_or_recover() {
                statements.push(statement);
            } else if self.pos == before {
                break;
            }
        }

        Some(Paragraph {
            span,
            name,
            statements,
        })
    }

    fn try_parse_statement(&mut self) -> Result<Option<Statement>, ParseError> {
        let parsed = match self.current_token().map(|token| &token.kind) {
            Some(TokenKind::Keyword(Keyword::Move)) => Some(self.parse_move_statement()),
            Some(TokenKind::Keyword(Keyword::Display)) => Some(self.parse_display_statement()),
            Some(TokenKind::Keyword(Keyword::Accept)) => Some(self.parse_accept_statement()),
            Some(TokenKind::Keyword(Keyword::Perform)) => Some(self.parse_perform_statement()),
            Some(TokenKind::Keyword(Keyword::If)) => Some(self.parse_if_statement()),
            Some(TokenKind::Keyword(Keyword::Stop)) => Some(self.parse_stop_run_statement()),
            Some(TokenKind::Keyword(Keyword::Run)) => {
                let span = self.current_token().expect("token was matched").span;
                self.advance();
                Some(Ok(Statement::StopRun(span)))
            }
            Some(TokenKind::Identifier(id)) if matches!(id.as_str(), "GO" | "GOBACK" | "GOTO") => {
                Some(self.parse_goto_statement())
            }
            Some(TokenKind::Identifier(id)) if id == "ADD" => Some(self.parse_add_statement()),
            Some(TokenKind::Identifier(id)) if id == "SUBTRACT" => {
                Some(self.parse_subtract_statement())
            }
            Some(TokenKind::Identifier(id)) if id == "MULTIPLY" => {
                Some(self.parse_multiply_statement())
            }
            Some(TokenKind::Identifier(id)) if id == "DIVIDE" => {
                Some(self.parse_divide_statement())
            }
            Some(TokenKind::Identifier(id)) if id == "COMPUTE" => {
                Some(self.parse_compute_statement())
            }
            Some(TokenKind::Identifier(id)) if id == "SET" => Some(self.parse_set_statement()),
            Some(TokenKind::Identifier(id)) if id == "SEARCH" => {
                Some(self.parse_search_statement())
            }
            Some(TokenKind::Identifier(id)) if id == "INSPECT" => {
                Some(self.parse_inspect_statement())
            }
            Some(TokenKind::Identifier(id)) if id == "EVALUATE" => {
                Some(self.parse_evaluate_statement())
            }
            Some(TokenKind::Identifier(id)) if id == "OPEN" => Some(self.parse_open_statement()),
            Some(TokenKind::Identifier(id)) if id == "CLOSE" => Some(self.parse_close_statement()),
            Some(TokenKind::Identifier(id)) if id == "READ" => Some(self.parse_read_statement()),
            Some(TokenKind::Identifier(id)) if id == "WRITE" => Some(self.parse_write_statement()),
            Some(TokenKind::Identifier(id)) if id == "START" => Some(self.parse_start_statement()),
            Some(TokenKind::Identifier(id)) if id == "INITIALIZE" => {
                Some(self.parse_initialize_statement())
            }
            _ => None,
        };
        parsed.transpose()
    }

    fn parse_statement_or_recover(&mut self) -> Option<Statement> {
        let start = self.pos;
        match self.try_parse_statement() {
            Ok(Some(statement)) => Some(statement),
            Ok(None) if self.at_statement_boundary() => None,
            Ok(None) => {
                self.report_unsupported(start, "procedure statement");
                self.recover_statement(start);
                None
            }
            Err(error) => {
                self.report_parse_error(start, error.to_string());
                self.recover_statement(start);
                None
            }
        }
    }

    fn at_statement_boundary(&self) -> bool {
        self.current_token()
            .is_none_or(|token| matches!(token.kind, TokenKind::Period | TokenKind::Semicolon))
            || self.is_keyword_at(Keyword::Else)
            || self.at_clause_boundary()
            || self.is_paragraph_start()
            || self.is_keyword_at(Keyword::Procedure)
            || self.is_end_program_start()
    }

    /// Words that close or continue an enclosing statement (`WHEN`, `AT END`, `END-IF`...).
    fn at_clause_boundary(&self) -> bool {
        self.is_identifier_at("WHEN")
            || self.is_phrase_at(&["AT", "END"])
            || self.is_phrase_at(&["NOT", "AT", "END"])
            || self.is_phrase_at(&["INVALID", "KEY"])
            || self.is_phrase_at(&["NOT", "INVALID", "KEY"])
            || matches!(
                self.current_token().map(|token| &token.kind),
                Some(TokenKind::Identifier(name)) if matches!(
                    name.as_str(),
                    "END-IF"
                        | "END-SEARCH"
                        | "END-EVALUATE"
                        | "END-READ"
                        | "END-WRITE"
                        | "END-START"
                        | "END-PERFORM"
                )
            )
    }

    fn is_phrase_at(&self, words: &[&str]) -> bool {
        words.iter().enumerate().all(|(offset, word)| {
            self.tokens.get(self.pos + offset).is_some_and(
                |token| matches!(&token.kind, TokenKind::Identifier(name) if name == word),
            )
        })
    }

    fn parse_statement_block(&mut self) -> Vec<Statement> {
        let mut statements = Vec::new();
        while !self.at_statement_boundary() {
            let before = self.pos;
            if let Some(statement) = self.parse_statement_or_recover() {
                statements.push(statement);
            } else if before == self.pos {
                break;
            }
        }
        statements
    }

    fn at_recovery_boundary(&self) -> bool {
        self.current_token()
            .is_none_or(|token| matches!(token.kind, TokenKind::Period | TokenKind::Semicolon))
            || self.is_keyword_at(Keyword::Else)
            || self.at_clause_boundary()
            || self.is_keyword_at(Keyword::Procedure)
            || self.is_end_program_start()
    }

    fn recover_statement(&mut self, start: usize) {
        let block = match self.tokens.get(start).map(|token| &token.kind) {
            Some(TokenKind::Identifier(name)) if name == "EVALUATE" => {
                Some(("EVALUATE", "END-EVALUATE"))
            }
            Some(TokenKind::Identifier(name)) if name == "SEARCH" => Some(("SEARCH", "END-SEARCH")),
            _ => None,
        };
        if let Some((opening, closing)) = block {
            let mut depth = 1_usize;
            self.advance();
            while let Some(token) = self.current_token() {
                match &token.kind {
                    TokenKind::Identifier(name) if name == opening => {
                        depth += 1;
                        self.advance();
                    }
                    TokenKind::Identifier(name) if name == closing => {
                        depth -= 1;
                        self.advance();
                        if depth == 0 {
                            if self.current_token().is_some_and(|token| {
                                matches!(token.kind, TokenKind::Period | TokenKind::Semicolon)
                            }) {
                                self.advance();
                            }
                            return;
                        }
                    }
                    _ => self.advance(),
                }
            }
            return;
        }

        if self.pos == start {
            self.advance();
        }
        while self.current_token().is_some() {
            if self.at_recovery_boundary() {
                if self.current_token().is_some_and(|token| {
                    matches!(token.kind, TokenKind::Period | TokenKind::Semicolon)
                }) {
                    self.advance();
                }
                return;
            }
            if self.is_statement_start() {
                return;
            }
            self.advance();
        }
    }

    fn parse_move_statement(&mut self) -> Result<Statement, ParseError> {
        let span = self.expect_keyword(Keyword::Move)?.span;
        let source = self.parse_expression()?;
        self.consume_identifier_matching("TO");
        let targets = self.parse_expression_list_until(&["ON"])?;
        Ok(Statement::Move(MoveStatement {
            span,
            source,
            targets,
        }))
    }

    fn parse_compute_statement(&mut self) -> Result<Statement, ParseError> {
        let span = self.consume_verb("COMPUTE")?;
        let target = self.parse_expression()?;
        if self.consume_relational_operator().map(|(op, _)| op) != Some(BinaryOperator::Equal) {
            return Err(ParseError::InvalidStatement);
        }
        let expression = self.parse_expression()?;
        Ok(Statement::Compute(ComputeStatement {
            span,
            target,
            expression,
        }))
    }

    fn parse_add_statement(&mut self) -> Result<Statement, ParseError> {
        let span = self.consume_verb("ADD")?;
        let operands = self.parse_expression_list_until(&["TO", "GIVING", "ON"])?;
        let (giving, targets, giving_targets) = if self.consume_identifier_matching("TO") {
            let targets = self.parse_expression_list_until(&["GIVING", "ON"])?;
            let giving_targets = if self.consume_identifier_matching("GIVING") {
                self.parse_expression_list_until(&["ON"])?
            } else {
                Vec::new()
            };
            (false, targets, giving_targets)
        } else if self.consume_identifier_matching("GIVING") {
            (true, self.parse_expression_list_until(&["ON"])?, Vec::new())
        } else {
            return Err(ParseError::InvalidStatement);
        };
        let on_size_error = self.parse_on_size_error()?;

        Ok(Statement::Add(AddStatement {
            span,
            operands,
            targets,
            giving,
            giving_targets,
            on_size_error,
        }))
    }

    fn parse_subtract_statement(&mut self) -> Result<Statement, ParseError> {
        let span = self.consume_verb("SUBTRACT")?;
        let operands = self.parse_expression_list_until(&["FROM", "ON"])?;
        if !self.consume_identifier_matching("FROM") {
            return Err(ParseError::InvalidStatement);
        }
        let from = self.parse_expression_list_until(&["GIVING", "ON"])?;
        let giving = if self.consume_identifier_matching("GIVING") {
            self.parse_expression_list_until(&["ON"])?
        } else {
            Vec::new()
        };
        let on_size_error = self.parse_on_size_error()?;

        Ok(Statement::Subtract(SubtractStatement {
            span,
            operands,
            from,
            giving,
            on_size_error,
        }))
    }

    fn parse_multiply_statement(&mut self) -> Result<Statement, ParseError> {
        let span = self.consume_verb("MULTIPLY")?;
        let factor = self.parse_expression()?;
        if !self.consume_identifier_matching("BY") {
            return Err(ParseError::InvalidStatement);
        }
        let by = self.parse_expression_list_until(&["GIVING", "ON"])?;
        let giving = if self.consume_identifier_matching("GIVING") {
            self.parse_expression_list_until(&["ON"])?
        } else {
            Vec::new()
        };
        let on_size_error = self.parse_on_size_error()?;

        Ok(Statement::Multiply(MultiplyStatement {
            span,
            factor,
            by,
            giving,
            on_size_error,
        }))
    }

    fn parse_divide_statement(&mut self) -> Result<Statement, ParseError> {
        let span = self.consume_verb("DIVIDE")?;
        let dividend = self.parse_expression()?;
        if !self.consume_identifier_matching("BY") {
            return Err(ParseError::InvalidStatement);
        }
        let divisor = self.parse_expression()?;
        let giving = if self.consume_identifier_matching("GIVING") {
            self.parse_expression_list_until(&["REMAINDER", "ON"])?
        } else {
            Vec::new()
        };
        let remainder = if self.consume_identifier_matching("REMAINDER") {
            Some(self.parse_expression()?)
        } else {
            None
        };
        let on_size_error = self.parse_on_size_error()?;

        Ok(Statement::Divide(DivideStatement {
            span,
            dividend,
            divisor,
            giving,
            remainder,
            on_size_error,
        }))
    }

    fn consume_verb(&mut self, verb: &str) -> Result<Span, ParseError> {
        if !self.consume_identifier_matching(verb) {
            return Err(ParseError::InvalidStatement);
        }
        Ok(self.tokens[self.pos - 1].span)
    }

    fn parse_expression_list_until(
        &mut self,
        stop_words: &[&str],
    ) -> Result<Vec<Expression>, ParseError> {
        let mut expressions = Vec::new();
        loop {
            if self.at_expression_list_end(stop_words) {
                break;
            }
            if self
                .current_token()
                .is_some_and(|token| matches!(token.kind, TokenKind::Comma | TokenKind::Semicolon))
            {
                self.advance();
                continue;
            }
            expressions.push(self.parse_expression()?);
        }
        if expressions.is_empty() {
            return Err(ParseError::InvalidStatement);
        }
        Ok(expressions)
    }

    fn at_expression_list_end(&self, stop_words: &[&str]) -> bool {
        if self.current_token().is_none_or(|token| {
            matches!(
                token.kind,
                TokenKind::Period | TokenKind::Semicolon | TokenKind::Keyword(_)
            )
        }) || self.is_statement_start()
            || self.is_identifier_at("ELSE")
            || self.at_clause_boundary()
        {
            return true;
        }
        stop_words.iter().any(|word| self.is_identifier_at(word))
    }

    fn is_statement_start(&self) -> bool {
        match self.current_token().map(|token| &token.kind) {
            Some(TokenKind::Keyword(
                Keyword::Display
                | Keyword::Accept
                | Keyword::Perform
                | Keyword::If
                | Keyword::Stop
                | Keyword::Run,
            )) => true,
            Some(TokenKind::Identifier(name)) => matches!(
                name.as_str(),
                "ADD"
                    | "SUBTRACT"
                    | "MULTIPLY"
                    | "DIVIDE"
                    | "COMPUTE"
                    | "GO"
                    | "GOTO"
                    | "GOBACK"
                    | "OPEN"
                    | "CLOSE"
                    | "READ"
                    | "WRITE"
                    | "START"
                    | "EVALUATE"
                    | "SEARCH"
                    | "SET"
                    | "INSPECT"
                    | "INITIALIZE"
            ),
            _ => false,
        }
    }

    fn parse_on_size_error(&mut self) -> Result<Vec<Statement>, ParseError> {
        if !self.consume_identifier_matching("ON") {
            return Ok(Vec::new());
        }
        if !self.consume_identifier_matching("SIZE") || !self.consume_identifier_matching("ERROR") {
            return Err(ParseError::InvalidStatement);
        }

        let mut statements = Vec::new();
        while !self.at_statement_boundary() {
            let before = self.pos;
            if let Some(statement) = self.parse_statement_or_recover() {
                statements.push(statement);
            } else if before == self.pos {
                break;
            }
        }
        Ok(statements)
    }

    fn parse_display_statement(&mut self) -> Result<Statement, ParseError> {
        let span = self.expect_keyword(Keyword::Display)?.span;
        let mut items = Vec::new();
        let mut no_advancing = false;
        while let Some(token) = self.current_token() {
            if self.is_phrase_at(&["WITH", "NO", "ADVANCING"]) {
                self.pos += 3;
                no_advancing = true;
                continue;
            }
            if self.is_identifier_at("WITH") {
                self.report_unsupported(self.pos, "DISPLAY WITH clause");
                while self.current_token().is_some()
                    && !self.current_token().is_some_and(|token| {
                        matches!(token.kind, TokenKind::Period | TokenKind::Semicolon)
                    })
                    && !self.is_statement_start()
                {
                    self.advance();
                }
                break;
            }
            if self.is_statement_start()
                || self.is_identifier_at("ELSE")
                || self.at_clause_boundary()
            {
                break;
            }
            match &token.kind {
                TokenKind::StringLiteral(s) => {
                    items.push(DisplayItem::String(s.clone(), token.span));
                    self.advance();
                }
                TokenKind::Period => break,
                TokenKind::Keyword(_) => break,
                _ => {
                    items.push(DisplayItem::Expression(self.parse_expression()?));
                }
            }
        }
        Ok(Statement::Display(DisplayStatement {
            span,
            items,
            no_advancing,
        }))
    }

    fn parse_accept_statement(&mut self) -> Result<Statement, ParseError> {
        let span = self.expect_keyword(Keyword::Accept)?.span;
        let target = self.parse_expression()?;
        Ok(Statement::Accept(AcceptStatement { span, target }))
    }

    fn at_perform_option(&self) -> bool {
        self.is_identifier_at("UNTIL")
            || self.is_identifier_at("VARYING")
            || self.is_phrase_at(&["WITH", "TEST"])
            || self.is_times_option()
    }

    fn is_times_option(&self) -> bool {
        let times_next = self.tokens.get(self.pos + 1).is_some_and(
            |token| matches!(&token.kind, TokenKind::Identifier(name) if name == "TIMES"),
        );
        match self.current_token().map(|token| &token.kind) {
            Some(TokenKind::NumericLiteral(_)) => true,
            Some(TokenKind::Identifier(_)) => times_next,
            _ => false,
        }
    }

    fn parse_perform_varying(
        &mut self,
        span: Span,
        paragraph: Option<String>,
    ) -> Result<PerformVarying, ParseError> {
        if !self.consume_identifier_matching("VARYING") {
            return Err(ParseError::InvalidStatement);
        }
        let variable = self.parse_primary_expression()?;
        if !self.consume_identifier_matching("FROM") {
            return Err(ParseError::InvalidStatement);
        }
        let from = self.parse_expression()?;
        if !self.consume_identifier_matching("BY") {
            return Err(ParseError::InvalidStatement);
        }
        let by = self.parse_expression()?;
        if !self.consume_identifier_matching("UNTIL") {
            return Err(ParseError::InvalidStatement);
        }
        let condition = self.parse_condition()?;
        let body = if paragraph.is_none() {
            self.parse_perform_body()
        } else {
            Vec::new()
        };
        Ok(PerformVarying {
            span,
            paragraph,
            variable,
            from,
            by,
            condition,
            body,
        })
    }

    /// Statements of an inline PERFORM, up to and including `END-PERFORM`.
    fn parse_perform_body(&mut self) -> Vec<Statement> {
        let body = self.parse_statement_block();
        self.consume_identifier_matching("END-PERFORM");
        body
    }

    fn parse_perform_statement(&mut self) -> Result<Statement, ParseError> {
        let span = self.expect_keyword(Keyword::Perform)?.span;
        self.consume_perform_test();
        if self.at_perform_option() || self.is_statement_start() || self.at_clause_boundary() {
            return self.parse_inline_perform(span);
        }
        let (name, _) = self.expect_identifier()?;
        if self.consume_identifier_matching("THRU") || self.consume_identifier_matching("THROUGH") {
            let (last, _) = self.expect_identifier()?;
            return Ok(Statement::Perform(PerformStatement::Thru {
                first: name,
                last,
                span,
            }));
        }
        self.consume_perform_test();
        if self.is_identifier_at("VARYING") {
            let varying = self.parse_perform_varying(span, Some(name))?;
            return Ok(Statement::Perform(PerformStatement::Varying(varying)));
        }
        if self.is_times_option() {
            let times = self.parse_primary_expression()?;
            self.consume_identifier_matching("TIMES");
            return Ok(Statement::Perform(PerformStatement::Times {
                times: Box::new(times),
                paragraph: name,
                span,
            }));
        }
        if self.consume_identifier_matching("UNTIL") {
            let condition = self.parse_condition()?;
            Ok(Statement::Perform(PerformStatement::Until {
                condition: Box::new(condition),
                paragraph: name,
                span,
            }))
        } else {
            Ok(Statement::Perform(PerformStatement::ProcedureName(
                name, span,
            )))
        }
    }

    fn consume_perform_test(&mut self) {
        if self.is_phrase_at(&["WITH", "TEST"]) {
            self.pos += 3;
            self.consume_identifier_matching("BEFORE");
            self.consume_identifier_matching("AFTER");
        }
    }

    fn parse_inline_perform(&mut self, span: Span) -> Result<Statement, ParseError> {
        if self.is_identifier_at("VARYING") {
            let varying = self.parse_perform_varying(span, None)?;
            return Ok(Statement::Perform(PerformStatement::Varying(varying)));
        }
        let mut times = None;
        let mut condition = None;
        if self.is_times_option() {
            times = Some(self.parse_primary_expression()?);
            self.consume_identifier_matching("TIMES");
        } else if self.consume_identifier_matching("UNTIL") {
            condition = Some(self.parse_condition()?);
        }
        let body = self.parse_perform_body();
        Ok(Statement::Perform(PerformStatement::Inline(
            PerformInline {
                span,
                times,
                condition,
                body,
            },
        )))
    }

    fn parse_if_statement(&mut self) -> Result<Statement, ParseError> {
        let span = self.expect_keyword(Keyword::If)?.span;
        let condition = self.parse_condition()?;
        let mut then_statements = Vec::new();
        while !self.at_statement_boundary() {
            let before = self.pos;
            if let Some(statement) = self.parse_statement_or_recover() {
                then_statements.push(statement);
            } else if before == self.pos {
                break;
            }
        }
        let else_statements = if self.is_keyword_at(Keyword::Else) {
            self.advance();
            let mut stmts = Vec::new();
            while !self.at_statement_boundary() {
                let before = self.pos;
                if let Some(statement) = self.parse_statement_or_recover() {
                    stmts.push(statement);
                } else if before == self.pos {
                    break;
                }
            }
            Some(stmts)
        } else {
            None
        };
        self.consume_identifier_matching("END-IF");
        Ok(Statement::If(IfStatement {
            span,
            condition,
            then_statements,
            else_statements,
        }))
    }

    fn parse_set_statement(&mut self) -> Result<Statement, ParseError> {
        let span = self.consume_verb("SET")?;
        let targets = self.parse_expression_list_until(&["TO", "UP", "DOWN"])?;
        let kind = if self.consume_identifier_matching("TO") {
            SetKind::To
        } else if self.consume_identifier_matching("UP") {
            self.consume_identifier_matching("BY");
            SetKind::UpBy
        } else if self.consume_identifier_matching("DOWN") {
            self.consume_identifier_matching("BY");
            SetKind::DownBy
        } else {
            return Err(ParseError::InvalidStatement);
        };
        let value = self.parse_expression()?;
        Ok(Statement::Set(SetStatement {
            span,
            targets,
            kind,
            value,
        }))
    }

    fn parse_search_statement(&mut self) -> Result<Statement, ParseError> {
        let span = self.consume_verb("SEARCH")?;
        let all = self.consume_identifier_matching("ALL");
        let table = self.parse_primary_expression()?;
        let varying = if self.consume_identifier_matching("VARYING") {
            Some(self.parse_expression()?)
        } else {
            None
        };
        let mut at_end = None;
        let mut whens = Vec::new();
        loop {
            if self.is_phrase_at(&["AT", "END"]) {
                self.pos += 2;
                at_end = Some(self.parse_statement_block());
            } else if self.is_identifier_at("WHEN") {
                let when_span = self.tokens[self.pos].span;
                self.advance();
                let condition = self.parse_condition()?;
                let statements = self.parse_statement_block();
                whens.push(SearchWhen {
                    span: when_span,
                    condition,
                    statements,
                });
            } else {
                break;
            }
        }
        if whens.is_empty() {
            return Err(ParseError::InvalidStatement);
        }
        self.consume_identifier_matching("END-SEARCH");
        Ok(Statement::Search(SearchStatement {
            span,
            all,
            table,
            varying,
            at_end,
            whens,
        }))
    }

    fn parse_evaluate_statement(&mut self) -> Result<Statement, ParseError> {
        let span = self.consume_verb("EVALUATE")?;
        let subject = self.parse_expression()?;
        let condition_subject = matches!(
            &subject,
            Expression::FigurativeConstant(word, _) if word == "TRUE" || word == "FALSE"
        );
        let mut whens = Vec::new();
        while self.is_identifier_at("WHEN") {
            let when_span = self.tokens[self.pos].span;
            let mut choices = Vec::new();
            let mut other = false;
            // Stacked `WHEN` clauses share one statement block.
            while self.is_identifier_at("WHEN") {
                self.advance();
                if self.consume_identifier_matching("OTHER") {
                    other = true;
                } else {
                    choices.push(self.parse_evaluate_choice(condition_subject)?);
                }
            }
            let statements = self.parse_statement_block();
            whens.push(EvaluateWhen {
                span: when_span,
                choices,
                other,
                statements,
            });
        }
        if whens.is_empty() {
            return Err(ParseError::InvalidStatement);
        }
        self.consume_identifier_matching("END-EVALUATE");
        Ok(Statement::Evaluate(EvaluateStatement {
            span,
            subject,
            whens,
        }))
    }

    fn parse_evaluate_choice(
        &mut self,
        condition_subject: bool,
    ) -> Result<EvaluateChoice, ParseError> {
        if self.consume_identifier_matching("ANY") {
            return Ok(EvaluateChoice::Any);
        }
        if condition_subject {
            return Ok(EvaluateChoice::Condition(self.parse_condition()?));
        }
        let first = self.parse_expression()?;
        if self.consume_identifier_matching("THRU") || self.consume_identifier_matching("THROUGH") {
            return Ok(EvaluateChoice::Range(first, self.parse_expression()?));
        }
        Ok(EvaluateChoice::Value(first))
    }

    fn consume_inspect_mode(&mut self, allow_first: bool) -> Option<InspectMode> {
        let mode = match self.current_token().map(|token| &token.kind) {
            Some(TokenKind::Identifier(name)) => match name.as_str() {
                "CHARACTERS" => InspectMode::Characters,
                "ALL" => InspectMode::All,
                "LEADING" => InspectMode::Leading,
                "FIRST" if allow_first => InspectMode::First,
                _ => return None,
            },
            _ => return None,
        };
        self.advance();
        Some(mode)
    }

    fn parse_inspect_region(&mut self) -> Result<InspectRegion, ParseError> {
        let mut region = InspectRegion::default();
        loop {
            let is_before = self.is_identifier_at("BEFORE");
            if !is_before && !self.is_identifier_at("AFTER") {
                return Ok(region);
            }
            self.advance();
            self.consume_identifier_matching("INITIAL");
            let limit = self.parse_primary_expression()?;
            if is_before {
                region.before = Some(limit);
            } else {
                region.after = Some(limit);
            }
        }
    }

    fn parse_inspect_statement(&mut self) -> Result<Statement, ParseError> {
        let span = self.consume_verb("INSPECT")?;
        let target = self.parse_primary_expression()?;
        let mut tallying = Vec::new();
        let mut replacing = Vec::new();
        let mut converting = None;
        loop {
            if self.consume_identifier_matching("TALLYING") {
                loop {
                    let counter = self.parse_primary_expression()?;
                    if !self.consume_identifier_matching("FOR") {
                        return Err(ParseError::InvalidStatement);
                    }
                    while let Some(mode) = self.consume_inspect_mode(false) {
                        let pattern = if mode == InspectMode::Characters {
                            None
                        } else {
                            Some(self.parse_primary_expression()?)
                        };
                        let region = self.parse_inspect_region()?;
                        tallying.push(InspectTally {
                            counter: counter.clone(),
                            mode,
                            pattern,
                            region,
                        });
                    }
                    if self.at_expression_list_end(&["REPLACING", "CONVERTING"]) {
                        break;
                    }
                }
            } else if self.consume_identifier_matching("REPLACING") {
                while let Some(mode) = self.consume_inspect_mode(true) {
                    let pattern = if mode == InspectMode::Characters {
                        None
                    } else {
                        Some(self.parse_primary_expression()?)
                    };
                    if !self.consume_identifier_matching("BY") {
                        return Err(ParseError::InvalidStatement);
                    }
                    let by = self.parse_primary_expression()?;
                    let region = self.parse_inspect_region()?;
                    replacing.push(InspectReplace {
                        mode,
                        pattern,
                        by,
                        region,
                    });
                }
            } else if self.consume_identifier_matching("CONVERTING") {
                let from = self.parse_primary_expression()?;
                if !self.consume_identifier_matching("TO") {
                    return Err(ParseError::InvalidStatement);
                }
                let to = self.parse_primary_expression()?;
                let region = self.parse_inspect_region()?;
                converting = Some(InspectConvert { from, to, region });
            } else {
                break;
            }
        }
        if tallying.is_empty() && replacing.is_empty() && converting.is_none() {
            return Err(ParseError::InvalidStatement);
        }
        Ok(Statement::Inspect(InspectStatement {
            span,
            target,
            tallying,
            replacing,
            converting,
        }))
    }

    fn consume_open_mode(&mut self) -> Option<String> {
        let mode = match self.current_token().map(|token| &token.kind) {
            Some(TokenKind::Identifier(name))
                if matches!(name.as_str(), "INPUT" | "OUTPUT" | "I-O" | "EXTEND") =>
            {
                name.clone()
            }
            _ => return None,
        };
        self.advance();
        Some(mode)
    }

    fn parse_file_names(&mut self) -> Vec<(String, Span)> {
        let mut files = Vec::new();
        while self.is_identifier_at_next()
            && !self.is_statement_start()
            && !self.at_clause_boundary()
            && !self.is_identifier_at("ELSE")
            && !matches!(
                self.current_token().map(|token| &token.kind),
                Some(TokenKind::Identifier(name))
                    if matches!(name.as_str(), "INPUT" | "OUTPUT" | "I-O" | "EXTEND")
            )
        {
            files.extend(self.consume_identifier());
        }
        files
    }

    fn parse_open_statement(&mut self) -> Result<Statement, ParseError> {
        let span = self.consume_verb("OPEN")?;
        let mut groups = Vec::new();
        while let Some(mode) = self.consume_open_mode() {
            let files = self.parse_file_names();
            if files.is_empty() {
                return Err(ParseError::InvalidStatement);
            }
            groups.push(OpenGroup { mode, files });
        }
        if groups.is_empty() {
            return Err(ParseError::InvalidStatement);
        }
        Ok(Statement::Open(OpenStatement { span, groups }))
    }

    fn parse_close_statement(&mut self) -> Result<Statement, ParseError> {
        let span = self.consume_verb("CLOSE")?;
        let files = self.parse_file_names();
        if files.is_empty() {
            return Err(ParseError::InvalidStatement);
        }
        Ok(Statement::Close(CloseStatement { span, files }))
    }

    fn parse_io_handlers(&mut self, end_word: &str) -> IoHandlers {
        let mut handlers = IoHandlers::default();
        loop {
            if self.is_phrase_at(&["AT", "END"]) {
                self.pos += 2;
                handlers.at_end = Some(self.parse_statement_block());
            } else if self.is_phrase_at(&["NOT", "AT", "END"]) {
                self.pos += 3;
                handlers.not_at_end = Some(self.parse_statement_block());
            } else if self.is_phrase_at(&["INVALID", "KEY"]) {
                self.pos += 2;
                handlers.invalid_key = Some(self.parse_statement_block());
            } else if self.is_phrase_at(&["NOT", "INVALID", "KEY"]) {
                self.pos += 3;
                handlers.not_invalid_key = Some(self.parse_statement_block());
            } else {
                break;
            }
        }
        self.consume_identifier_matching(end_word);
        handlers
    }

    fn parse_read_statement(&mut self) -> Result<Statement, ParseError> {
        let span = self.consume_verb("READ")?;
        let file = self.expect_identifier()?;
        self.consume_identifier_matching("NEXT");
        self.consume_identifier_matching("RECORD");
        let into = if self.consume_identifier_matching("INTO") {
            Some(self.parse_primary_expression()?)
        } else {
            None
        };
        let key = if self.consume_identifier_matching("KEY") {
            self.consume_identifier_matching("IS");
            Some(self.parse_primary_expression()?)
        } else {
            None
        };
        let handlers = self.parse_io_handlers("END-READ");
        Ok(Statement::Read(ReadStatement {
            span,
            file,
            into,
            key,
            handlers,
        }))
    }

    fn parse_write_statement(&mut self) -> Result<Statement, ParseError> {
        let span = self.consume_verb("WRITE")?;
        let record = self.parse_primary_expression()?;
        let from = if self.consume_identifier_matching("FROM") {
            Some(self.parse_primary_expression()?)
        } else {
            None
        };
        let advancing = self.parse_write_advancing()?;
        let handlers = self.parse_io_handlers("END-WRITE");
        Ok(Statement::Write(WriteStatement {
            span,
            record,
            from,
            advancing,
            handlers,
        }))
    }

    fn parse_write_advancing(&mut self) -> Result<Option<WriteAdvancing>, ParseError> {
        let before = if self.is_identifier_at("BEFORE") {
            true
        } else if self.is_identifier_at("AFTER") {
            false
        } else {
            return Ok(None);
        };
        self.advance();
        if !self.consume_identifier_matching("ADVANCING") {
            return Err(ParseError::InvalidStatement);
        }
        if self.consume_identifier_matching("PAGE") {
            return Ok(Some(WriteAdvancing {
                before,
                lines: None,
            }));
        }
        let lines = self.parse_primary_expression()?;
        self.consume_identifier_matching("LINE");
        self.consume_identifier_matching("LINES");
        Ok(Some(WriteAdvancing {
            before,
            lines: Some(lines),
        }))
    }

    fn parse_start_statement(&mut self) -> Result<Statement, ParseError> {
        let span = self.consume_verb("START")?;
        let file = self.expect_identifier()?;
        let key = if self.consume_identifier_matching("KEY") {
            self.consume_identifier_matching("IS");
            let op = self.parse_start_relation();
            Some((op, self.parse_primary_expression()?))
        } else {
            None
        };
        let handlers = self.parse_io_handlers("END-START");
        Ok(Statement::Start(StartStatement {
            span,
            file,
            key,
            handlers,
        }))
    }

    /// Parses the relation of `START ... KEY`; plain `KEY name` means equality.
    fn parse_start_relation(&mut self) -> BinaryOperator {
        if let Some((op, _)) = self.consume_relational_operator() {
            return op;
        }
        let negated = self.consume_identifier_matching("NOT");
        let base = if self.consume_identifier_matching("EQUAL") {
            BinaryOperator::Equal
        } else if self.consume_identifier_matching("GREATER") {
            BinaryOperator::GreaterThan
        } else if self.consume_identifier_matching("LESS") {
            BinaryOperator::LessThan
        } else {
            return BinaryOperator::Equal;
        };
        self.consume_identifier_matching("THAN");
        let or_equal =
            self.consume_identifier_matching("OR") && self.consume_identifier_matching("EQUAL");
        self.consume_identifier_matching("TO");
        match (base, negated, or_equal) {
            (BinaryOperator::Equal, true, _) => BinaryOperator::NotEqual,
            (BinaryOperator::GreaterThan, true, _) => BinaryOperator::LessThanOrEqual,
            (BinaryOperator::LessThan, true, _) => BinaryOperator::GreaterThanOrEqual,
            (BinaryOperator::GreaterThan, false, true) => BinaryOperator::GreaterThanOrEqual,
            (BinaryOperator::LessThan, false, true) => BinaryOperator::LessThanOrEqual,
            (base, _, _) => base,
        }
    }

    fn parse_stop_run_statement(&mut self) -> Result<Statement, ParseError> {
        let span = self.expect_keyword(Keyword::Stop)?.span;
        self.expect_keyword(Keyword::Run)?;
        Ok(Statement::StopRun(span))
    }

    fn parse_goto_statement(&mut self) -> Result<Statement, ParseError> {
        let span = self.current_token().ok_or(ParseError::UnexpectedEof)?.span;
        self.advance();
        let (target, _) = self.expect_identifier()?;
        Ok(Statement::GoTo(GoToStatement { span, target }))
    }

    fn parse_expression(&mut self) -> Result<Expression, ParseError> {
        self.parse_additive_expression()
    }

    fn parse_additive_expression(&mut self) -> Result<Expression, ParseError> {
        let mut expr = self.parse_multiplicative_expression()?;
        while let Some((op, _op_span)) = self.consume_additive_operator() {
            let right = self.parse_multiplicative_expression()?;
            let span = Span::new(expr.span().file, expr.span().start, right.span().end);
            expr = Expression::BinaryOp {
                left: Box::new(expr),
                op,
                right: Box::new(right),
                span,
            };
        }
        Ok(expr)
    }

    fn parse_multiplicative_expression(&mut self) -> Result<Expression, ParseError> {
        let mut expr = self.parse_unary_expression()?;
        while let Some((op, _op_span)) = self.consume_multiplicative_operator() {
            let right = self.parse_unary_expression()?;
            let span = Span::new(expr.span().file, expr.span().start, right.span().end);
            expr = Expression::BinaryOp {
                left: Box::new(expr),
                op,
                right: Box::new(right),
                span,
            };
        }
        Ok(expr)
    }

    fn parse_unary_expression(&mut self) -> Result<Expression, ParseError> {
        if let Some(op) = self.consume_unary_operator() {
            let operand = self.parse_unary_expression()?;
            let span = Span::new(op.span.file, op.span.start, operand.span().end);
            let unary_op = match op.kind {
                TokenKind::Plus => UnaryOperator::Plus,
                TokenKind::Minus => UnaryOperator::Minus,
                _ => UnaryOperator::Plus,
            };
            Ok(Expression::UnaryOp {
                op: unary_op,
                operand: Box::new(operand),
                span,
            })
        } else {
            self.parse_primary_expression()
        }
    }

    fn parse_primary_expression(&mut self) -> Result<Expression, ParseError> {
        if let Some(token) = self.current_token() {
            match &token.kind {
                TokenKind::Identifier(name) => {
                    if name == "FUNCTION" {
                        return self.parse_function_call(token.span);
                    }
                    let mut identifier = name.clone();
                    let span = token.span;
                    self.advance();
                    if is_figurative_constant(&identifier) {
                        return Ok(Expression::FigurativeConstant(identifier, span));
                    }
                    let mut end = span.end;
                    while self.is_identifier_at("OF") || self.is_identifier_at("IN") {
                        let qualifier = if self.consume_identifier_matching("OF") {
                            "OF"
                        } else {
                            self.consume_identifier_matching("IN");
                            "IN"
                        };
                        let (parent, parent_span) = self.expect_identifier()?;
                        identifier.push_str(&format!(" {qualifier} {parent}"));
                        if parent_span.file == span.file {
                            end = parent_span.end;
                        }
                    }
                    if self
                        .current_token()
                        .is_some_and(|token| matches!(token.kind, TokenKind::LParen))
                    {
                        return self.parse_subscript(identifier, span);
                    }
                    Ok(Expression::Identifier(
                        identifier,
                        Span::new(span.file, span.start, end),
                    ))
                }
                TokenKind::NumericLiteral(n) => {
                    let result = Expression::NumericLiteral(n.clone(), token.span);
                    self.advance();
                    Ok(result)
                }
                TokenKind::StringLiteral(s) => {
                    let result = Expression::StringLiteral(s.clone(), token.span);
                    self.advance();
                    Ok(result)
                }
                TokenKind::LParen => {
                    self.advance();
                    let expr = self.parse_expression()?;
                    if let Some(token) = self.current_token()
                        && matches!(token.kind, TokenKind::RParen)
                    {
                        self.advance();
                    }
                    Ok(expr)
                }
                _ => Err(ParseError::unexpected_token("expression", &token.kind)),
            }
        } else {
            Err(ParseError::UnexpectedEof)
        }
    }

    fn parse_subscript(&mut self, name: String, start: Span) -> Result<Expression, ParseError> {
        self.advance();
        let mut indices = Vec::new();
        loop {
            if self
                .current_token()
                .is_some_and(|token| matches!(token.kind, TokenKind::Comma))
            {
                self.advance();
                continue;
            }
            if let Some(token) = self.current_token()
                && matches!(token.kind, TokenKind::RParen)
            {
                let end = token.span.end;
                self.advance();
                if indices.is_empty() {
                    return Err(ParseError::InvalidStatement);
                }
                return Ok(Expression::Subscript {
                    name,
                    indices,
                    span: Span::new(start.file, start.start, end),
                });
            }
            indices.push(self.parse_expression()?);
            if indices.len() == 1
                && self
                    .current_token()
                    .is_some_and(|token| matches!(token.kind, TokenKind::Colon))
            {
                return self.parse_ref_mod(name, start, indices.remove(0));
            }
        }
    }

    /// Parses the rest of `name(start:[length])`, after the start expression.
    fn parse_ref_mod(
        &mut self,
        name: String,
        span_start: Span,
        start: Expression,
    ) -> Result<Expression, ParseError> {
        self.advance();
        let length = if self
            .current_token()
            .is_some_and(|token| matches!(token.kind, TokenKind::RParen))
        {
            None
        } else {
            Some(Box::new(self.parse_expression()?))
        };
        let Some(token) = self.current_token() else {
            return Err(ParseError::UnexpectedEof);
        };
        if !matches!(token.kind, TokenKind::RParen) {
            return Err(ParseError::unexpected_token("')'", &token.kind));
        }
        let end = token.span.end;
        self.advance();
        Ok(Expression::RefMod {
            name,
            start: Box::new(start),
            length,
            span: Span::new(span_start.file, span_start.start, end),
        })
    }

    fn parse_initialize_statement(&mut self) -> Result<Statement, ParseError> {
        let span = self.consume_verb("INITIALIZE")?;
        let targets = self.parse_expression_list_until(&[])?;
        if targets.is_empty() {
            return Err(ParseError::InvalidStatement);
        }
        Ok(Statement::Initialize(InitializeStatement { span, targets }))
    }

    fn parse_condition(&mut self) -> Result<Condition, ParseError> {
        self.parse_or_condition()
    }

    fn parse_or_condition(&mut self) -> Result<Condition, ParseError> {
        let mut cond = self.parse_and_condition()?;
        while self.consume_identifier_matching("OR") {
            let right = if let Some((left, op)) = last_relation(&cond) {
                if self.is_expression_start() {
                    let start = self.pos;
                    let expression = self.parse_expression()?;
                    if self.consume_relational_operator().is_none() {
                        let span =
                            Span::new(left.span().file, left.span().start, expression.span().end);
                        Condition::Relation {
                            left: left.clone(),
                            op,
                            right: expression,
                            span,
                        }
                    } else {
                        self.pos = start;
                        self.parse_and_condition()?
                    }
                } else {
                    self.parse_and_condition()?
                }
            } else {
                self.parse_and_condition()?
            };
            let span = Span::new(cond.span().file, cond.span().start, right.span().end);
            cond = Condition::Or {
                left: Box::new(cond),
                right: Box::new(right),
                span,
            };
        }
        Ok(cond)
    }

    fn parse_and_condition(&mut self) -> Result<Condition, ParseError> {
        let mut cond = self.parse_relation_condition()?;
        while self.consume_identifier_matching("AND") {
            let right = self.parse_relation_condition()?;
            let span = Span::new(cond.span().file, cond.span().start, right.span().end);
            cond = Condition::And {
                left: Box::new(cond),
                right: Box::new(right),
                span,
            };
        }
        Ok(cond)
    }

    fn parse_relation_condition(&mut self) -> Result<Condition, ParseError> {
        if self.consume_identifier_matching("NOT") {
            let cond = self.parse_relation_condition()?;
            let span = cond.span();
            return Ok(Condition::Not {
                condition: Box::new(cond),
                span,
            });
        }
        let left = self.parse_expression()?;
        if let Some((op, _op_span)) = self.consume_relational_operator() {
            let right = self.parse_expression()?;
            let span = Span::new(left.span().file, left.span().start, right.span().end);
            Ok(Condition::Relation {
                left,
                op,
                right,
                span,
            })
        } else {
            Err(ParseError::InvalidStatement)
        }
    }

    fn consume_additive_operator(&mut self) -> Option<(BinaryOperator, Span)> {
        if let Some(token) = self.current_token() {
            let op = match token.kind {
                TokenKind::Plus => Some((BinaryOperator::Add, token.span)),
                TokenKind::Minus => Some((BinaryOperator::Subtract, token.span)),
                _ => None,
            };
            if op.is_some() {
                self.advance();
            }
            op
        } else {
            None
        }
    }

    fn consume_multiplicative_operator(&mut self) -> Option<(BinaryOperator, Span)> {
        if let Some(token) = self.current_token() {
            let op = match token.kind {
                TokenKind::Star => Some((BinaryOperator::Multiply, token.span)),
                TokenKind::Slash => Some((BinaryOperator::Divide, token.span)),
                TokenKind::Power => Some((BinaryOperator::Power, token.span)),
                _ => None,
            };
            if op.is_some() {
                self.advance();
            }
            op
        } else {
            None
        }
    }

    fn consume_relational_operator(&mut self) -> Option<(BinaryOperator, Span)> {
        if let Some(token) = self.current_token() {
            let op = match token.kind {
                TokenKind::Eq => Some((BinaryOperator::Equal, token.span)),
                TokenKind::Lt => Some((BinaryOperator::LessThan, token.span)),
                TokenKind::Gt => Some((BinaryOperator::GreaterThan, token.span)),
                TokenKind::LtEq => Some((BinaryOperator::LessThanOrEqual, token.span)),
                TokenKind::GtEq => Some((BinaryOperator::GreaterThanOrEqual, token.span)),
                _ => None,
            };
            if op.is_some() {
                self.advance();
            }
            op
        } else {
            None
        }
    }

    fn consume_unary_operator(&mut self) -> Option<Token> {
        if let Some(token) = self.current_token()
            && matches!(token.kind, TokenKind::Plus | TokenKind::Minus)
        {
            let result = token.clone();
            self.advance();
            return Some(result);
        }
        None
    }

    fn consume_identifier_matching(&mut self, name: &str) -> bool {
        if self.is_identifier_at(name) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn is_keyword_at(&self, keyword: Keyword) -> bool {
        self.current_token()
            .map(|t| matches!(&t.kind, TokenKind::Keyword(k) if *k == keyword))
            .unwrap_or(false)
    }

    fn is_identifier_at(&self, name: &str) -> bool {
        self.current_token()
            .map(|t| matches!(&t.kind, TokenKind::Identifier(id) if id == name))
            .unwrap_or(false)
    }

    fn is_identifier_at_next(&self) -> bool {
        self.current_token()
            .map(|t| matches!(t.kind, TokenKind::Identifier(_)))
            .unwrap_or(false)
    }

    fn is_paragraph_start(&self) -> bool {
        self.is_identifier_at_next()
            && self
                .tokens
                .get(self.pos + 1)
                .is_some_and(|token| matches!(token.kind, TokenKind::Period))
    }

    fn is_level_number_at(&self) -> bool {
        self.current_token()
            .map(|t| matches!(t.kind, TokenKind::NumericLiteral(_)))
            .unwrap_or(false)
    }
}

fn is_figurative_constant(word: &str) -> bool {
    matches!(
        word,
        "ZERO"
            | "TRUE"
            | "FALSE"
            | "ZEROS"
            | "ZEROES"
            | "SPACE"
            | "SPACES"
            | "HIGH-VALUE"
            | "HIGH-VALUES"
            | "LOW-VALUE"
            | "LOW-VALUES"
            | "QUOTE"
            | "QUOTES"
            | "NULL"
            | "NULLS"
    )
}

impl Expression {
    fn span(&self) -> Span {
        match self {
            Expression::Identifier(_, span) => *span,
            Expression::NumericLiteral(_, span) => *span,
            Expression::StringLiteral(_, span) => *span,
            Expression::FigurativeConstant(_, span) => *span,
            Expression::Subscript { span, .. } | Expression::RefMod { span, .. } => *span,
            Expression::FunctionCall { span, .. } => *span,
            Expression::BinaryOp { span, .. } => *span,
            Expression::UnaryOp { span, .. } => *span,
        }
    }
}

fn last_relation(condition: &Condition) -> Option<(&Expression, BinaryOperator)> {
    match condition {
        Condition::Relation { left, op, .. } => Some((left, *op)),
        Condition::And { right, .. } | Condition::Or { right, .. } => last_relation(right),
        Condition::Not { condition, .. } => last_relation(condition),
    }
}

impl Condition {
    fn span(&self) -> Span {
        match self {
            Condition::Relation { span, .. } => *span,
            Condition::And { span, .. } => *span,
            Condition::Or { span, .. } => *span,
            Condition::Not { span, .. } => *span,
        }
    }
}
