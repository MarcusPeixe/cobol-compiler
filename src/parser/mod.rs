mod ast;
mod builder;
mod errors;

pub use ast::*;
pub use builder::Parser;
pub use errors::ParseError;

use crate::diagnostics::Diagnostic;
use crate::lexer::Token;

pub fn parse(tokens: Vec<Token>) -> (Program, Vec<Diagnostic>) {
    let parser = Parser::new(tokens);
    parser.parse()
}

#[cfg(test)]
mod tests;
