mod analysis;
mod errors;
mod symbol_table;
mod type_system;

pub use analysis::{AnalyzedProgram, SemanticAnalyzer, analyze};
pub use symbol_table::{Scope, Symbol, SymbolKind, SymbolTable};
pub use type_system::DataType;

#[cfg(test)]
mod tests;
