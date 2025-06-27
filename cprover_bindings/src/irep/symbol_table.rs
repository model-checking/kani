// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
use super::Symbol;
use crate::InternedString;
use std::collections::BTreeMap;

/// A direct implementation of the CBMC serilization format for symbol tables implemented in
/// <https://github.com/diffblue/cbmc/blob/develop/src/util/symbol_table.h>
#[derive(Debug, PartialEq)]
pub struct SymbolTable {
    pub symbol_table: BTreeMap<InternedString, Symbol>,
}

/// Constructors
impl Default for SymbolTable {
    fn default() -> Self {
        Self::new()
    }
}

impl SymbolTable {
    pub fn new() -> SymbolTable {
        SymbolTable { symbol_table: BTreeMap::new() }
    }
}

/// Setters
impl SymbolTable {
    pub fn insert(&mut self, symbol: Symbol) {
        self.symbol_table.insert(symbol.name, symbol);
    }
}
