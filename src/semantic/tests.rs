use crate::parser::{
    BinaryOperator, Condition, DataDivision, DataItem, DisplayItem, DisplayStatement, Expression,
    GoToStatement, IdentificationDivision, IfStatement, MoveStatement, Paragraph,
    ProcedureDivision, Program, Statement, WorkingStorageSection,
};
use crate::source::{FileId, Span};

use super::{DataType, analyze};

fn span() -> Span {
    Span::new(FileId(0), 0, 1)
}

fn program(items: Vec<DataItem>, statements: Vec<Statement>) -> Program {
    Program {
        identification_division: IdentificationDivision {
            span: span(),
            program_id: "TEST".to_string(),
            author: None,
            installation: None,
            date_written: None,
            date_compiled: None,
            security: None,
            remarks: None,
        },
        environment_division: None,
        data_division: Some(DataDivision {
            span: span(),
            file_section: None,
            working_storage_section: Some(WorkingStorageSection {
                span: span(),
                data_items: items,
            }),
        }),
        procedure_division: Some(ProcedureDivision {
            span: span(),
            paragraphs: vec![Paragraph {
                span: span(),
                name: "MAIN".to_string(),
                statements,
            }],
        }),
    }
}

fn data_item(name: &str, picture: &str) -> DataItem {
    DataItem {
        span: span(),
        level: 1,
        name: name.to_string(),
        picture: Some(picture.to_string()),
        value: None,
        occurs: None,
        indexed_by: Vec::new(),
        children: Vec::new(),
    }
}

#[test]
fn declares_unique_identifiers_and_infers_picture_types() {
    let input = program(
        vec![data_item("COUNT", "9(5)"), data_item("NAME", "A(10)")],
        Vec::new(),
    );
    let (result, diagnostics) = analyze(&input);

    assert!(diagnostics.is_empty());
    assert_eq!(
        result
            .symbol_table
            .lookup("COUNT", super::Scope::Global)
            .unwrap()
            .data_type,
        DataType::Numeric
    );
    assert_eq!(
        result
            .symbol_table
            .lookup("NAME", super::Scope::Global)
            .unwrap()
            .data_type,
        DataType::Alphabetic
    );
}

#[test]
fn reports_duplicate_identifiers() {
    let input = program(
        vec![data_item("DUP", "X(4)"), data_item("DUP", "9(4)")],
        Vec::new(),
    );
    let (_, diagnostics) = analyze(&input);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code, "cobol::semantic::identifier_duplicate");
}

#[test]
fn reports_undefined_paragraphs_and_identifiers() {
    let input = program(
        Vec::new(),
        vec![
            Statement::GoTo(GoToStatement {
                span: span(),
                target: "MISSING-PARAGRAPH".to_string(),
            }),
            Statement::Display(DisplayStatement {
                span: span(),
                no_advancing: false,
                items: vec![DisplayItem::Expression(Expression::Identifier(
                    "MISSING-VARIABLE".to_string(),
                    span(),
                ))],
            }),
        ],
    );
    let (_, diagnostics) = analyze(&input);

    assert_eq!(diagnostics.len(), 2);
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "cobol::semantic::undefined_paragraph")
    );
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "cobol::semantic::undefined_identifier")
    );
}

#[test]
fn reports_incompatible_move_types() {
    let input = program(
        vec![data_item("TEXT-VALUE", "A(10)"), data_item("COUNT", "9(5)")],
        vec![Statement::Move(MoveStatement {
            span: span(),
            source: Expression::Identifier("TEXT-VALUE".to_string(), span()),
            targets: vec![Expression::Identifier("COUNT".to_string(), span())],
        })],
    );
    let (_, diagnostics) = analyze(&input);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code, "cobol::semantic::move_incompatible");
}

#[test]
fn accepts_numeric_move_to_alphanumeric() {
    let input = program(
        vec![data_item("COUNT", "9(5)"), data_item("TEXT-VALUE", "X(10)")],
        vec![Statement::Move(MoveStatement {
            span: span(),
            source: Expression::Identifier("COUNT".to_string(), span()),
            targets: vec![Expression::Identifier("TEXT-VALUE".to_string(), span())],
        })],
    );
    let (_, diagnostics) = analyze(&input);

    assert!(diagnostics.is_empty());
}

#[test]
fn reports_invalid_picture_clauses() {
    let input = program(vec![data_item("INVALID", "X(0)")], Vec::new());
    let (_, diagnostics) = analyze(&input);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code, "cobol::semantic::invalid_picture");
}

#[test]
fn requires_numeric_arithmetic_operands() {
    let input = program(
        vec![data_item("TEXT-VALUE", "A(10)"), data_item("COUNT", "9(5)")],
        vec![Statement::Move(MoveStatement {
            span: span(),
            source: Expression::BinaryOp {
                left: Box::new(Expression::Identifier("TEXT-VALUE".to_string(), span())),
                op: BinaryOperator::Add,
                right: Box::new(Expression::NumericLiteral("1".to_string(), span())),
                span: span(),
            },
            targets: vec![Expression::Identifier("COUNT".to_string(), span())],
        })],
    );
    let (_, diagnostics) = analyze(&input);

    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "cobol::semantic::numeric_operand_required")
    );
}

#[test]
fn accepts_numeric_arithmetic_operands() {
    let input = program(
        vec![data_item("LEFT-VALUE", "9(5)"), data_item("COUNT", "9(5)")],
        vec![Statement::Move(MoveStatement {
            span: span(),
            source: Expression::BinaryOp {
                left: Box::new(Expression::Identifier("LEFT-VALUE".to_string(), span())),
                op: BinaryOperator::Add,
                right: Box::new(Expression::NumericLiteral("1".to_string(), span())),
                span: span(),
            },
            targets: vec![Expression::Identifier("COUNT".to_string(), span())],
        })],
    );
    let (_, diagnostics) = analyze(&input);

    assert!(diagnostics.is_empty());
}

#[test]
fn validates_conditions_and_nested_statements() {
    let input = program(
        vec![data_item("TEXT-VALUE", "A(10)"), data_item("COUNT", "9(5)")],
        vec![Statement::If(IfStatement {
            span: span(),
            condition: Condition::Relation {
                left: Expression::Identifier("TEXT-VALUE".to_string(), span()),
                op: BinaryOperator::Equal,
                right: Expression::Identifier("COUNT".to_string(), span()),
                span: span(),
            },
            then_statements: vec![Statement::Display(DisplayStatement {
                span: span(),
                no_advancing: false,
                items: vec![DisplayItem::Expression(Expression::Identifier(
                    "MISSING".to_string(),
                    span(),
                ))],
            })],
            else_statements: None,
        })],
    );
    let (_, diagnostics) = analyze(&input);

    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "cobol::semantic::type_mismatch")
    );
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "cobol::semantic::undefined_identifier")
    );
}

#[test]
fn reports_duplicate_paragraph_names() {
    let mut input = program(Vec::new(), Vec::new());
    input
        .procedure_division
        .as_mut()
        .unwrap()
        .paragraphs
        .push(Paragraph {
            span: span(),
            name: "MAIN".to_string(),
            statements: Vec::new(),
        });
    let (_, diagnostics) = analyze(&input);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code, "cobol::semantic::identifier_duplicate");
}

#[test]
fn reports_undefined_move_source_and_target() {
    let input = program(
        Vec::new(),
        vec![Statement::Move(MoveStatement {
            span: span(),
            source: Expression::Identifier("MISSING-SOURCE".to_string(), span()),
            targets: vec![Expression::Identifier("MISSING-TARGET".to_string(), span())],
        })],
    );
    let (_, diagnostics) = analyze(&input);

    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == "cobol::semantic::undefined_identifier")
            .count(),
        2
    );
}
