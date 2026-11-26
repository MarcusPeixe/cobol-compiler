use std::collections::{HashMap, HashSet};

use crate::diagnostics::Diagnostic;
use crate::parser::{
    BinaryOperator, Condition, DataItem, EvaluateChoice, Expression, FileDescription,
    InspectRegion, IoHandlers, Literal, PerformStatement, Program, Statement,
};
use crate::source::Span;

use super::errors;
use super::layout::{StorageLayout, compute_layout, picture_info};
use super::symbol_table::{Scope, Symbol, SymbolKind, SymbolTable};
use super::type_system::{
    DataType, can_operate_arithmetic, is_move_compatible, is_valid_picture, parse_picture,
};

#[derive(Debug, Clone)]
pub struct AnalyzedProgram {
    pub program: Program,
    pub symbol_table: SymbolTable,
    pub layout: StorageLayout,
}

/// Table facts about a declared data item, used to check subscripts and I/O.
#[derive(Debug, Clone, Default)]
struct ItemInfo {
    /// Number of `OCCURS` clauses on the item and its ancestors.
    table_depth: usize,
    /// For a record under an FD, the file it belongs to.
    file: Option<String>,
}

pub struct SemanticAnalyzer<'a> {
    program: &'a Program,
    symbol_table: SymbolTable,
    paragraphs: HashSet<String>,
    files: HashSet<String>,
    items: HashMap<(String, Vec<String>), ItemInfo>,
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
            layout: compute_layout(program.data_division.as_ref()),
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
            items: HashMap::new(),
            diagnostics: Vec::new(),
        }
    }

    fn item_info(&self, name: &str) -> Option<&ItemInfo> {
        let (base_name, qualifiers) = split_qualified_name(name);
        let mut matches = self
            .items
            .iter()
            .filter(|((item_name, item_qualifiers), _)| {
                item_name == &base_name.to_ascii_uppercase()
                    && qualifiers_match(item_qualifiers, &qualifiers)
            })
            .map(|(_, info)| info);
        let info = matches.next()?;
        matches.next().is_none().then_some(info)
    }

    fn matching_data_items(&self, name: &str) -> Vec<Symbol> {
        let (base_name, qualifiers) = split_qualified_name(name);
        self.symbol_table
            .lookup_all(base_name, Scope::Global)
            .iter()
            .filter(|symbol| {
                matches!(symbol.kind, SymbolKind::DataItem | SymbolKind::IndexName)
                    && qualifiers_match(&symbol.qualifiers, &qualifiers)
            })
            .cloned()
            .collect()
    }

    fn unique_data_item(&self, name: &str) -> Option<Symbol> {
        let mut matches = self.matching_data_items(name).into_iter();
        let item = matches.next()?;
        matches.next().is_none().then_some(item)
    }

    fn resolve_data_item(&mut self, name: &str, span: Span) -> Option<Symbol> {
        let matches = self.matching_data_items(name);
        match matches.as_slice() {
            [symbol] => Some(symbol.clone()),
            [] => {
                self.diagnostics
                    .push(errors::undefined_identifier(name, span));
                None
            }
            _ => {
                self.diagnostics
                    .push(errors::ambiguous_identifier(name, span));
                None
            }
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
                    self.declare_data_item(item, 0, None, &[]);
                }
            }
        }

        if let Some(procedure_division) = &self.program.procedure_division {
            for paragraph in &procedure_division.paragraphs {
                let name = paragraph.name.to_ascii_uppercase();
                self.paragraphs.insert(name.clone());
                self.declare(Symbol {
                    name,
                    qualifiers: Vec::new(),
                    span: paragraph.span,
                    data_type: DataType::Alphanumeric,
                    kind: SymbolKind::Paragraph,
                    level: 0,
                    scope: Scope::Global,
                });
            }
        }
    }

    fn declare_data_item(
        &mut self,
        item: &DataItem,
        parent_depth: usize,
        file: Option<&str>,
        parents: &[String],
    ) {
        let depth = parent_depth + usize::from(item.occurs.is_some());
        let qualifiers: Vec<_> = parents
            .iter()
            .rev()
            .map(|parent| parent.to_ascii_uppercase())
            .collect();
        self.items
            .entry((item.name.to_ascii_uppercase(), qualifiers.clone()))
            .or_insert(ItemInfo {
                table_depth: depth,
                file: file.map(str::to_string),
            });
        let data_type = item
            .picture
            .as_deref()
            .map(parse_picture)
            .unwrap_or(DataType::Alphanumeric);
        self.declare(Symbol {
            name: item.name.to_ascii_uppercase(),
            qualifiers: qualifiers.clone(),
            span: item.span,
            data_type,
            kind: SymbolKind::DataItem,
            level: item.level,
            scope: Scope::Global,
        });
        for index_name in &item.indexed_by {
            self.declare(Symbol {
                name: index_name.to_ascii_uppercase(),
                qualifiers: Vec::new(),
                span: item.span,
                data_type: DataType::Numeric,
                kind: SymbolKind::IndexName,
                level: 0,
                scope: Scope::Global,
            });
        }
        let mut child_parents = parents.to_vec();
        child_parents.push(item.name.clone());
        for child in &item.children {
            self.declare_data_item(child, depth, file, &child_parents);
        }
    }

    fn declare_file(&mut self, file: &FileDescription) {
        let name = file.file_name.to_ascii_uppercase();
        self.files.insert(name.clone());
        self.declare(Symbol {
            name,
            qualifiers: Vec::new(),
            span: file.span,
            data_type: DataType::Alphanumeric,
            kind: SymbolKind::File,
            level: 0,
            scope: Scope::Global,
        });
        for record in &file.records {
            self.declare_data_item(record, 0, Some(&file.file_name.to_ascii_uppercase()), &[]);
        }
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
                    for record in &file.records {
                        self.validate_data_item(record);
                    }
                }
            }
            if let Some(working_storage) = &data_division.working_storage_section {
                for item in &working_storage.data_items {
                    self.validate_data_item(item);
                }
            }
        }
        self.validate_file_controls();

        if let Some(procedure_division) = &self.program.procedure_division {
            for paragraph in &procedure_division.paragraphs {
                for statement in &paragraph.statements {
                    self.validate_statement(statement);
                }
            }
        }
    }

    fn validate_file_controls(&mut self) {
        let Some(section) = self
            .program
            .environment_division
            .as_ref()
            .and_then(|environment| environment.input_output_section.as_ref())
        else {
            return;
        };
        for control in &section.file_controls {
            if !self.files.contains(&control.file_name.to_ascii_uppercase()) {
                self.diagnostics
                    .push(errors::file_not_declared(&control.file_name, control.span));
            }
            for expression in control
                .assign_to
                .iter()
                .chain(&control.record_key)
                .chain(&control.alternate_keys)
                .chain(&control.file_status)
            {
                self.expression_type(expression);
            }
        }
    }

    fn check_file(&mut self, file: &(String, Span)) {
        if !self.files.contains(&file.0.to_ascii_uppercase()) {
            self.diagnostics
                .push(errors::file_not_declared(&file.0, file.1));
        }
    }

    fn validate_block(&mut self, statements: &[Statement]) {
        for statement in statements {
            self.validate_statement(statement);
        }
    }

    fn validate_handlers(&mut self, handlers: &IoHandlers) {
        for block in [
            &handlers.at_end,
            &handlers.not_at_end,
            &handlers.invalid_key,
            &handlers.not_invalid_key,
        ]
        .into_iter()
        .flatten()
        {
            self.validate_block(block);
        }
    }

    fn validate_region(&mut self, region: &InspectRegion) {
        for limit in region.before.iter().chain(&region.after) {
            self.expression_type(limit);
        }
    }

    fn validate_data_item(&mut self, item: &DataItem) {
        if let Some(picture) = &item.picture
            && !is_valid_picture(picture)
        {
            self.diagnostics
                .push(errors::invalid_picture(picture, item.span));
        }
        if item.picture.is_some() && !item.children.is_empty() {
            self.diagnostics
                .push(errors::group_with_picture(&item.name, item.span));
        }
        if item.picture.is_none() && item.children.is_empty() {
            self.diagnostics
                .push(errors::missing_picture(&item.name, item.span));
        }
        if let Some(count) = &item.occurs
            && !count.parse::<usize>().is_ok_and(|count| count > 0)
        {
            self.diagnostics
                .push(errors::invalid_occurs(&item.name, item.span));
        }
        self.validate_value(item);
        for child in &item.children {
            self.validate_data_item(child);
        }
    }

    fn validate_value(&mut self, item: &DataItem) {
        let (Some(value), Some(info)) =
            (&item.value, item.picture.as_deref().and_then(picture_info))
        else {
            return;
        };
        match value {
            Literal::String(text, span) => {
                if info.data_type == DataType::Numeric {
                    self.diagnostics
                        .push(errors::value_type_mismatch(&item.name, *span));
                } else if text.chars().count() > info.size {
                    self.diagnostics
                        .push(errors::value_too_long(&item.name, *span));
                }
            }
            Literal::Numeric(text, span) => {
                if info.data_type != DataType::Numeric {
                    self.diagnostics
                        .push(errors::value_type_mismatch(&item.name, *span));
                } else if !info.edited {
                    let (integer, fraction) = text.split_once('.').unwrap_or((text, ""));
                    let integer_digits = integer.trim_start_matches('0').len();
                    if integer_digits > info.integer_digits() || fraction.len() > info.scale {
                        self.diagnostics
                            .push(errors::value_too_long(&item.name, *span));
                    }
                }
            }
            Literal::FigurativeConstant(..) => {}
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
            Statement::Compute(compute_statement) => {
                self.check_numeric_expression(&compute_statement.target);
                self.check_numeric_expression(&compute_statement.expression);
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
                PerformStatement::Thru { first, last, span } => {
                    self.check_paragraph(first, *span);
                    self.check_paragraph(last, *span);
                }
                PerformStatement::Varying(varying) => {
                    if let Some(paragraph) = &varying.paragraph {
                        self.check_paragraph(paragraph, varying.span);
                    }
                    self.check_numeric_expression(&varying.variable);
                    self.check_numeric_expression(&varying.from);
                    self.check_numeric_expression(&varying.by);
                    self.validate_condition(&varying.condition);
                    self.validate_block(&varying.body);
                }
                PerformStatement::Inline(inline) => {
                    if let Some(times) = &inline.times {
                        self.check_numeric_expression(times);
                    }
                    if let Some(condition) = &inline.condition {
                        self.validate_condition(condition);
                    }
                    self.validate_block(&inline.body);
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
            Statement::Add(statement) => {
                for expression in statement
                    .operands
                    .iter()
                    .chain(&statement.targets)
                    .chain(&statement.giving_targets)
                {
                    self.check_numeric_expression(expression);
                }
                for nested in &statement.on_size_error {
                    self.validate_statement(nested);
                }
            }
            Statement::Subtract(statement) => {
                for expression in statement
                    .operands
                    .iter()
                    .chain(&statement.from)
                    .chain(&statement.giving)
                {
                    self.check_numeric_expression(expression);
                }
                for nested in &statement.on_size_error {
                    self.validate_statement(nested);
                }
            }
            Statement::Multiply(statement) => {
                self.check_numeric_expression(&statement.factor);
                for expression in statement.by.iter().chain(&statement.giving) {
                    self.check_numeric_expression(expression);
                }
                for nested in &statement.on_size_error {
                    self.validate_statement(nested);
                }
            }
            Statement::Divide(statement) => {
                self.check_numeric_expression(&statement.dividend);
                self.check_numeric_expression(&statement.divisor);
                for expression in &statement.giving {
                    self.check_numeric_expression(expression);
                }
                if let Some(remainder) = &statement.remainder {
                    self.check_numeric_expression(remainder);
                }
                for nested in &statement.on_size_error {
                    self.validate_statement(nested);
                }
            }
            Statement::GoTo(go_to) => self.check_paragraph(&go_to.target, go_to.span),
            Statement::StopRun(_) | Statement::Paragraph(_, _) => {}
            Statement::Set(statement) => {
                for target in &statement.targets {
                    self.check_numeric_expression(target);
                }
                self.check_numeric_expression(&statement.value);
            }
            Statement::Search(statement) => {
                self.expression_type(&statement.table);
                if let Expression::Identifier(name, span) | Expression::Subscript { name, span, .. } =
                    &statement.table
                    && self
                        .item_info(name)
                        .is_some_and(|info| info.table_depth == 0)
                {
                    self.diagnostics
                        .push(errors::search_requires_table(name, *span));
                }
                if let Some(varying) = &statement.varying {
                    self.expression_type(varying);
                }
                if let Some(at_end) = &statement.at_end {
                    self.validate_block(at_end);
                }
                for when in &statement.whens {
                    self.validate_condition(&when.condition);
                    self.validate_block(&when.statements);
                }
            }
            Statement::Inspect(statement) => {
                self.expression_type(&statement.target);
                for tally in &statement.tallying {
                    self.check_numeric_expression(&tally.counter);
                    if let Some(pattern) = &tally.pattern {
                        self.expression_type(pattern);
                    }
                    self.validate_region(&tally.region);
                }
                for replace in &statement.replacing {
                    if let Some(pattern) = &replace.pattern {
                        self.expression_type(pattern);
                    }
                    self.expression_type(&replace.by);
                    self.validate_region(&replace.region);
                }
                if let Some(convert) = &statement.converting {
                    self.expression_type(&convert.from);
                    self.expression_type(&convert.to);
                    self.validate_region(&convert.region);
                }
            }
            Statement::Evaluate(statement) => {
                let subject_type = self.expression_type(&statement.subject);
                for when in &statement.whens {
                    for choice in &when.choices {
                        match choice {
                            EvaluateChoice::Value(expression) => {
                                let choice_type = self.expression_type(expression);
                                self.check_comparable(subject_type, expression, choice_type);
                            }
                            EvaluateChoice::Range(first, last) => {
                                for bound in [first, last] {
                                    let bound_type = self.expression_type(bound);
                                    self.check_comparable(subject_type, bound, bound_type);
                                }
                            }
                            EvaluateChoice::Condition(condition) => {
                                self.validate_condition(condition);
                            }
                            EvaluateChoice::Any => {}
                        }
                    }
                    self.validate_block(&when.statements);
                }
            }
            Statement::Open(statement) => {
                for group in &statement.groups {
                    for file in &group.files {
                        self.check_file(file);
                    }
                }
            }
            Statement::Close(statement) => {
                for file in &statement.files {
                    self.check_file(file);
                }
            }
            Statement::Read(statement) => {
                self.check_file(&statement.file);
                for expression in statement.into.iter().chain(&statement.key) {
                    self.expression_type(expression);
                }
                self.validate_handlers(&statement.handlers);
            }
            Statement::Write(statement) => {
                let record_type = self.expression_type(&statement.record);
                if let Expression::Identifier(name, span) = &statement.record {
                    let is_record = self.item_info(name).is_some_and(|info| info.file.is_some())
                        && self.unique_data_item(name).is_some_and(|symbol| {
                            symbol.kind == SymbolKind::DataItem && symbol.level == 1
                        });
                    if !is_record && self.item_info(name).is_some() {
                        self.diagnostics.push(errors::not_a_record(name, *span));
                    }
                }
                if let Some(from) = &statement.from {
                    let from_type = self.expression_type(from);
                    if let (Some(from_type), Some(record_type)) = (from_type, record_type)
                        && !is_move_compatible(from_type, record_type)
                    {
                        self.diagnostics.push(errors::incompatible_move(
                            data_type_name(from_type),
                            data_type_name(record_type),
                            statement.span,
                        ));
                    }
                }
                if let Some(lines) = statement
                    .advancing
                    .as_ref()
                    .and_then(|advancing| advancing.lines.as_ref())
                {
                    self.check_numeric_expression(lines);
                }
                self.validate_handlers(&statement.handlers);
            }
            Statement::Initialize(statement) => {
                for target in &statement.targets {
                    self.expression_type(target);
                }
            }
            Statement::Start(statement) => {
                self.check_file(&statement.file);
                if let Some((_, key)) = &statement.key {
                    self.expression_type(key);
                }
                self.validate_handlers(&statement.handlers);
            }
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
            Expression::Identifier(name, span) => self
                .resolve_data_item(name, *span)
                .map(|symbol| symbol.data_type),
            Expression::Subscript {
                name,
                indices,
                span,
            } => {
                for index in indices {
                    self.check_numeric_expression(index);
                }
                let symbol = self.resolve_data_item(name, *span)?;
                if symbol.kind != SymbolKind::DataItem {
                    self.diagnostics
                        .push(errors::subscript_on_non_table(name, *span));
                    return None;
                }
                if let Some(depth) = self.item_info(name).map(|info| info.table_depth) {
                    if depth == 0 {
                        self.diagnostics
                            .push(errors::subscript_on_non_table(name, *span));
                    } else if depth != indices.len() {
                        self.diagnostics.push(errors::subscript_count(
                            name,
                            depth,
                            indices.len(),
                            *span,
                        ));
                    }
                }
                Some(symbol.data_type)
            }
            Expression::RefMod {
                name,
                start,
                length,
                span,
            } => {
                self.check_numeric_expression(start);
                if let Some(length) = length {
                    self.check_numeric_expression(length);
                }
                match self.resolve_data_item(name, *span) {
                    Some(symbol) if symbol.kind == SymbolKind::DataItem => {
                        Some(DataType::Alphanumeric)
                    }
                    Some(_) => {
                        self.diagnostics
                            .push(errors::undefined_identifier(name, *span));
                        None
                    }
                    None => None,
                }
            }
            Expression::FigurativeConstant(_, _) => None,
            Expression::NumericLiteral(_, _) => Some(DataType::Numeric),
            Expression::StringLiteral(_, _) => Some(DataType::Alphanumeric),
            Expression::FunctionCall {
                name, arguments, ..
            } => {
                let argument_types = arguments
                    .iter()
                    .map(|argument| self.expression_type(argument))
                    .collect::<Vec<_>>();
                if name == "UPPER-CASE" {
                    argument_types
                        .first()
                        .copied()
                        .flatten()
                        .or(Some(DataType::Alphanumeric))
                } else {
                    Some(DataType::Alphanumeric)
                }
            }
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

    fn check_comparable(
        &mut self,
        left_type: Option<DataType>,
        right: &Expression,
        right_type: Option<DataType>,
    ) {
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

    fn validate_condition(&mut self, condition: &Condition) {
        match condition {
            Condition::Relation { left, right, .. } => {
                let left_type = self.expression_type(left);
                let right_type = self.expression_type(right);
                self.check_comparable(left_type, right, right_type);
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

fn split_qualified_name(name: &str) -> (&str, Vec<String>) {
    let mut parts = name.split_whitespace();
    let base_name = parts.next().unwrap_or(name);
    let mut qualifiers = Vec::new();
    while let Some(connector) = parts.next() {
        let Some(qualifier) = parts.next() else {
            break;
        };
        if connector.eq_ignore_ascii_case("OF") || connector.eq_ignore_ascii_case("IN") {
            qualifiers.push(qualifier.to_ascii_uppercase());
        } else {
            break;
        }
    }
    (base_name, qualifiers)
}

fn qualifiers_match(declared: &[String], requested: &[String]) -> bool {
    declared.len() >= requested.len()
        && declared
            .iter()
            .zip(requested)
            .all(|(declared, requested)| declared.eq_ignore_ascii_case(requested))
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
        | Expression::StringLiteral(_, span)
        | Expression::FigurativeConstant(_, span) => *span,
        Expression::Subscript { span, .. }
        | Expression::RefMod { span, .. }
        | Expression::FunctionCall { span, .. }
        | Expression::BinaryOp { span, .. }
        | Expression::UnaryOp { span, .. } => *span,
    }
}
