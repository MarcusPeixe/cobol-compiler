use std::path::{Path, PathBuf};

use cobol_compiler::diagnostics::{Diagnostic, Severity};
use cobol_compiler::lexer::lex;
use cobol_compiler::parser::parse;
use cobol_compiler::parser::{DataItem, DisplayItem, Expression, IoHandlers, Program, Statement};
use cobol_compiler::preprocess::{FsLoader, preprocess};
use cobol_compiler::semantic::analyze;
use cobol_compiler::source::{ReaderOptions, SourceMap};

#[test]
fn test_valid_program() {
    test_semantic_golden("semantic_test_valid_program.cob");
}

#[test]
fn test_duplicate_identifier() {
    test_semantic_golden("semantic_test_duplicate_identifier.cob");
}

#[test]
fn test_undefined_paragraph() {
    test_semantic_golden("semantic_test_undefined_paragraph.cob");
}

#[test]
fn test_undefined_identifier() {
    test_semantic_golden("semantic_test_undefined_identifier.cob");
}

#[test]
fn test_move_incompatible() {
    test_semantic_golden("semantic_test_move_incompatible.cob");
}

#[test]
fn test_multiple_errors() {
    test_semantic_golden("semantic_test_multiple_errors.cob");
}

#[test]
fn test_prog001_divisoes() {
    test_semantic_golden("prog001_divisoes.cob");
}

#[test]
fn test_prog002_operacoes() {
    test_semantic_golden("prog002_operacoes.cob");
}

#[test]
fn test_prog003_calculadora() {
    test_semantic_golden("prog003_calculadora.cob");
}

#[test]
fn test_prog004_folha_pgto() {
    test_semantic_golden("prog004_folha_pgto.cob");
}

#[test]
fn test_prog005_strings() {
    test_semantic_golden("prog005_strings.cob");
}

#[test]
fn test_prog006_files() {
    test_semantic_golden("prog006_files.cob");
}

#[test]
fn test_prog007_loja() {
    test_semantic_golden("prog007_loja.cob");
}

fn test_semantic_golden(file_name: &str) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source_path = root.join(Path::new("cobol-examples").join(file_name));
    let source = std::fs::read_to_string(source_path).expect("cannot read source");
    let actual = analyze_cobol_string(&source).expect("analysis should complete");

    insta::assert_snapshot!(file_name, actual);
}

fn analyze_cobol_string(source: &str) -> Result<String, String> {
    let mut source_map = SourceMap::default();
    let file_id = source_map.add_file("test.cob", source.to_string());
    let loader = FsLoader {
        search_dirs: Vec::new(),
    };
    let (logical_text, mut diagnostics) =
        preprocess(&mut source_map, file_id, ReaderOptions::default(), &loader);
    let (tokens, lexer_diagnostics) = lex(&logical_text);
    diagnostics.extend(lexer_diagnostics);
    let (program, parser_diagnostics) = parse(tokens);
    diagnostics.extend(parser_diagnostics);
    let (analyzed, semantic_diagnostics) = analyze(&program);
    diagnostics.extend(semantic_diagnostics);

    let mut output = format_ast(&analyzed.program);
    output.push_str("\nSYMBOL TABLE:\n");
    let mut symbols: Vec<_> = analyzed.symbol_table.all_symbols().collect();
    symbols.sort_by(|left, right| left.name.cmp(&right.name));
    for symbol in symbols {
        output.push_str(&format!(
            "  {}: {:?} (level {})\n",
            symbol.name, symbol.data_type, symbol.level
        ));
    }
    output.push_str("\nDIAGNOSTICS:\n");
    for diagnostic in &diagnostics {
        output.push_str(&format_diagnostic(diagnostic, &source_map));
        output.push('\n');
    }
    Ok(output)
}

fn format_ast(program: &Program) -> String {
    let mut output = format!(
        "Program\n  IdentificationDivision\n    program_id: {}\n",
        program.identification_division.program_id
    );

    if let Some(environment) = &program.environment_division {
        output.push_str("  EnvironmentDivision\n");
        if environment.configuration_section.is_some() {
            output.push_str("    ConfigurationSection\n");
        }
        if environment.input_output_section.is_some() {
            output.push_str("    InputOutputSection\n");
        }
    }

    if let Some(data) = &program.data_division {
        output.push_str("  DataDivision\n");
        if let Some(file_section) = &data.file_section {
            output.push_str("    FileSection\n");
            for file in &file_section.file_descriptions {
                output.push_str(&format!("      FileDescription: {}\n", file.file_name));
                for record in &file.records {
                    format_data_item(&mut output, record, 4);
                }
            }
        }
        if let Some(working_storage) = &data.working_storage_section {
            output.push_str("    WorkingStorageSection\n");
            for item in &working_storage.data_items {
                format_data_item(&mut output, item, 3);
            }
        }
    }

    if let Some(procedure) = &program.procedure_division {
        output.push_str("  ProcedureDivision\n");
        for paragraph in &procedure.paragraphs {
            output.push_str(&format!("    Paragraph: {}\n", paragraph.name));
            for statement in &paragraph.statements {
                format_statement(&mut output, statement, 3);
            }
        }
    }

    output
}

fn format_data_item(output: &mut String, item: &DataItem, depth: usize) {
    let indent = "  ".repeat(depth);
    output.push_str(&format!(
        "{indent}DataItem: {} (level {}",
        item.name, item.level
    ));
    if let Some(picture) = &item.picture {
        output.push_str(&format!(", PIC {picture})\n"));
    } else {
        output.push_str(")\n");
    }
    if let Some(occurs) = &item.occurs {
        output.push_str(&format!("{indent}  OCCURS: {occurs}\n"));
    }
    if !item.indexed_by.is_empty() {
        output.push_str(&format!(
            "{indent}  INDEXED BY: {}\n",
            item.indexed_by.join(", ")
        ));
    }
    if let Some(value) = &item.value {
        let value = match value {
            cobol_compiler::parser::Literal::Numeric(value, _) => value.clone(),
            cobol_compiler::parser::Literal::String(value, _) => format!("\"{value}\""),
            cobol_compiler::parser::Literal::FigurativeConstant(value, _) => value.clone(),
        };
        output.push_str(&format!("{indent}  VALUE: {value}\n"));
    }
    for child in &item.children {
        format_data_item(output, child, depth + 1);
    }
}

fn format_statement(output: &mut String, statement: &Statement, depth: usize) {
    let indent = "  ".repeat(depth);
    match statement {
        Statement::Move(_) => output.push_str(&format!("{indent}Move\n")),
        Statement::Compute(compute) => {
            output.push_str(&format!("{indent}Compute\n"));
            output.push_str(&format!(
                "{indent}  Target: {}\n",
                format_expression(&compute.target)
            ));
            output.push_str(&format!(
                "{indent}  Expression: {}\n",
                format_expression(&compute.expression)
            ));
        }
        Statement::Display(display) => {
            output.push_str(&format!("{indent}Display\n"));
            for item in &display.items {
                match item {
                    DisplayItem::String(value, _) => {
                        output.push_str(&format!("{indent}  String: {value}\n"));
                    }
                    DisplayItem::Expression(expression) => {
                        output.push_str(&format!(
                            "{indent}  Expression: {}\n",
                            format_expression(expression)
                        ));
                    }
                }
            }
        }
        Statement::Accept(_) => output.push_str(&format!("{indent}Accept\n")),
        Statement::Initialize(_) => output.push_str(&format!("{indent}Initialize\n")),
        Statement::Perform(_) => output.push_str(&format!("{indent}Perform\n")),
        Statement::If(if_statement) => {
            output.push_str(&format!("{indent}If\n"));
            for nested in &if_statement.then_statements {
                format_statement(output, nested, depth + 1);
            }
            if let Some(else_statements) = &if_statement.else_statements {
                output.push_str(&format!("{indent}Else\n"));
                for nested in else_statements {
                    format_statement(output, nested, depth + 1);
                }
            }
        }
        Statement::Add(add) => {
            output.push_str(&format!("{indent}Add\n"));
            output.push_str(&format!(
                "{indent}  Operands: {}\n",
                format_expression_list(&add.operands)
            ));
            output.push_str(&format!(
                "{indent}  {}: {}\n",
                if add.giving { "Giving" } else { "To" },
                format_expression_list(&add.targets)
            ));
            if !add.giving_targets.is_empty() {
                output.push_str(&format!(
                    "{indent}  Giving: {}\n",
                    format_expression_list(&add.giving_targets)
                ));
            }
            for nested in &add.on_size_error {
                format_statement(output, nested, depth + 1);
            }
        }
        Statement::Subtract(subtract) => {
            output.push_str(&format!("{indent}Subtract\n"));
            output.push_str(&format!(
                "{indent}  Operands: {}\n",
                format_expression_list(&subtract.operands)
            ));
            output.push_str(&format!(
                "{indent}  From: {}\n",
                format_expression_list(&subtract.from)
            ));
            if !subtract.giving.is_empty() {
                output.push_str(&format!(
                    "{indent}  Giving: {}\n",
                    format_expression_list(&subtract.giving)
                ));
            }
            for nested in &subtract.on_size_error {
                format_statement(output, nested, depth + 1);
            }
        }
        Statement::Multiply(multiply) => {
            output.push_str(&format!("{indent}Multiply\n"));
            output.push_str(&format!(
                "{indent}  Factor: {}\n",
                format_expression(&multiply.factor)
            ));
            output.push_str(&format!(
                "{indent}  By: {}\n",
                format_expression_list(&multiply.by)
            ));
            if !multiply.giving.is_empty() {
                output.push_str(&format!(
                    "{indent}  Giving: {}\n",
                    format_expression_list(&multiply.giving)
                ));
            }
            for nested in &multiply.on_size_error {
                format_statement(output, nested, depth + 1);
            }
        }
        Statement::Divide(divide) => {
            output.push_str(&format!("{indent}Divide\n"));
            output.push_str(&format!(
                "{indent}  Dividend: {}\n",
                format_expression(&divide.dividend)
            ));
            output.push_str(&format!(
                "{indent}  Divisor: {}\n",
                format_expression(&divide.divisor)
            ));
            if !divide.giving.is_empty() {
                output.push_str(&format!(
                    "{indent}  Giving: {}\n",
                    format_expression_list(&divide.giving)
                ));
            }
            if let Some(remainder) = &divide.remainder {
                output.push_str(&format!(
                    "{indent}  Remainder: {}\n",
                    format_expression(remainder)
                ));
            }
            for nested in &divide.on_size_error {
                format_statement(output, nested, depth + 1);
            }
        }
        Statement::StopRun(_) => output.push_str(&format!("{indent}StopRun\n")),
        Statement::GoTo(go_to) => {
            output.push_str(&format!("{indent}GoTo: {}\n", go_to.target));
        }
        Statement::Paragraph(name, _) => {
            output.push_str(&format!("{indent}Paragraph: {name}\n"));
        }
        Statement::Set(_) => output.push_str(&format!("{indent}Set\n")),
        Statement::Search(search) => {
            output.push_str(&format!("{indent}Search\n"));
            format_block(output, search.at_end.iter().flatten(), depth + 1);
            for when in &search.whens {
                format_block(output, &when.statements, depth + 1);
            }
        }
        Statement::Inspect(_) => output.push_str(&format!("{indent}Inspect\n")),
        Statement::Evaluate(evaluate) => {
            output.push_str(&format!("{indent}Evaluate\n"));
            for when in &evaluate.whens {
                format_block(output, &when.statements, depth + 1);
            }
        }
        Statement::Open(_) => output.push_str(&format!("{indent}Open\n")),
        Statement::Close(_) => output.push_str(&format!("{indent}Close\n")),
        Statement::Read(read) => {
            output.push_str(&format!("{indent}Read: {}\n", read.file.0));
            format_handlers(output, &read.handlers, depth + 1);
        }
        Statement::Write(write) => {
            output.push_str(&format!("{indent}Write\n"));
            format_handlers(output, &write.handlers, depth + 1);
        }
        Statement::Start(start) => {
            output.push_str(&format!("{indent}Start: {}\n", start.file.0));
            format_handlers(output, &start.handlers, depth + 1);
        }
    }
}

fn format_block<'a>(
    output: &mut String,
    statements: impl IntoIterator<Item = &'a Statement>,
    depth: usize,
) {
    for statement in statements {
        format_statement(output, statement, depth);
    }
}

fn format_handlers(output: &mut String, handlers: &IoHandlers, depth: usize) {
    for block in [
        &handlers.at_end,
        &handlers.not_at_end,
        &handlers.invalid_key,
        &handlers.not_invalid_key,
    ]
    .into_iter()
    .flatten()
    {
        format_block(output, block, depth);
    }
}

fn format_expression_list(expressions: &[Expression]) -> String {
    expressions
        .iter()
        .map(format_expression)
        .collect::<Vec<_>>()
        .join(", ")
}

fn format_expression(expression: &Expression) -> String {
    match expression {
        Expression::Identifier(name, _) => name.clone(),
        Expression::NumericLiteral(value, _) => value.clone(),
        Expression::StringLiteral(value, _) => format!("\"{value}\""),
        Expression::FigurativeConstant(word, _) => word.clone(),
        Expression::Subscript { name, indices, .. } => {
            format!("{name}({})", format_expression_list(indices))
        }
        Expression::RefMod {
            name,
            start,
            length,
            ..
        } => match length {
            Some(length) => format!(
                "{name}({}:{})",
                format_expression(start),
                format_expression(length)
            ),
            None => format!("{name}({}:)", format_expression(start)),
        },
        Expression::FunctionCall {
            name, arguments, ..
        } => format!("FUNCTION {}({})", name, format_expression_list(arguments)),
        Expression::BinaryOp {
            left, op, right, ..
        } => {
            format!(
                "{} {op:?} {}",
                format_expression(left),
                format_expression(right)
            )
        }
        Expression::UnaryOp { op, operand, .. } => {
            format!("{op:?} {}", format_expression(operand))
        }
    }
}

fn format_diagnostic(diagnostic: &Diagnostic, source_map: &SourceMap) -> String {
    let location = diagnostic.span.map_or_else(
        || "<unknown>".to_string(),
        |span| {
            let file = &source_map.file(span.file).name;
            let (line, column) = source_map.line_col(span.file, span.start);
            format!("{file}:{line:03}:{column:03}")
        },
    );
    let severity = match diagnostic.severity {
        Severity::Info => "INFO",
        Severity::Warning => "WARNING",
        Severity::Error => "ERROR",
        Severity::Severe => "SEVERE",
    };
    format!(
        "{severity} {} {location} {}",
        diagnostic.code, diagnostic.message
    )
}
