//! Phase 1: lexical analysis.

mod scanner;
mod token;

pub use scanner::lex;
pub use token::{Keyword, Token, TokenKind};
