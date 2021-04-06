// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
use super::{Irep, IrepId, Symbol, SymbolTable};
use crate::btree_map;
use rustc_serialize::json::*;

impl ToJson for Irep {
    fn to_json(&self) -> Json {
        let mut output = btree_map![("id".to_string(), self.id.to_string().to_json())];
        if !self.sub.is_empty() {
            output.insert("sub".to_string(), self.sub.to_json());
        }
        if !self.named_sub.is_empty() {
            output.insert("namedSub".to_string(), self.named_sub.to_json());
        }
        Json::Object(output)
    }
}

impl ToJson for Symbol {
    fn to_json(&self) -> Json {
        let output = btree_map![
            (IrepId::Type.to_string(), self.typ.to_json()),
            (IrepId::Value.to_string(), self.value.to_json()),
            ("location".to_string(), self.location.to_json()),
            (IrepId::Name.to_string(), self.name.to_json()),
            (IrepId::Module.to_string(), self.module.to_json()),
            ("baseName".to_string(), self.base_name.to_json()),
            ("prettyName".to_string(), self.pretty_name.to_json()),
            (IrepId::Mode.to_string(), self.mode.to_json()),
            ("isType".to_string(), self.is_type.to_json()),
            ("isMacro".to_string(), self.is_macro.to_json()),
            ("isExported".to_string(), self.is_exported.to_json()),
            ("isInput".to_string(), self.is_input.to_json()),
            ("isOutput".to_string(), self.is_output.to_json()),
            ("isStateVar".to_string(), self.is_state_var.to_json()),
            ("isProperty".to_string(), self.is_property.to_json()),
            ("isStaticLifetime".to_string(), self.is_static_lifetime.to_json()),
            ("isThreadLocal".to_string(), self.is_thread_local.to_json()),
            ("isLvalue".to_string(), self.is_lvalue.to_json()),
            ("isFileLocal".to_string(), self.is_file_local.to_json()),
            ("isExtern".to_string(), self.is_extern.to_json()),
            ("isVolatile".to_string(), self.is_volatile.to_json()),
            ("isParameter".to_string(), self.is_parameter.to_json()),
            ("isAuxiliary".to_string(), self.is_auxiliary.to_json()),
            ("isWeak".to_string(), self.is_weak.to_json()),
        ];

        Json::Object(output)
    }
}

impl ToJson for SymbolTable {
    fn to_json(&self) -> Json {
        let output = btree_map![("symbolTable".to_string(), self.symbol_table.to_json())];
        Json::Object(output)
    }
}
