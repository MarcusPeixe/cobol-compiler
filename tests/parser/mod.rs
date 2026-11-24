use cobol_compiler::lexer::lex;
use cobol_compiler::parser::parse;
use cobol_compiler::source::{FileId, LogicalText};

fn parse_source(source: &str) -> (cobol_compiler::parser::Program, Vec<cobol_compiler::diagnostics::Diagnostic>) {
    let mut logical = LogicalText::default();
    for (offset, c) in source.char_indices() {
        logical.push_char(c, FileId(0), offset);
    }
    let (tokens, lexer_diagnostics) = lex(&logical);
    assert!(lexer_diagnostics.is_empty());
    parse(&tokens)
}

#[test]
fn parses_example_identification_division() {
    let (program, diagnostics) = parse_source(
        "IDENTIFICATION DIVISION.\nPROGRAM-ID. GREETING.",
    );

    assert!(diagnostics.is_empty());
    assert_eq!(program.identification_division.program_name(), Some("GREETING"));
}

#[test]
fn does_not_accept_missing_identification_header() {
    let (_, diagnostics) = parse_source("ENVIRONMENT DIVISION.");
    assert_eq!(
        diagnostics[0].code,
        "cobol::parser::missing_identification_division"
    );
}
