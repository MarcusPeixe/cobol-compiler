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
                self.diagnostics.push(Diagnostic::error(
                    "cobol::parser::parse_error",
                    e.to_string(),
                ));
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
        let environment_division = self.try_parse_environment_division();
        let data_division = self.try_parse_data_division();
        let procedure_division = self.try_parse_procedure_division();

        Ok(Program {
            identification_division,
            environment_division,
            data_division,
            procedure_division,
        })
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

        let configuration_section = self.try_parse_configuration_section();
        let input_output_section = self.try_parse_input_output_section();

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

        Some(InputOutputSection { span })
    }

    fn try_parse_data_division(&mut self) -> Option<DataDivision> {
        if !self.is_keyword_at(Keyword::Data) {
            return None;
        }

        let div_start = self.current_token()?.span;
        self.advance();
        self.expect_keyword(Keyword::Division).ok()?;
        self.expect_period().ok()?;

        let file_section = self.try_parse_file_section();
        let working_storage_section = self.try_parse_working_storage_section();

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
            if let Ok((file_name, _)) = self.expect_identifier() {
                self.expect_period().ok()?;
                let file_desc = FileDescription {
                    span,
                    file_name,
                    clauses: Vec::new(),
                };
                file_descriptions.push(file_desc);
            }
        }

        Some(FileSection {
            span,
            file_descriptions,
        })
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
        if level <= parent_level {
            return None;
        }

        // Now actually consume the level number
        self.parse_level_number()?;

        let span = self.current_token()?.span;
        let (name, _) = self.consume_identifier()?;

        let mut picture = None;
        let mut value = None;

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
                TokenKind::Identifier(ref id) if id == "VALUE" => {
                    self.advance();
                    if let Some(lit) = self.parse_literal() {
                        value = Some(lit);
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
        if !self.is_identifier_at_next() {
            return None;
        }

        let (name, span) = self.consume_identifier()?;
        self.expect_period().ok()?;

        let mut statements = Vec::new();
        while let Some(stmt) = self.try_parse_statement() {
            statements.push(stmt);
            if self
                .current_token()
                .map(|t| matches!(t.kind, TokenKind::Period))
                .unwrap_or(false)
            {
                self.advance();
            }
            if self.is_identifier_at_next() || self.is_keyword_at(Keyword::Procedure) {
                break;
            }
        }

        Some(Paragraph {
            span,
            name,
            statements,
        })
    }

    fn try_parse_statement(&mut self) -> Option<Statement> {
        if let Some(token) = self.current_token() {
            match &token.kind {
                TokenKind::Keyword(Keyword::Move) => {
                    return self.parse_move_statement().ok();
                }
                TokenKind::Keyword(Keyword::Display) => {
                    return self.parse_display_statement().ok();
                }
                TokenKind::Keyword(Keyword::Accept) => {
                    return self.parse_accept_statement().ok();
                }
                TokenKind::Keyword(Keyword::Perform) => {
                    return self.parse_perform_statement().ok();
                }
                TokenKind::Keyword(Keyword::If) => {
                    return self.parse_if_statement().ok();
                }
                TokenKind::Keyword(Keyword::Stop) => {
                    return self.parse_stop_run_statement().ok();
                }
                TokenKind::Keyword(Keyword::Run) => {
                    let span = token.span;
                    self.advance();
                    return Some(Statement::StopRun(span));
                }
                TokenKind::Identifier(id) if id == "GO" || id == "GOBACK" || id == "GOTO" => {
                    return self.parse_goto_statement().ok();
                }
                _ => {}
            }
        }
        None
    }

    fn parse_move_statement(&mut self) -> Result<Statement, ParseError> {
        let span = self.expect_keyword(Keyword::Move)?.span;
        let source = self.parse_expression()?;
        self.consume_identifier_matching("TO");
        let mut targets = Vec::new();
        targets.push(self.parse_expression()?);
        while self.consume_identifier_matching("TO") {
            targets.push(self.parse_expression()?);
        }
        Ok(Statement::Move(MoveStatement {
            span,
            source,
            targets,
        }))
    }

    fn parse_display_statement(&mut self) -> Result<Statement, ParseError> {
        let span = self.expect_keyword(Keyword::Display)?.span;
        let mut items = Vec::new();
        while let Some(token) = self.current_token() {
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
        Ok(Statement::Display(DisplayStatement { span, items }))
    }

    fn parse_accept_statement(&mut self) -> Result<Statement, ParseError> {
        let span = self.expect_keyword(Keyword::Accept)?.span;
        let target = self.parse_expression()?;
        Ok(Statement::Accept(AcceptStatement { span, target }))
    }

    fn parse_perform_statement(&mut self) -> Result<Statement, ParseError> {
        let span = self.expect_keyword(Keyword::Perform)?.span;
        let (name, _) = self.expect_identifier()?;
        Ok(Statement::Perform(PerformStatement::ProcedureName(
            name, span,
        )))
    }

    fn parse_if_statement(&mut self) -> Result<Statement, ParseError> {
        let span = self.expect_keyword(Keyword::If)?.span;
        let condition = self.parse_condition()?;
        let mut then_statements = Vec::new();
        while let Some(stmt) = self.try_parse_statement() {
            then_statements.push(stmt);
        }
        let else_statements = if self.consume_identifier_matching("ELSE") {
            let mut stmts = Vec::new();
            while let Some(stmt) = self.try_parse_statement() {
                stmts.push(stmt);
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
                    let result = Expression::Identifier(name.clone(), token.span);
                    self.advance();
                    Ok(result)
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

    fn parse_condition(&mut self) -> Result<Condition, ParseError> {
        self.parse_or_condition()
    }

    fn parse_or_condition(&mut self) -> Result<Condition, ParseError> {
        let mut cond = self.parse_and_condition()?;
        while self.consume_identifier_matching("OR") {
            let right = self.parse_and_condition()?;
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

    fn is_level_number_at(&self) -> bool {
        self.current_token()
            .map(|t| matches!(t.kind, TokenKind::NumericLiteral(_)))
            .unwrap_or(false)
    }
}

impl Expression {
    fn span(&self) -> Span {
        match self {
            Expression::Identifier(_, span) => *span,
            Expression::NumericLiteral(_, span) => *span,
            Expression::StringLiteral(_, span) => *span,
            Expression::BinaryOp { span, .. } => *span,
            Expression::UnaryOp { span, .. } => *span,
        }
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
