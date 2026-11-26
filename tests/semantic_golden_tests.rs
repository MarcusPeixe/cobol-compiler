use std::path::PathBuf;

use cobol_compiler::diagnostics::{Diagnostic, Severity};
use cobol_compiler::lexer::lex;
use cobol_compiler::parser::{format_program, parse};
use cobol_compiler::preprocess::{FsLoader, preprocess};
use cobol_compiler::semantic::{analyze, format_symbol_table};
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
fn test_tables() {
    test_semantic_golden("semantic_test_tables.cob");
}

#[test]
fn test_values() {
    test_semantic_golden("semantic_test_values.cob");
}

#[test]
fn test_records() {
    test_semantic_golden("semantic_test_records.cob");
}

#[test]
fn test_qualified_names() {
    test_semantic_golden("semantic_test_qualified_names.cob");
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
    let source_path = root.join("cobol-examples").join(file_name);
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

    let mut output = format_program(&analyzed.program);
    output.push_str("\nSYMBOL TABLE:\n");
    output.push_str(&format_symbol_table(&analyzed.symbol_table));
    output.push_str("\nSTORAGE LAYOUT:\n");
    output.push_str(&cobol_compiler::semantic::format_layout(&analyzed.layout));
    output.push_str("\nDIAGNOSTICS:\n");
    for diagnostic in &diagnostics {
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
