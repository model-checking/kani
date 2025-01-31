// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! This crate implements irep serialization using serde Serializer.
use crate::InternedString;
use crate::irep::{Irep, IrepId, Symbol, SymbolTable};
use serde::Serialize;
use serde::ser::{SerializeMap, Serializer};

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

// A direct serialization for the goto SymbolTable (contrasting to the irep SymbolTable just above).
// This permits a "streaming optimization" where we reduce memory usage considerably by
// only holding the irep conversion of one symbol in memory at a time.
impl Serialize for crate::goto_program::SymbolTable {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut obj = serializer.serialize_map(None)?;
        obj.serialize_entry("symbolTable", &StreamingSymbols(self))?;
        obj.end()
    }
}
struct StreamingSymbols<'a>(&'a crate::goto_program::SymbolTable);
impl Serialize for StreamingSymbols<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mm = self.0.machine_model();
        let mut obj = serializer.serialize_map(None)?;
        for (k, v) in self.0.iter() {
            // We're only storing the to_irep in RAM for one symbol at a time
            obj.serialize_entry(k, &v.to_irep(mm))?;
        }
        obj.end()
    }
}

impl Serialize for InternedString {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.to_string().serialize(serializer)
    }
}

struct InternedStringVisitor;

impl<'de> serde::Deserialize<'de> for InternedString {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(InternedStringVisitor)
    }
}

impl serde::de::Visitor<'_> for InternedStringVisitor {
    type Value = InternedString;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("a String like thing")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(v.into())
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
    use serde_test::{Token, assert_ser_tokens};
    #[test]
    fn serialize_irep() {
        let irep = Irep::empty();
        assert_ser_tokens(&irep, &[
            Token::Map { len: None },
            Token::String("id"),
            Token::String("empty"),
            Token::MapEnd,
        ]);
    }

    #[test]
    fn serialize_sym_table() {
        let mut sym_table = SymbolTable::new();
        let symbol = Symbol {
            typ: Irep::empty(),
            value: Irep::empty(),
            location: Irep::empty(),
            name: "my_name".into(),
            module: "".into(),
            base_name: "".into(),
            pretty_name: "".into(),
            mode: "".into(),
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
        sym_table.insert(symbol);
        assert_ser_tokens(&sym_table, &[
            Token::Map { len: None },
            Token::String("symbolTable"),
            Token::Map { len: Some(1) },
            Token::String("my_name"),
            // symbol start
            Token::Map { len: None },
            // type irep
            Token::String("type"),
            Token::Map { len: None },
            Token::String("id"),
            Token::String("empty"),
            Token::MapEnd,
            // value irep
            Token::String("value"),
            Token::Map { len: None },
            Token::String("id"),
            Token::String("empty"),
            Token::MapEnd,
            // value locaton
            Token::String("location"),
            Token::Map { len: None },
            Token::String("id"),
            Token::String("empty"),
            Token::MapEnd,
            Token::String("name"),
            Token::String("my_name"),
            Token::String("module"),
            Token::String(""),
            Token::String("baseName"),
            Token::String(""),
            Token::String("prettyName"),
            Token::String(""),
            Token::String("mode"),
            Token::String(""),
            Token::String("isType"),
            Token::Bool(false),
            Token::String("isMacro"),
            Token::Bool(false),
            Token::String("isExported"),
            Token::Bool(false),
            Token::String("isInput"),
            Token::Bool(false),
            Token::String("isOutput"),
            Token::Bool(false),
            Token::String("isStateVar"),
            Token::Bool(false),
            Token::String("isProperty"),
            Token::Bool(false),
            Token::String("isStaticLifetime"),
            Token::Bool(false),
            Token::String("isThreadLocal"),
            Token::Bool(false),
            Token::String("isLvalue"),
            Token::Bool(false),
            Token::String("isFileLocal"),
            Token::Bool(false),
            Token::String("isExtern"),
            Token::Bool(false),
            Token::String("isVolatile"),
            Token::Bool(false),
            Token::String("isParameter"),
            Token::Bool(false),
            Token::String("isAuxiliary"),
            Token::Bool(false),
            Token::String("isWeak"),
            Token::Bool(false),
            Token::MapEnd,
            Token::MapEnd,
            Token::MapEnd,
        ]);
    }

    #[test]
    fn serialize_irep_sub() {
        let empty_irep = Irep::empty();
        let one_irep = Irep::one();
        let sub_irep = Irep::just_sub(vec![empty_irep.clone(), one_irep]);
        let top_irep = Irep::just_sub(vec![sub_irep, empty_irep]);
        assert_ser_tokens(&top_irep, &[
            // top_irep
            Token::Map { len: None },
            Token::String("id"),
            Token::String(""),
            Token::String("sub"),
            Token::Seq { len: Some(2) },
            // sub_irep
            Token::Map { len: None },
            Token::String("id"),
            Token::String(""),
            Token::String("sub"),
            Token::Seq { len: Some(2) },
            // empty_irep
            Token::Map { len: None },
            Token::String("id"),
            Token::String("empty"),
            Token::MapEnd,
            // one_irep
            Token::Map { len: None },
            Token::String("id"),
            Token::String("1"),
            Token::MapEnd,
            Token::SeqEnd,
            Token::MapEnd,
            // empty_irep
            Token::Map { len: None },
            Token::String("id"),
            Token::String("empty"),
            Token::MapEnd,
            Token::SeqEnd,
            Token::MapEnd,
        ]);
    }
}
