// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
use super::super::{env, MachineModel};
use super::{BuiltinFn, DatatypeComponent, Stmt, Symbol, Type};
use std::collections::BTreeMap;
/// This is a typesafe implementation of the CBMC symbol table, based on the CBMC code at:
/// https://github.com/diffblue/cbmc/blob/develop/src/util/symbol_table.h
/// Since the field is kept private, with only immutable references handed out, elements can only
#[derive(Debug)]
pub struct SymbolTable {
    symbol_table: BTreeMap<String, Symbol>,
    machine_model: MachineModel,
}

/// Constructors
impl SymbolTable {
    pub fn new(machine_model: MachineModel) -> SymbolTable {
        let mut symtab = SymbolTable { machine_model, symbol_table: BTreeMap::new() };
        env::machine_model_symbols(symtab.machine_model())
            .into_iter()
            .for_each(|s| symtab.insert(s));
        env::additional_env_symbols().into_iter().for_each(|s| symtab.insert(s));
        BuiltinFn::list_all().iter().for_each(|x| symtab.insert(x.as_symbol()));
        symtab
    }
}

/// Setters
impl SymbolTable {
    /// Ensures that the `name` appears in the Symbol table.
    /// If it doesn't, inserts it using `f`.
    pub fn ensure<F: FnOnce(&Self, &str) -> Symbol>(&mut self, name: &str, f: F) -> &Symbol {
        if !self.contains(name) {
            let sym = f(self, name);
            assert_eq!(sym.name, name);
            self.insert(sym);
        }
        self.lookup(name).unwrap()
    }

    /// Insert the element into the table. Errors if element already exists.
    pub fn insert(&mut self, symbol: Symbol) {
        assert!(
            self.lookup(&symbol.name).is_none(),
            "Tried to insert symbol which already existed\n\t: {:?}\n\t",
            &symbol
        );
        self.symbol_table.insert(symbol.name.to_string(), symbol);
    }

    /// Validates the previous value of the symbol using the validator function, then replaces it.
    /// Useful to replace declarations with the actual definition.
    pub fn replace<F: FnOnce(Option<&Symbol>) -> bool>(
        &mut self,
        checker_fn: F,
        new_value: Symbol,
    ) {
        let old_value = self.lookup(&new_value.name);
        assert!(checker_fn(old_value), "{:?}||{:?}", old_value, new_value);
        self.symbol_table.insert(new_value.name.to_string(), new_value);
    }

    /// Replace an incomplete struct or union with a complete struct or union
    pub fn replace_with_completion(&mut self, new_symbol: Symbol) {
        self.replace(|old_symbol| new_symbol.completes(old_symbol), new_symbol.clone())
    }

    pub fn update_fn_declaration_with_definition(&mut self, name: &str, body: Stmt) {
        self.symbol_table.get_mut(name).unwrap().update_fn_declaration_with_definition(body);
    }

    pub fn remove(&mut self, name: &str) -> Option<Symbol> {
        self.symbol_table.remove(name)
    }
}

/// Getters
impl SymbolTable {
    pub fn contains(&self, name: &str) -> bool {
        self.symbol_table.contains_key(name)
    }

    pub fn iter(&self) -> std::collections::btree_map::Iter<'_, String, Symbol> {
        self.symbol_table.iter()
    }

    pub fn lookup(&self, name: &str) -> Option<&Symbol> {
        self.symbol_table.get(name)
    }

    /// If aggr_name.field_name exists in the symbol table, return Some(field_type),
    /// otherwise, return none.
    pub fn lookup_field_type(&self, aggr_name: &str, field_name: &str) -> Option<&Type> {
        self.lookup(aggr_name)
            .and_then(|x| x.typ.components())
            .and_then(|fields| fields.iter().find(|&field| field.name() == field_name))
            .and_then(|field| field.field_typ())
    }

    /// If aggr_name.field_name exists in the symbol table, return Some(field_type),
    /// otherwise, return none.
    pub fn lookup_field_type_in_type(&self, base_type: &Type, field_name: &str) -> Option<&Type> {
        base_type.type_name().and_then(|aggr_name| self.lookup_field_type(&aggr_name, field_name))
    }

    pub fn lookup_fields_in_type(&self, base_type: &Type) -> Option<&Vec<DatatypeComponent>> {
        base_type
            .type_name()
            .and_then(|aggr_name| self.lookup(&aggr_name))
            .and_then(|x| x.typ.components())
    }

    pub fn machine_model(&self) -> &MachineModel {
        &self.machine_model
    }
}
