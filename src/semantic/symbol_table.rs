use std::collections::HashMap;

use crate::source::Span;

use super::DataType;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Scope {
    Global,
    Procedure(usize),
}

impl Scope {
    fn id(self) -> usize {
        match self {
            Scope::Global => 0,
            Scope::Procedure(id) => id,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolKind {
    DataItem,
    Paragraph,
    File,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Symbol {
    pub name: String,
    pub span: Span,
    pub data_type: DataType,
    pub kind: SymbolKind,
    pub level: u8,
    pub scope: Scope,
}

#[derive(Debug, Default, Clone)]
pub struct SymbolTable {
    symbols: HashMap<(usize, String), Symbol>,
}

impl SymbolTable {
    pub fn declare(&mut self, symbol: Symbol) -> bool {
        let key = (symbol.scope.id(), symbol.name.clone());
        if self.symbols.contains_key(&key) {
            return false;
        }
        self.symbols.insert(key, symbol);
        true
    }

    pub fn lookup(&self, name: &str, scope: Scope) -> Option<&Symbol> {
        self.symbols
            .get(&(scope.id(), name.to_ascii_uppercase()))
            .or_else(|| {
                if scope != Scope::Global {
                    self.symbols.get(&(0, name.to_ascii_uppercase()))
                } else {
                    None
                }
            })
    }

    pub fn all_symbols(&self) -> impl Iterator<Item = &Symbol> {
        self.symbols.values()
    }
}

#[cfg(test)]
mod tests {
    use super::{Scope, Symbol, SymbolKind, SymbolTable};
    use crate::semantic::DataType;
    use crate::source::{FileId, Span};

    fn symbol(name: &str, scope: Scope) -> Symbol {
        Symbol {
            name: name.to_string(),
            span: Span::new(FileId(0), 0, 1),
            data_type: DataType::Numeric,
            kind: SymbolKind::DataItem,
            level: 1,
            scope,
        }
    }

    #[test]
    fn declares_and_looks_up_symbols_case_insensitively() {
        let mut table = SymbolTable::default();
        assert!(table.declare(symbol("COUNT", Scope::Global)));
        assert!(table.lookup("count", Scope::Global).is_some());
        assert_eq!(table.all_symbols().count(), 1);
    }

    #[test]
    fn rejects_duplicate_symbols_in_a_scope() {
        let mut table = SymbolTable::default();
        assert!(table.declare(symbol("COUNT", Scope::Global)));
        assert!(!table.declare(symbol("COUNT", Scope::Global)));
    }

    #[test]
    fn procedure_scope_can_see_global_symbols() {
        let mut table = SymbolTable::default();
        table.declare(symbol("COUNT", Scope::Global));
        assert!(table.lookup("COUNT", Scope::Procedure(1)).is_some());
    }
}
