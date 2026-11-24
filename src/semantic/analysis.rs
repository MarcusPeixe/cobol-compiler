use std::collections::HashSet;

use crate::diagnostics::Diagnostic;
use crate::parser::{
    BinaryOperator, Condition, DataItem, Expression, FileDescription, PerformStatement, Program,
    Statement,
};
use crate::source::Span;

use super::errors;
use super::symbol_table::{Scope, Symbol, SymbolKind, SymbolTable};
use super::type_system::{
    DataType, can_operate_arithmetic, is_move_compatible, is_valid_picture, parse_picture,
};

#[derive(Debug, Clone)]
pub struct AnalyzedProgram {
    pub program: Program,
    pub symbol_table: SymbolTable,
}

pub struct SemanticAnalyzer<'a> {
    program: &'a Program,
    symbol_table: SymbolTable,
    paragraphs: HashSet<String>,
    files: HashSet<String>,
    diagnostics: Vec<Diagnostic>,
}

pub fn analyze(program: &Program) -> (AnalyzedProgram, Vec<Diagnostic>) {
    let mut analyzer = SemanticAnalyzer::new(program);
    analyzer.collect_symbols();
    analyzer.validate_program();
    (
        AnalyzedProgram {
            program: program.clone(),
            symbol_table: analyzer.symbol_table,
        },
        analyzer.diagnostics,
    )
}

impl<'a> SemanticAnalyzer<'a> {
    pub fn new(program: &'a Program) -> Self {
        Self {
            program,
            symbol_table: SymbolTable::default(),
            paragraphs: HashSet::new(),
            files: HashSet::new(),
            diagnostics: Vec::new(),
        }
    }

    fn collect_symbols(&mut self) {
        if let Some(data_division) = &self.program.data_division {
            if let Some(file_section) = &data_division.file_section {
                for file in &file_section.file_descriptions {
                    self.declare_file(file);
                }
            }
            if let Some(working_storage) = &data_division.working_storage_section {
                for item in &working_storage.data_items {
                    self.declare_data_item(item);
                }
            }
        }

        if let Some(procedure_division) = &self.program.procedure_division {
            for paragraph in &procedure_division.paragraphs {
                let name = paragraph.name.to_ascii_uppercase();
                self.paragraphs.insert(name.clone());
                self.declare(Symbol {
                    name,
                    span: paragraph.span,
                    data_type: DataType::Alphanumeric,
                    kind: SymbolKind::Paragraph,
                    level: 0,
                    scope: Scope::Global,
                });
            }
        }
    }

    fn declare_data_item(&mut self, item: &DataItem) {
        let data_type = item
            .picture
            .as_deref()
            .map(parse_picture)
            .unwrap_or(DataType::Alphanumeric);
        self.declare(Symbol {
            name: item.name.to_ascii_uppercase(),
            span: item.span,
            data_type,
            kind: SymbolKind::DataItem,
            level: item.level,
            scope: Scope::Global,
        });
        for child in &item.children {
            self.declare_data_item(child);
        }
    }

    fn declare_file(&mut self, file: &FileDescription) {
        let name = file.file_name.to_ascii_uppercase();
        self.files.insert(name.clone());
        self.declare(Symbol {
            name,
            span: file.span,
            data_type: DataType::Alphanumeric,
            kind: SymbolKind::File,
            level: 0,
            scope: Scope::Global,
        });
    }

    fn declare(&mut self, symbol: Symbol) {
        if !self.symbol_table.declare(symbol.clone()) {
            self.diagnostics
                .push(errors::duplicate_identifier(&symbol.name, symbol.span));
        }
    }

    fn validate_program(&mut self) {
        if let Some(data_division) = &self.program.data_division {
            if let Some(file_section) = &data_division.file_section {
                for file in &file_section.file_descriptions {
                    self.validate_file_association(file);
                }
            }
            if let Some(working_storage) = &data_division.working_storage_section {
                for item in &working_storage.data_items {
                    self.validate_data_item(item);
                }
            }
        }

        if let Some(procedure_division) = &self.program.procedure_division {
            for paragraph in &procedure_division.paragraphs {
                for statement in &paragraph.statements {
                    self.validate_statement(statement);
                }
            }
        }
    }

    fn validate_file_association(&mut self, file: &FileDescription) {
        // The current AST has no OPEN/READ/WRITE/CLOSE statements to associate
        // with declarations. Check declaration consistency here; use sites can
        // be validated when those statement nodes are added.
        if !self.files.contains(&file.file_name.to_ascii_uppercase()) {
            self.diagnostics
                .push(errors::file_not_declared(&file.file_name, file.span));
        }
    }

    fn validate_data_item(&mut self, item: &DataItem) {
        if let Some(picture) = &item.picture
            && !is_valid_picture(picture)
        {
            self.diagnostics
                .push(errors::invalid_picture(picture, item.span));
        }
        for child in &item.children {
            self.validate_data_item(child);
        }
    }

    fn validate_statement(&mut self, statement: &Statement) {
        match statement {
            Statement::Move(move_statement) => {
                let source_type = self.expression_type(&move_statement.source);
                for target in &move_statement.targets {
                    let target_type = self.expression_type(target);
                    if let (Some(source_type), Some(target_type)) = (source_type, target_type)
                        && !is_move_compatible(source_type, target_type)
                    {
                        self.diagnostics.push(errors::incompatible_move(
                            data_type_name(source_type),
                            data_type_name(target_type),
                            move_statement.span,
                        ));
                    }
                }
            }
            Statement::Display(display_statement) => {
                for item in &display_statement.items {
                    if let crate::parser::DisplayItem::Expression(expression) = item {
                        self.expression_type(expression);
                    }
                }
            }
            Statement::Accept(accept_statement) => {
                self.expression_type(&accept_statement.target);
            }
            Statement::Perform(perform_statement) => match perform_statement {
                PerformStatement::ProcedureName(name, span) => self.check_paragraph(name, *span),
                PerformStatement::Times {
                    times,
                    paragraph,
                    span,
                } => {
                    self.check_numeric_expression(times);
                    self.check_paragraph(paragraph, *span);
                }
                PerformStatement::Until {
                    condition,
                    paragraph,
                    span,
                } => {
                    self.validate_condition(condition);
                    self.check_paragraph(paragraph, *span);
                }
            },
            Statement::If(if_statement) => {
                self.validate_condition(&if_statement.condition);
                for nested in &if_statement.then_statements {
                    self.validate_statement(nested);
                }
                if let Some(else_statements) = &if_statement.else_statements {
                    for nested in else_statements {
                        self.validate_statement(nested);
                    }
                }
            }
            Statement::GoTo(go_to) => self.check_paragraph(&go_to.target, go_to.span),
            Statement::StopRun(_) | Statement::Paragraph(_, _) => {}
        }
    }

    fn check_paragraph(&mut self, name: &str, span: Span) {
        if !self.paragraphs.contains(&name.to_ascii_uppercase()) {
            self.diagnostics
                .push(errors::undefined_paragraph(name, span));
        }
    }

    fn expression_type(&mut self, expression: &Expression) -> Option<DataType> {
        match expression {
            Expression::Identifier(name, span) => {
                match self.symbol_table.lookup(name, Scope::Global) {
                    Some(symbol) if symbol.kind == SymbolKind::DataItem => Some(symbol.data_type),
                    _ => {
                        self.diagnostics
                            .push(errors::undefined_identifier(name, *span));
                        None
                    }
                }
            }
            Expression::NumericLiteral(_, _) => Some(DataType::Numeric),
            Expression::StringLiteral(_, _) => Some(DataType::Alphanumeric),
            Expression::UnaryOp { operand, span, .. } => {
                self.check_numeric_expression_at(operand, *span);
                Some(DataType::Numeric)
            }
            Expression::BinaryOp {
                left,
                op,
                right,
                span,
            } => {
                let left_type = self.expression_type(left);
                let right_type = self.expression_type(right);
                if is_arithmetic_operator(*op) {
                    for data_type in [left_type, right_type].into_iter().flatten() {
                        if !can_operate_arithmetic(data_type) {
                            self.diagnostics
                                .push(errors::numeric_operand_required(*span));
                        }
                    }
                    Some(DataType::Numeric)
                } else {
                    left_type.or(right_type)
                }
            }
        }
    }

    fn check_numeric_expression(&mut self, expression: &Expression) {
        self.check_numeric_expression_at(expression, expression_span(expression));
    }

    fn check_numeric_expression_at(&mut self, expression: &Expression, span: Span) {
        if let Some(data_type) = self.expression_type(expression)
            && !can_operate_arithmetic(data_type)
        {
            self.diagnostics
                .push(errors::numeric_operand_required(span));
        }
    }

    fn validate_condition(&mut self, condition: &Condition) {
        match condition {
            Condition::Relation { left, right, .. } => {
                let left_type = self.expression_type(left);
                let right_type = self.expression_type(right);
                if let (Some(left_type), Some(right_type)) = (left_type, right_type)
                    && left_type != right_type
                    && !(is_move_compatible(left_type, right_type)
                        && is_move_compatible(right_type, left_type))
                {
                    self.diagnostics.push(errors::type_mismatch(
                        expression_span(right),
                        left_type,
                        right_type,
                    ));
                }
            }
            Condition::And { left, right, .. } | Condition::Or { left, right, .. } => {
                self.validate_condition(left);
                self.validate_condition(right);
            }
            Condition::Not { condition, .. } => self.validate_condition(condition),
        }
    }
}

fn data_type_name(data_type: DataType) -> &'static str {
    match data_type {
        DataType::Numeric => "numeric",
        DataType::Alphabetic => "alphabetic",
        DataType::Alphanumeric => "alphanumeric",
    }
}

fn is_arithmetic_operator(operator: BinaryOperator) -> bool {
    matches!(
        operator,
        BinaryOperator::Add
            | BinaryOperator::Subtract
            | BinaryOperator::Multiply
            | BinaryOperator::Divide
            | BinaryOperator::Power
    )
}

fn expression_span(expression: &Expression) -> Span {
    match expression {
        Expression::Identifier(_, span)
        | Expression::NumericLiteral(_, span)
        | Expression::StringLiteral(_, span) => *span,
        Expression::BinaryOp { span, .. } | Expression::UnaryOp { span, .. } => *span,
    }
}
