use super::{
    DatatypeComponent, Expr, Location, Stmt, Symbol, SymbolTable, SymbolValues, Transformer, Type,
};
use std::collections::{BTreeMap, HashSet};

pub struct IdentityTransformer {
    new_symbol_table: SymbolTable,
}

impl IdentityTransformer {
    pub fn transform(original_symbol_table: &SymbolTable) -> SymbolTable {
        let new_symbol_table = SymbolTable::new(original_symbol_table.machine_model().clone());
        IdentityTransformer { new_symbol_table }.transform_symbol_table(original_symbol_table)
    }
}

impl Transformer for IdentityTransformer {
    fn symbol_table(&self) -> &SymbolTable {
        &self.new_symbol_table
    }

    fn mut_symbol_table(&mut self) -> &mut SymbolTable {
        &mut self.new_symbol_table
    }

    fn extract_symbol_table(self) -> SymbolTable {
        self.new_symbol_table
    }
}
