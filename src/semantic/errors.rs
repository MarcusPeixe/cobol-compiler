use crate::diagnostics::Diagnostic;
use crate::semantic::DataType;
use crate::source::Span;

pub(crate) const IDENTIFIER_DUPLICATE: &str = "cobol::semantic::identifier_duplicate";
pub(crate) const MOVE_INCOMPATIBLE: &str = "cobol::semantic::move_incompatible";
pub(crate) const UNDEFINED_IDENTIFIER: &str = "cobol::semantic::undefined_identifier";
pub(crate) const UNDEFINED_PARAGRAPH: &str = "cobol::semantic::undefined_paragraph";
pub(crate) const FILE_NOT_DECLARED: &str = "cobol::semantic::file_not_declared";
pub(crate) const INVALID_PICTURE: &str = "cobol::semantic::invalid_picture";
pub(crate) const TYPE_MISMATCH: &str = "cobol::semantic::type_mismatch";
pub(crate) const NUMERIC_OPERAND_REQUIRED: &str = "cobol::semantic::numeric_operand_required";

pub(crate) fn duplicate_identifier(name: &str, span: Span) -> Diagnostic {
    Diagnostic::error(
        IDENTIFIER_DUPLICATE,
        format!("Identifier '{name}' is declared more than once"),
    )
    .with_span(span)
}

pub(crate) fn undefined_identifier(name: &str, span: Span) -> Diagnostic {
    Diagnostic::error(
        UNDEFINED_IDENTIFIER,
        format!("Identifier '{name}' is not defined"),
    )
    .with_span(span)
}

pub(crate) fn undefined_paragraph(name: &str, span: Span) -> Diagnostic {
    Diagnostic::error(
        UNDEFINED_PARAGRAPH,
        format!("Paragraph '{name}' is not defined"),
    )
    .with_span(span)
}

pub(crate) fn incompatible_move(source: &str, target: &str, span: Span) -> Diagnostic {
    Diagnostic::error(
        MOVE_INCOMPATIBLE,
        format!("Cannot MOVE {source} to {target}"),
    )
    .with_span(span)
}

pub(crate) fn invalid_picture(picture: &str, span: Span) -> Diagnostic {
    Diagnostic::error(
        INVALID_PICTURE,
        format!("Invalid PICTURE clause '{picture}'"),
    )
    .with_span(span)
}

pub(crate) fn numeric_operand_required(span: Span) -> Diagnostic {
    Diagnostic::error(
        NUMERIC_OPERAND_REQUIRED,
        "Numeric operand required for arithmetic operation",
    )
    .with_span(span)
}

pub(crate) fn file_not_declared(name: &str, span: Span) -> Diagnostic {
    Diagnostic::error(FILE_NOT_DECLARED, format!("File '{name}' is not declared")).with_span(span)
}

pub(crate) fn type_mismatch(span: Span, left: DataType, right: DataType) -> Diagnostic {
    Diagnostic::error(
        TYPE_MISMATCH,
        format!("Cannot compare values of type {left:?} and {right:?}"),
    )
    .with_span(span)
}
