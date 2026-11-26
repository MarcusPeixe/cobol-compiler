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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SymbolKind {
    DataItem,
    IndexName,
    Paragraph,
    File,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Symbol {
    pub name: String,
    /// Containing data names, nearest first.
    pub qualifiers: Vec<String>,
    pub span: Span,
    pub data_type: DataType,
    pub kind: SymbolKind,
    pub level: u8,
    pub scope: Scope,
}

#[derive(Debug, Default, Clone)]
pub struct SymbolTable {
    symbols: HashMap<(usize, String), Vec<Symbol>>,
}

impl SymbolTable {
    pub fn declare(&mut self, symbol: Symbol) -> bool {
        let key = (symbol.scope.id(), symbol.name.clone());
        let symbols = self.symbols.entry(key).or_default();
        let duplicate = symbols.iter().any(|existing| {
            existing.kind != SymbolKind::DataItem
                || symbol.kind != SymbolKind::DataItem
                || existing.qualifiers == symbol.qualifiers
        });
        if duplicate {
            return false;
        }
        symbols.push(symbol);
        true
    }

    pub fn lookup(&self, name: &str, scope: Scope) -> Option<&Symbol> {
        let [symbol] = self.lookup_all(name, scope) else {
            return None;
        };
        Some(symbol)
    }

    pub fn lookup_all(&self, name: &str, scope: Scope) -> &[Symbol] {
        self.symbols
            .get(&(scope.id(), name.to_ascii_uppercase()))
            .or_else(|| {
                if scope != Scope::Global {
                    self.symbols.get(&(0, name.to_ascii_uppercase()))
                } else {
                    None
                }
            })
            .map_or(&[], Vec::as_slice)
    }

    pub fn all_symbols(&self) -> impl Iterator<Item = &Symbol> {
        self.symbols.values().flatten()
    }
}

pub fn format_symbol_table(symbol_table: &SymbolTable) -> String {
    let mut symbols: Vec<_> = symbol_table.all_symbols().collect();
    symbols.sort_by(|left, right| {
        left.name
            .cmp(&right.name)
            .then_with(|| left.qualifiers.cmp(&right.qualifiers))
            .then_with(|| left.kind.cmp(&right.kind))
    });

    let mut output = String::new();
    for symbol in symbols {
        output.push_str("  ");
        output.push_str(&symbol.name);
        if !symbol.qualifiers.is_empty() {
            output.push_str(" OF ");
            output.push_str(&symbol.qualifiers.join(" OF "));
        }
        output.push_str(": ");
        match symbol.kind {
            SymbolKind::DataItem => output.push_str(&format!(
                "DataItem ({:?}, level {})",
                symbol.data_type, symbol.level
            )),
            SymbolKind::IndexName => {
                output.push_str(&format!("IndexName ({:?})", symbol.data_type));
            }
            SymbolKind::Paragraph => output.push_str("Paragraph"),
            SymbolKind::File => output.push_str("File"),
        }
        output.push('\n');
    }
    output
}

#[cfg(test)]
mod tests {
    use super::{Scope, Symbol, SymbolKind, SymbolTable, format_symbol_table};
    use crate::semantic::DataType;
    use crate::source::{FileId, Span};

    fn symbol(name: &str, scope: Scope) -> Symbol {
        Symbol {
            name: name.to_string(),
            qualifiers: Vec::new(),
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

    #[test]
    fn allows_same_data_name_under_distinct_qualifiers() {
        let mut table = SymbolTable::default();
        let mut first = symbol("ITEM", Scope::Global);
        first.qualifiers.push("GROUP-A".to_string());
        let mut second = symbol("ITEM", Scope::Global);
        second.qualifiers.push("GROUP-B".to_string());

        assert!(table.declare(first));
        assert!(table.declare(second));
        assert_eq!(table.lookup_all("ITEM", Scope::Global).len(), 2);
        assert!(table.lookup("ITEM", Scope::Global).is_none());
    }

    #[test]
    fn rejects_same_data_name_under_same_qualifiers() {
        let mut table = SymbolTable::default();
        let mut first = symbol("ITEM", Scope::Global);
        first.qualifiers.push("GROUP-A".to_string());
        let mut duplicate = first.clone();

        assert!(table.declare(first));
        assert!(!table.declare(duplicate.clone()));

        duplicate.qualifiers.clear();
        assert!(table.declare(duplicate));
    }

    #[test]
    fn formats_symbol_kinds_and_qualifiers() {
        let mut table = SymbolTable::default();
        let mut item = symbol("ITEM", Scope::Global);
        item.qualifiers.push("GROUP-A".to_string());
        let mut index = symbol("IDX", Scope::Global);
        index.kind = SymbolKind::IndexName;
        index.level = 0;
        let mut paragraph = symbol("MAIN", Scope::Global);
        paragraph.kind = SymbolKind::Paragraph;
        paragraph.level = 0;

        table.declare(item);
        table.declare(index);
        table.declare(paragraph);

        assert_eq!(
            format_symbol_table(&table),
            concat!(
                "  IDX: IndexName (Numeric)\n",
                "  ITEM OF GROUP-A: DataItem (Numeric, level 1)\n",
                "  MAIN: Paragraph\n",
            )
        );
    }
}
