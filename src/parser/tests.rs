use crate::lexer::{Keyword, Token, TokenKind};
use crate::source::{FileId, Span};

use super::*;

fn test_span() -> Span {
    Span::new(FileId(0), 0, 0)
}

#[test]
fn test_empty_program() {
    // Minimal IDENTIFICATION DIVISION
    let tokens = vec![
        Token {
            kind: TokenKind::Keyword(Keyword::Identification),
            span: test_span(),
        },
        Token {
            kind: TokenKind::Keyword(Keyword::Division),
            span: test_span(),
        },
        Token {
            kind: TokenKind::Period,
            span: test_span(),
        },
        Token {
            kind: TokenKind::Identifier("TEST".to_string()),
            span: test_span(),
        },
        Token {
            kind: TokenKind::Period,
            span: test_span(),
        },
    ];

    let (program, diagnostics) = parse(tokens);
    assert_eq!(program.identification_division.program_id, "TEST");
    assert!(diagnostics.is_empty());
}

#[test]
fn test_identification_division_with_author() {
    let tokens = vec![
        Token {
            kind: TokenKind::Keyword(Keyword::Identification),
            span: test_span(),
        },
        Token {
            kind: TokenKind::Keyword(Keyword::Division),
            span: test_span(),
        },
        Token {
            kind: TokenKind::Period,
            span: test_span(),
        },
        Token {
            kind: TokenKind::Identifier("MYPROG".to_string()),
            span: test_span(),
        },
        Token {
            kind: TokenKind::Period,
            span: test_span(),
        },
        Token {
            kind: TokenKind::Identifier("AUTHOR".to_string()),
            span: test_span(),
        },
        Token {
            kind: TokenKind::Period,
            span: test_span(),
        },
        Token {
            kind: TokenKind::CommentEntry("John Doe".to_string()),
            span: test_span(),
        },
        Token {
            kind: TokenKind::Period,
            span: test_span(),
        },
    ];

    let (program, diagnostics) = parse(tokens);
    assert_eq!(program.identification_division.program_id, "MYPROG");
    assert_eq!(
        program.identification_division.author,
        Some("John Doe".to_string())
    );
    assert!(diagnostics.is_empty());
}

#[test]
fn test_program_id_with_explicit_keyword() {
    let tokens = vec![
        Token {
            kind: TokenKind::Keyword(Keyword::Identification),
            span: test_span(),
        },
        Token {
            kind: TokenKind::Keyword(Keyword::Division),
            span: test_span(),
        },
        Token {
            kind: TokenKind::Period,
            span: test_span(),
        },
        Token {
            kind: TokenKind::Identifier("PROGRAM-ID".to_string()),
            span: test_span(),
        },
        Token {
            kind: TokenKind::Period,
            span: test_span(),
        },
        Token {
            kind: TokenKind::Identifier("MYPROG".to_string()),
            span: test_span(),
        },
        Token {
            kind: TokenKind::Period,
            span: test_span(),
        },
    ];

    let (program, diagnostics) = parse(tokens);
    assert_eq!(program.identification_division.program_id, "MYPROG");
    assert!(diagnostics.is_empty());
}

#[test]
fn test_data_division_with_simple_items() {
    let tokens = vec![
        Token {
            kind: TokenKind::Keyword(Keyword::Identification),
            span: test_span(),
        },
        Token {
            kind: TokenKind::Keyword(Keyword::Division),
            span: test_span(),
        },
        Token {
            kind: TokenKind::Period,
            span: test_span(),
        },
        Token {
            kind: TokenKind::Identifier("PROG".to_string()),
            span: test_span(),
        },
        Token {
            kind: TokenKind::Period,
            span: test_span(),
        },
        Token {
            kind: TokenKind::Keyword(Keyword::Data),
            span: test_span(),
        },
        Token {
            kind: TokenKind::Keyword(Keyword::Division),
            span: test_span(),
        },
        Token {
            kind: TokenKind::Period,
            span: test_span(),
        },
        Token {
            kind: TokenKind::Identifier("WORKING-STORAGE".to_string()),
            span: test_span(),
        },
        Token {
            kind: TokenKind::Keyword(Keyword::Section),
            span: test_span(),
        },
        Token {
            kind: TokenKind::Period,
            span: test_span(),
        },
        Token {
            kind: TokenKind::NumericLiteral("01".to_string()),
            span: test_span(),
        },
        Token {
            kind: TokenKind::Identifier("MY-VAR".to_string()),
            span: test_span(),
        },
        Token {
            kind: TokenKind::Picture,
            span: test_span(),
        },
        Token {
            kind: TokenKind::PictureString("9(5)".to_string()),
            span: test_span(),
        },
        Token {
            kind: TokenKind::Period,
            span: test_span(),
        },
    ];

    let (program, diagnostics) = parse(tokens);
    assert_eq!(program.identification_division.program_id, "PROG");
    assert!(program.data_division.is_some());
    if let Some(data_div) = program.data_division {
        if let Some(work_sec) = data_div.working_storage_section {
            assert_eq!(work_sec.data_items.len(), 1);
            assert_eq!(work_sec.data_items[0].name, "MY-VAR");
            assert_eq!(work_sec.data_items[0].picture, Some("9(5)".to_string()));
        } else {
            panic!("Expected working storage section");
        }
    } else {
        panic!("Expected data division");
    }
    assert!(diagnostics.is_empty());
}

#[test]
fn test_procedure_division_with_paragraph() {
    let tokens = vec![
        Token {
            kind: TokenKind::Keyword(Keyword::Identification),
            span: test_span(),
        },
        Token {
            kind: TokenKind::Keyword(Keyword::Division),
            span: test_span(),
        },
        Token {
            kind: TokenKind::Period,
            span: test_span(),
        },
        Token {
            kind: TokenKind::Identifier("PROG".to_string()),
            span: test_span(),
        },
        Token {
            kind: TokenKind::Period,
            span: test_span(),
        },
        Token {
            kind: TokenKind::Keyword(Keyword::Procedure),
            span: test_span(),
        },
        Token {
            kind: TokenKind::Keyword(Keyword::Division),
            span: test_span(),
        },
        Token {
            kind: TokenKind::Period,
            span: test_span(),
        },
        Token {
            kind: TokenKind::Identifier("MAIN-PROC".to_string()),
            span: test_span(),
        },
        Token {
            kind: TokenKind::Period,
            span: test_span(),
        },
        Token {
            kind: TokenKind::Keyword(Keyword::Stop),
            span: test_span(),
        },
        Token {
            kind: TokenKind::Keyword(Keyword::Run),
            span: test_span(),
        },
        Token {
            kind: TokenKind::Period,
            span: test_span(),
        },
    ];

    let (program, diagnostics) = parse(tokens);
    assert_eq!(program.identification_division.program_id, "PROG");
    assert!(program.procedure_division.is_some());
    if let Some(proc_div) = program.procedure_division {
        assert_eq!(proc_div.paragraphs.len(), 1);
        assert_eq!(proc_div.paragraphs[0].name, "MAIN-PROC");
    } else {
        panic!("Expected procedure division");
    }
    assert!(diagnostics.is_empty());
}

#[test]
fn test_expression_parsing_numeric_literal() {
    let tokens = vec![
        Token {
            kind: TokenKind::Keyword(Keyword::Identification),
            span: test_span(),
        },
        Token {
            kind: TokenKind::Keyword(Keyword::Division),
            span: test_span(),
        },
        Token {
            kind: TokenKind::Period,
            span: test_span(),
        },
        Token {
            kind: TokenKind::Identifier("PROG".to_string()),
            span: test_span(),
        },
        Token {
            kind: TokenKind::Period,
            span: test_span(),
        },
        Token {
            kind: TokenKind::Keyword(Keyword::Procedure),
            span: test_span(),
        },
        Token {
            kind: TokenKind::Keyword(Keyword::Division),
            span: test_span(),
        },
        Token {
            kind: TokenKind::Period,
            span: test_span(),
        },
        Token {
            kind: TokenKind::Identifier("MAIN".to_string()),
            span: test_span(),
        },
        Token {
            kind: TokenKind::Period,
            span: test_span(),
        },
        Token {
            kind: TokenKind::Keyword(Keyword::Display),
            span: test_span(),
        },
        Token {
            kind: TokenKind::NumericLiteral("42".to_string()),
            span: test_span(),
        },
        Token {
            kind: TokenKind::Period,
            span: test_span(),
        },
    ];

    let (program, diagnostics) = parse(tokens);
    assert_eq!(program.identification_division.program_id, "PROG");
    assert!(diagnostics.is_empty());
}

#[test]
fn test_error_missing_period_after_identification() {
    let tokens = vec![
        Token {
            kind: TokenKind::Keyword(Keyword::Identification),
            span: test_span(),
        },
        Token {
            kind: TokenKind::Keyword(Keyword::Division),
            span: test_span(),
        },
        // Missing period here
        Token {
            kind: TokenKind::Identifier("PROG".to_string()),
            span: test_span(),
        },
    ];

    let (_program, diagnostics) = parse(tokens);
    assert!(!diagnostics.is_empty());
    assert!(diagnostics.iter().any(|d| d.is_error()));
}
