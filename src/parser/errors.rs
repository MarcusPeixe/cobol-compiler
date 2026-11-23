use thiserror::Error;

use crate::lexer::TokenKind;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ParseError {
    #[error("unexpected token: expected {expected}, found {found}")]
    UnexpectedToken { expected: String, found: String },

    #[error("unexpected end of input")]
    UnexpectedEof,

    #[error("missing PROGRAM-ID")]
    MissingProgramId,

    #[error("invalid level number")]
    InvalidLevelNumber,

    #[error("missing period after {context}")]
    MissingPeriod { context: String },

    #[error("invalid data item")]
    InvalidDataItem,

    #[error("invalid statement")]
    InvalidStatement,
}

impl ParseError {
    pub fn unexpected_token(expected: &str, found: &TokenKind) -> Self {
        Self::UnexpectedToken {
            expected: expected.to_string(),
            found: format!("{:?}", found),
        }
    }
}
