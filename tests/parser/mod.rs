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

#[test]
fn parses_environment_division_configuration() {
    let (program, diagnostics) = parse_source(
        "IDENTIFICATION DIVISION. PROGRAM-ID. TEST.\n\
         ENVIRONMENT DIVISION.\n\
         CONFIGURATION SECTION.\n\
         SOURCE-COMPUTER. LINUX.\n\
         OBJECT-COMPUTER. LINUX.",
    );

    assert!(diagnostics.is_empty());
    let environment = program.environment_division.expect("missing environment");
    let configuration = environment.configuration_section.expect("missing configuration");
    assert_eq!(
        configuration.source_computer.unwrap().name.as_deref(),
        Some("LINUX")
    );
}

#[test]
fn parses_environment_division_file_control() {
    let (program, diagnostics) = parse_source(
        "IDENTIFICATION DIVISION. PROGRAM-ID. TEST.\n\
         ENVIRONMENT DIVISION.\n\
         INPUT-OUTPUT SECTION.\
         FILE-CONTROL.\n\
         SELECT CUSTOMER-FILE ASSIGN TO 'customers.dat'.",
    );

    assert!(diagnostics.is_empty());
    let environment = program.environment_division.expect("missing environment");
    let input_output = environment
        .input_output_section
        .expect("missing input-output section");
    assert_eq!(input_output.file_control[0].file_name, "CUSTOMER-FILE");
    assert_eq!(
        input_output.file_control[0].destination.as_deref(),
        Some("customers.dat")
    );
}

#[test]
fn parses_data_division_items() {
    let (program, diagnostics) = parse_source(
        "IDENTIFICATION DIVISION. PROGRAM-ID. TEST.\n\
         DATA DIVISION.\n\
         01 WS-NAME PIC X(20) VALUE 'WORLD'.\n\
         02 WS-COUNT PIC S9(3) VALUE -12.\n\
         88 IS-NEGATIVE VALUE -12.",
    );

    assert!(diagnostics.is_empty());
    let data = program.data_division.expect("missing DATA DIVISION");
    assert_eq!(data.items.len(), 3);
    assert_eq!(data.items[0].name, "WS-NAME");
    assert_eq!(data.items[1].level, "02");
    assert_eq!(data.items[2].level, "88");
}

#[test]
fn parses_data_division_picture_and_signed_value() {
    let (program, diagnostics) = parse_source(
        "IDENTIFICATION DIVISION. PROGRAM-ID. TEST.\n\
         DATA DIVISION.\n\
         77 WS-PRICE PIC 9(5).99 VALUE +123.45.",
    );

    assert!(diagnostics.is_empty());
    let item = &program.data_division.unwrap().items[0];
    assert_eq!(item.picture.as_deref(), Some("9(5).99"));
    assert_eq!(item.value, Some(cobol_compiler::parser::DataValue::Numeric("+123.45".to_string())));
}

#[test]
fn parses_procedure_division_from_full_program() {
    let (program, diagnostics) = parse_source(
        "IDENTIFICATION DIVISION. PROGRAM-ID. TEST.\n\
         PROCEDURE DIVISION.\n\
         MAIN-PARAGRAPH.\n\
         DISPLAY 'HELLO'.\n\
         MOVE 10 TO WS-COUNT.\n\
         STOP RUN.",
    );

    assert!(diagnostics.is_empty());
    assert_eq!(program.procedure_division.paragraphs.len(), 1);
    assert_eq!(program.procedure_division.paragraphs[0].name, "MAIN-PARAGRAPH");
    assert_eq!(program.procedure_division.paragraphs[0].statements.len(), 3);
}

#[test]
fn parses_if_statement_from_full_program() {
    let (program, diagnostics) = parse_source(
        "IDENTIFICATION DIVISION. PROGRAM-ID. TEST.\n\
         PROCEDURE DIVISION.\n\
         MAIN-PARAGRAPH.\n\
         IF WS-COUNT >= 10\n\
         PERFORM SHOW-COUNT\n\
         ELSE\n\
         DISPLAY 'SMALL'\n\
         END-IF.",
    );

    assert!(diagnostics.is_empty());
    let statement = &program.procedure_division.paragraphs[0].statements[0];
    assert!(matches!(statement, cobol_compiler::parser::ProcedureStatement::If { .. }));
}
