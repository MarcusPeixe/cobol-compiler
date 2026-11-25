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
fn test_io_clauses() {
    test_parser_golden("parser_test_io_clauses.cob");
}

#[test]
fn test_unsupported_syntax_recovery() {
    test_parser_golden_with_errors("parser_test_unsupported_recovery.cob");
}

#[test]
fn test_perform_forms() {
    test_parser_golden("parser_test_perform_forms.cob");
}

#[test]
fn test_evaluate_forms() {
    test_parser_golden("parser_test_evaluate_forms.cob");
}

#[test]
fn test_search_all() {
    test_parser_golden("parser_test_search_all.cob");
}

#[test]
fn test_reference_modification() {
    test_parser_golden("parser_test_reference_modification.cob");
}

#[test]
fn test_write_advancing() {
    test_parser_golden("parser_test_write_advancing.cob");
}

#[test]
fn test_initialize() {
    test_parser_golden("parser_test_initialize.cob");
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
    test_parser_golden_with_error_snapshot(file_name, false);
}

fn test_parser_golden_with_errors(file_name: &str) {
    test_parser_golden_with_error_snapshot(file_name, true);
}

fn test_parser_golden_with_error_snapshot(file_name: &str, allow_errors: bool) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let relative_path = Path::new("cobol-examples").join(file_name);
    let source_path = root.join(&relative_path);

    let source = std::fs::read_to_string(&source_path).expect("cannot read source");
    let actual = parse_cobol_string(&source, allow_errors)
        .expect("parsing should not have unexpected errors");

    insta::assert_snapshot!(file_name, actual);
}

fn parse_cobol_string(source: &str, allow_errors: bool) -> Result<String, String> {
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
    if !allow_errors && !errors.is_empty() {
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
    let mut output = cobol_compiler::parser::format_program(&program);
    output.push_str("\nDIAGNOSTICS:\n");
    for diagnostic in all_diags
        .iter()
        .filter(|diagnostic| allow_errors || diagnostic.severity == Severity::Warning)
    {
        output.push_str(&format_diagnostic(diagnostic, &source_map));
        output.push('\n');
    }

    Ok(output)
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
