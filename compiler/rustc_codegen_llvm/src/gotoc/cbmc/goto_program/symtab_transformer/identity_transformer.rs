// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

use super::super::{
    DatatypeComponent, Expr, Location, Stmt, Symbol, SymbolTable, SymbolValues, Type,
};
use super::Transformer;
use std::collections::{BTreeMap, HashSet};

/// Struct for performing the identity transformation on a symbol table.
/// Mainly used as a demo/for testing.
pub struct IdentityTransformer {
    new_symbol_table: SymbolTable,
}

impl IdentityTransformer {
    /// Perform an identity transformation on the given symbol table.
    pub fn transform(original_symbol_table: &SymbolTable) -> SymbolTable {
        let new_symbol_table = SymbolTable::new(original_symbol_table.machine_model().clone());
        IdentityTransformer { new_symbol_table }.transform_symbol_table(original_symbol_table)
    }
}

impl Transformer for IdentityTransformer {
    /// Get reference to symbol table.
    fn symbol_table(&self) -> &SymbolTable {
        &self.new_symbol_table
    }

    /// Get mutable reference to symbol table.
    fn mut_symbol_table(&mut self) -> &mut SymbolTable {
        &mut self.new_symbol_table
    }

    /// Get owned symbol table.
    fn extract_symbol_table(self) -> SymbolTable {
        self.new_symbol_table
    }
}
