// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
use super::Symbol;
use std::collections::BTreeMap;

/// A direct implementation of the CBMC serilization format for symbol tables implemented in
/// https://github.com/diffblue/cbmc/blob/develop/src/util/symbol_table.h
#[derive(Debug)]
pub struct SymbolTable {
    pub symbol_table: BTreeMap<String, Symbol>,
}

/// Constructors
impl SymbolTable {
    pub fn new() -> SymbolTable {
        SymbolTable { symbol_table: BTreeMap::new() }
    }
}

/// Setters
impl SymbolTable {
    pub fn insert(&mut self, symbol: Symbol) {
        self.symbol_table.insert(symbol.name.clone(), symbol);
    }
}
