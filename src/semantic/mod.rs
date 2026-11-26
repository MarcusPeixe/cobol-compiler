mod analysis;
mod errors;
mod layout;
mod symbol_table;
mod type_system;

pub use analysis::{AnalyzedProgram, SemanticAnalyzer, analyze};
pub use layout::{
    FileLayout, ItemLayout, PictureInfo, StorageLayout, compute_layout, format_layout, picture_info,
};
pub use symbol_table::{Scope, Symbol, SymbolKind, SymbolTable, format_symbol_table};
pub use type_system::DataType;

#[cfg(test)]
mod tests;
