use crate::diagnostics::Diagnostic;
use crate::semantic::DataType;
use crate::source::Span;

pub(crate) const IDENTIFIER_DUPLICATE: &str = "cobol::semantic::identifier_duplicate";
pub(crate) const MOVE_INCOMPATIBLE: &str = "cobol::semantic::move_incompatible";
pub(crate) const UNDEFINED_IDENTIFIER: &str = "cobol::semantic::undefined_identifier";
pub(crate) const AMBIGUOUS_IDENTIFIER: &str = "cobol::semantic::ambiguous_identifier";
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

pub(crate) fn ambiguous_identifier(name: &str, span: Span) -> Diagnostic {
    Diagnostic::error(
        AMBIGUOUS_IDENTIFIER,
        format!("Identifier '{name}' is ambiguous; qualify it with OF or IN"),
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

pub(crate) const SUBSCRIPT_ON_NON_TABLE: &str = "cobol::semantic::subscript_on_non_table";
pub(crate) const SUBSCRIPT_COUNT: &str = "cobol::semantic::subscript_count";
pub(crate) const INVALID_OCCURS: &str = "cobol::semantic::invalid_occurs";
pub(crate) const VALUE_TYPE_MISMATCH: &str = "cobol::semantic::value_type_mismatch";
pub(crate) const VALUE_TOO_LONG: &str = "cobol::semantic::value_too_long";
pub(crate) const GROUP_WITH_PICTURE: &str = "cobol::semantic::group_with_picture";
pub(crate) const MISSING_PICTURE: &str = "cobol::semantic::missing_picture";
pub(crate) const NOT_A_RECORD: &str = "cobol::semantic::not_a_record";
pub(crate) const SEARCH_REQUIRES_TABLE: &str = "cobol::semantic::search_requires_table";

pub(crate) fn subscript_on_non_table(name: &str, span: Span) -> Diagnostic {
    Diagnostic::error(
        SUBSCRIPT_ON_NON_TABLE,
        format!("'{name}' is subscripted but is not part of a table (no OCCURS)"),
    )
    .with_span(span)
}

pub(crate) fn subscript_count(name: &str, expected: usize, found: usize, span: Span) -> Diagnostic {
    Diagnostic::error(
        SUBSCRIPT_COUNT,
        format!("'{name}' needs {expected} subscript(s) but {found} given"),
    )
    .with_span(span)
}

pub(crate) fn invalid_occurs(name: &str, span: Span) -> Diagnostic {
    Diagnostic::error(
        INVALID_OCCURS,
        format!("OCCURS count of '{name}' must be a positive integer"),
    )
    .with_span(span)
}

pub(crate) fn value_type_mismatch(name: &str, span: Span) -> Diagnostic {
    Diagnostic::error(
        VALUE_TYPE_MISMATCH,
        format!("VALUE of '{name}' does not match its PICTURE category"),
    )
    .with_span(span)
}

pub(crate) fn value_too_long(name: &str, span: Span) -> Diagnostic {
    Diagnostic::error(
        VALUE_TOO_LONG,
        format!("VALUE of '{name}' does not fit its PICTURE"),
    )
    .with_span(span)
}

pub(crate) fn group_with_picture(name: &str, span: Span) -> Diagnostic {
    Diagnostic::error(
        GROUP_WITH_PICTURE,
        format!("Group item '{name}' cannot have a PICTURE clause"),
    )
    .with_span(span)
}

pub(crate) fn missing_picture(name: &str, span: Span) -> Diagnostic {
    Diagnostic::error(
        MISSING_PICTURE,
        format!("Elementary item '{name}' requires a PICTURE clause"),
    )
    .with_span(span)
}

pub(crate) fn not_a_record(name: &str, span: Span) -> Diagnostic {
    Diagnostic::error(
        NOT_A_RECORD,
        format!("'{name}' is not a record of a file declared in the FILE SECTION"),
    )
    .with_span(span)
}

pub(crate) fn search_requires_table(name: &str, span: Span) -> Diagnostic {
    Diagnostic::error(
        SEARCH_REQUIRES_TABLE,
        format!("SEARCH requires a table item with OCCURS, but '{name}' has none"),
    )
    .with_span(span)
}
