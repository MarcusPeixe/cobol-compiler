use std::path::{Path, PathBuf};

use cobol_compiler::diagnostics::{Diagnostic, Severity};
use cobol_compiler::lexer::lex;
use cobol_compiler::parser::parse;
use cobol_compiler::preprocess::{FsLoader, preprocess};
use cobol_compiler::source::{ReaderOptions, SourceMap};

#[test]
fn test_prog001_divisoes() {
    test_parser_golden("prog001_divisoes.cob");
}

#[test]
fn test_prog002_operacoes() {
    test_parser_golden("prog002_operacoes.cob");
}

#[test]
fn test_prog003_calculadora() {
    test_parser_golden("prog003_calculadora.cob");
}

#[test]
fn test_prog004_folha_pgto() {
    test_parser_golden("prog004_folha_pgto.cob");
}

#[test]
fn test_prog005_strings() {
    test_parser_golden("prog005_strings.cob");
}

#[test]
fn test_prog006_files() {
    test_parser_golden("prog006_files.cob");
}

#[test]
fn test_prog007_loja() {
    test_parser_golden("prog007_loja.cob");
}

#[test]
fn test_minimal_program() {
    test_parser_golden("parser_test_minimal_program.cob");
}

#[test]
fn test_identification_division_complete() {
    test_parser_golden("parser_test_identification_division_complete.cob");
}

#[test]
fn test_environment_division() {
    test_parser_golden("parser_test_environment_division.cob");
}

#[test]
fn test_data_division_working_storage() {
    test_parser_golden("parser_test_data_division_working_storage.cob");
}

#[test]
fn test_procedure_division_display() {
    test_parser_golden("parser_test_procedure_division_display.cob");
}

#[test]
fn test_procedure_division_multiple_statements_no_periods() {
    test_parser_golden("parser_test_procedure_division_multiple_statements.cob");
}

#[test]
fn test_procedure_division_stop_run() {
    test_parser_golden("parser_test_procedure_division_stop_run.cob");
}

#[test]
fn test_procedure_division_multiple_paragraphs() {
    test_parser_golden("parser_test_procedure_division_multiple_paragraphs.cob");
}

#[test]
fn test_procedure_division_move_statement() {
    test_parser_golden("parser_test_procedure_division_move_statement.cob");
}

#[test]
fn test_procedure_division_if_statement() {
    test_parser_golden("parser_test_procedure_division_if_statement.cob");
}

#[test]
fn test_complete_program() {
    test_parser_golden("parser_test_complete_program.cob");
}

fn test_parser_golden(file_name: &str) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let relative_path = Path::new("cobol-examples").join(file_name);
    let source_path = root.join(&relative_path);

    let source = std::fs::read_to_string(&source_path).expect("cannot read source");
    let actual = parse_cobol_string(&source).expect("parsing should succeed");

    insta::assert_snapshot!(file_name, actual);
}

fn parse_cobol_string(source: &str) -> Result<String, String> {
    let mut source_map = SourceMap::default();
    let file_id = source_map.add_file("test.cob".to_string(), source.to_string());
    let loader = FsLoader {
        search_dirs: vec![],
    };

    let (logical_text, pp_diags) =
        preprocess(&mut source_map, file_id, ReaderOptions::default(), &loader);
    let (tokens, lex_diags) = lex(&logical_text);
    let (program, parse_diags) = parse(tokens);

    let mut all_diags = pp_diags;
    all_diags.extend(lex_diags);
    all_diags.extend(parse_diags);

    let errors: Vec<&Diagnostic> = all_diags.iter().filter(|d| d.is_error()).collect();
    if !errors.is_empty() {
        return Err(format!(
            "parsing failed: {}",
            errors
                .iter()
                .map(|diagnostic| format_diagnostic(diagnostic, &source_map))
                .collect::<Vec<_>>()
                .join("; ")
        ));
    }

    // Format the AST for snapshot comparison
    let mut output = format_ast(&program);
    output.push_str("\nDIAGNOSTICS:\n");
    for diagnostic in all_diags
        .iter()
        .filter(|diagnostic| diagnostic.severity == Severity::Warning)
    {
        output.push_str(&format_diagnostic(diagnostic, &source_map));
        output.push('\n');
    }

    Ok(output)
}

fn format_ast(program: &cobol_compiler::parser::Program) -> String {
    let mut output = String::new();
    output.push_str("Program\n");
    output.push_str("  IdentificationDivision\n");
    output.push_str(&format!(
        "    program_id: {}\n",
        program.identification_division.program_id
    ));

    if let Some(author) = &program.identification_division.author {
        output.push_str(&format!("    author: {}\n", author));
    }
    if let Some(installation) = &program.identification_division.installation {
        output.push_str(&format!("    installation: {}\n", installation));
    }
    if let Some(date_written) = &program.identification_division.date_written {
        output.push_str(&format!("    date_written: {}\n", date_written));
    }
    if let Some(date_compiled) = &program.identification_division.date_compiled {
        output.push_str(&format!("    date_compiled: {}\n", date_compiled));
    }
    if let Some(security) = &program.identification_division.security {
        output.push_str(&format!("    security: {}\n", security));
    }
    if let Some(remarks) = &program.identification_division.remarks {
        output.push_str(&format!("    remarks: {}\n", remarks));
    }

    if let Some(env_div) = &program.environment_division {
        output.push_str("  EnvironmentDivision\n");
        if env_div.configuration_section.is_some() {
            output.push_str("    ConfigurationSection\n");
        }
        if env_div.input_output_section.is_some() {
            output.push_str("    InputOutputSection\n");
        }
    }

    if let Some(data_div) = &program.data_division {
        output.push_str("  DataDivision\n");
        if let Some(file_sec) = &data_div.file_section {
            output.push_str("    FileSection\n");
            for file_desc in &file_sec.file_descriptions {
                output.push_str(&format!("      FileDescription: {}\n", file_desc.file_name));
            }
        }
        if let Some(work_sec) = &data_div.working_storage_section {
            output.push_str("    WorkingStorageSection\n");
            format_data_items(&mut output, &work_sec.data_items, 6);
        }
    }

    if let Some(proc_div) = &program.procedure_division {
        output.push_str("  ProcedureDivision\n");
        for para in &proc_div.paragraphs {
            output.push_str(&format!("    Paragraph: {}\n", para.name));
            for (i, stmt) in para.statements.iter().enumerate() {
                format_statement(&mut output, stmt, 6, i);
            }
        }
    }

    output
}

fn format_data_items(
    output: &mut String,
    items: &[cobol_compiler::parser::DataItem],
    indent: usize,
) {
    for item in items {
        let ind = " ".repeat(indent);
        output.push_str(&format!(
            "{}DataItem (level {}): {}\n",
            ind, item.level, item.name
        ));
        if let Some(pic) = &item.picture {
            output.push_str(&format!("{}  PIC: {}\n", ind, pic));
        }
        if !item.children.is_empty() {
            format_data_items(output, &item.children, indent + 2);
        }
    }
}

fn format_statement(
    output: &mut String,
    stmt: &cobol_compiler::parser::Statement,
    indent: usize,
    index: usize,
) {
    use cobol_compiler::parser::Statement;
    let ind = " ".repeat(indent);

    match stmt {
        Statement::Display(display_stmt) => {
            output.push_str(&format!("{}[{}] DISPLAY\n", ind, index));
            for item in &display_stmt.items {
                match item {
                    cobol_compiler::parser::DisplayItem::String(s, _) => {
                        output.push_str(&format!("{}      \"{}\"\n", ind, s));
                    }
                    cobol_compiler::parser::DisplayItem::Expression(expr) => {
                        output.push_str(&format!("{}      {}\n", ind, format_expression(expr)));
                    }
                }
            }
        }
        Statement::Move(move_stmt) => {
            output.push_str(&format!("{}[{}] MOVE\n", ind, index));
            output.push_str(&format!(
                "{}      from: {}\n",
                ind,
                format_expression(&move_stmt.source)
            ));
            output.push_str(&format!("{}      to: ", ind));
            for (i, target) in move_stmt.targets.iter().enumerate() {
                if i > 0 {
                    output.push_str(", ");
                }
                output.push_str(&format_expression(target));
            }
            output.push('\n');
        }
        Statement::Accept(accept_stmt) => {
            output.push_str(&format!("{}[{}] ACCEPT\n", ind, index));
            output.push_str(&format!(
                "{}      {}\n",
                ind,
                format_expression(&accept_stmt.target)
            ));
        }
        Statement::Perform(perform_stmt) => {
            output.push_str(&format!("{}[{}] PERFORM\n", ind, index));
            use cobol_compiler::parser::PerformStatement;
            match perform_stmt {
                PerformStatement::ProcedureName(name, _) => {
                    output.push_str(&format!("{}      {}\n", ind, name));
                }
                PerformStatement::Times {
                    times, paragraph, ..
                } => {
                    output.push_str(&format!(
                        "{}      {} times {}\n",
                        ind,
                        format_expression(times),
                        paragraph
                    ));
                }
                PerformStatement::Until {
                    paragraph,
                    condition,
                    ..
                } => {
                    output.push_str(&format!(
                        "{}      {} until {}\n",
                        ind,
                        paragraph,
                        format_condition(condition)
                    ));
                }
            }
        }
        Statement::If(if_stmt) => {
            output.push_str(&format!("{}[{}] IF\n", ind, index));
            output.push_str(&format!(
                "{}      condition: {}\n",
                ind,
                format_condition(&if_stmt.condition)
            ));
            output.push_str(&format!(
                "{}      then: {} statements\n",
                ind,
                if_stmt.then_statements.len()
            ));
            if let Some(else_stmts) = &if_stmt.else_statements {
                output.push_str(&format!(
                    "{}      else: {} statements\n",
                    ind,
                    else_stmts.len()
                ));
            }
        }
        Statement::StopRun(_) => {
            output.push_str(&format!("{}[{}] STOP RUN\n", ind, index));
        }
        Statement::GoTo(goto_stmt) => {
            output.push_str(&format!("{}[{}] GO TO\n", ind, index));
            output.push_str(&format!("{}      {}\n", ind, goto_stmt.target));
        }
        Statement::Paragraph(_, _) => {
            // Paragraphs are handled separately in the procedure division loop
        }
    }
}

fn format_expression(expr: &cobol_compiler::parser::Expression) -> String {
    use cobol_compiler::parser::Expression;

    match expr {
        Expression::StringLiteral(s, _) => format!("\"{}\"", s),
        Expression::NumericLiteral(n, _) => n.clone(),
        Expression::Identifier(id, _) => id.clone(),
        Expression::BinaryOp {
            left, op, right, ..
        } => {
            format!(
                "{} {} {}",
                format_expression(left),
                format_binary_operator(op),
                format_expression(right)
            )
        }
        Expression::UnaryOp { op, operand, .. } => {
            format!(
                "{}{}",
                format_unary_operator(op),
                format_expression(operand)
            )
        }
    }
}

fn format_binary_operator(op: &cobol_compiler::parser::BinaryOperator) -> &'static str {
    use cobol_compiler::parser::BinaryOperator;
    match op {
        BinaryOperator::Add => "+",
        BinaryOperator::Subtract => "-",
        BinaryOperator::Multiply => "*",
        BinaryOperator::Divide => "/",
        BinaryOperator::Power => "**",
        BinaryOperator::Equal => "=",
        BinaryOperator::NotEqual => "!=",
        BinaryOperator::LessThan => "<",
        BinaryOperator::GreaterThan => ">",
        BinaryOperator::LessThanOrEqual => "<=",
        BinaryOperator::GreaterThanOrEqual => ">=",
    }
}

fn format_unary_operator(op: &cobol_compiler::parser::UnaryOperator) -> &'static str {
    use cobol_compiler::parser::UnaryOperator;
    match op {
        UnaryOperator::Plus => "+",
        UnaryOperator::Minus => "-",
    }
}

fn format_condition(cond: &cobol_compiler::parser::Condition) -> String {
    use cobol_compiler::parser::Condition;
    match cond {
        Condition::Relation {
            left, op, right, ..
        } => {
            format!(
                "{} {} {}",
                format_expression(left),
                format_binary_operator(op),
                format_expression(right)
            )
        }
        Condition::And { left, right, .. } => {
            format!("{} AND {}", format_condition(left), format_condition(right))
        }
        Condition::Or { left, right, .. } => {
            format!("{} OR {}", format_condition(left), format_condition(right))
        }
        Condition::Not { condition, .. } => {
            format!("NOT {}", format_condition(condition))
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
