// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! This crate implements irep serialization using serde Serializer.
use crate::irep::{Irep, IrepId, Symbol, SymbolTable};
use serde::ser::{SerializeMap, Serializer};
use serde::Serialize;

impl Serialize for Irep {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut obj = serializer.serialize_map(None)?;
        obj.serialize_entry("id", &self.id)?;
        if !self.sub.is_empty() {
            obj.serialize_entry("sub", &self.sub)?;
        }
        if !self.named_sub.is_empty() {
            obj.serialize_entry("namedSub", &self.named_sub)?;
        }
        obj.end()
    }
}

impl Serialize for IrepId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.to_string().serialize(serializer)
    }
}

impl Serialize for SymbolTable {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut obj = serializer.serialize_map(None)?;
        obj.serialize_entry("symbolTable", &self.symbol_table)?;
        obj.end()
    }
}

impl Serialize for Symbol {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut obj = serializer.serialize_map(None)?;
        obj.serialize_entry(&IrepId::Type.to_string(), &self.typ)?;
        obj.serialize_entry(&IrepId::Value.to_string(), &self.value)?;
        obj.serialize_entry("location", &self.location)?;
        obj.serialize_entry(&IrepId::Name.to_string(), &self.name)?;
        obj.serialize_entry(&IrepId::Module.to_string(), &self.module)?;
        obj.serialize_entry("baseName", &self.base_name)?;
        obj.serialize_entry("prettyName", &self.pretty_name)?;
        obj.serialize_entry(&IrepId::Mode.to_string(), &self.mode)?;
        obj.serialize_entry("isType", &self.is_type)?;
        obj.serialize_entry("isMacro", &self.is_macro)?;
        obj.serialize_entry("isExported", &self.is_exported)?;
        obj.serialize_entry("isInput", &self.is_input)?;
        obj.serialize_entry("isOutput", &self.is_output)?;
        obj.serialize_entry("isStateVar", &self.is_state_var)?;
        obj.serialize_entry("isProperty", &self.is_property)?;
        obj.serialize_entry("isStaticLifetime", &self.is_static_lifetime)?;
        obj.serialize_entry("isThreadLocal", &self.is_thread_local)?;
        obj.serialize_entry("isLvalue", &self.is_lvalue)?;
        obj.serialize_entry("isFileLocal", &self.is_file_local)?;
        obj.serialize_entry("isExtern", &self.is_extern)?;
        obj.serialize_entry("isVolatile", &self.is_volatile)?;
        obj.serialize_entry("isParameter", &self.is_parameter)?;
        obj.serialize_entry("isAuxiliary", &self.is_auxiliary)?;
        obj.serialize_entry("isWeak", &self.is_weak)?;

        obj.end()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn serialize_irep() {
        let irep = Irep::empty();
        let json_str = serde_json::to_string(&irep);
        assert!(json_str.unwrap().contains(&IrepId::Empty.to_string()));
    }

    #[test]
    fn serialize_sym_table() {
        let mut sym_table = SymbolTable::new();
        let symbol = Symbol {
            typ: Irep::empty(),
            value: Irep::empty(),
            location: Irep::empty(),
            name: String::from("name"),
            module: String::new(),
            base_name: String::new(),
            pretty_name: String::new(),
            mode: String::new(),
            is_type: false,
            is_macro: false,
            is_exported: false,
            is_input: false,
            is_output: false,
            is_state_var: false,
            is_property: false,

            // ansi-C properties
            is_static_lifetime: false,
            is_thread_local: false,
            is_lvalue: false,
            is_file_local: false,
            is_extern: false,
            is_volatile: false,
            is_parameter: false,
            is_auxiliary: false,
            is_weak: false,
        };
        sym_table.insert(symbol.clone());
        let json_str = serde_json::to_string(&sym_table);
        assert!(json_str.unwrap().contains(&symbol.name));
    }
}
