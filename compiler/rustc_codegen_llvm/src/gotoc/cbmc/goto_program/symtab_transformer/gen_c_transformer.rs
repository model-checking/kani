// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

use super::super::{
    CIntType, DatatypeComponent, Expr, Location, Parameter, Stmt, StmtBody, Symbol, SymbolTable,
    SymbolValues, Type,
};
use super::Transformer;
use std::collections::{BTreeMap, HashMap, HashSet};

/// Converts an arbitrary identifier into a valid C identifier.
fn normalize_identifier(name: &str) -> String {
    // Convert non-(alphanumeric + underscore) characters to underscore
    let valid_chars = name.replace(|ch: char| !(ch.is_alphanumeric() || ch == '_'), "_");

    /// If the first character is a number, prefix with underscore
    let new_name = match valid_chars.chars().next() {
        Some(first) => {
            if first.is_numeric() {
                let mut name = "_".to_string();
                name.push_str(&valid_chars);
                name
            } else {
                valid_chars
            }
        }
        None => "_".to_string(),
    };

    /// Replace reserved names with alternatives
    let mut illegal_names: HashMap<_, _> = [("case", "case_"), ("main", "main_")]
        .iter()
        .map(|(key, value)| (key.to_string(), value.to_string()))
        .collect();
    illegal_names.remove(&new_name).unwrap_or(new_name)
}

/// Struct for performing the gen-c transformation on a symbol table.
pub struct GenCTransformer {
    new_symbol_table: SymbolTable,
}

impl GenCTransformer {
    /// Transform all identifiers in the symbol table to be valid C identifiers;
    /// perform other clean-up operations to make valid C code.
    pub fn transform(original_symbol_table: &SymbolTable) -> SymbolTable {
        let new_symbol_table = SymbolTable::new(original_symbol_table.machine_model().clone());
        GenCTransformer { new_symbol_table }.transform_symbol_table(original_symbol_table)
    }
}

impl Transformer for GenCTransformer {
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

    /// Normalize parameter identifier.
    fn transform_parameter(&self, parameter: &Parameter) -> Parameter {
        Type::parameter(
            parameter.identifier().map(|name| normalize_identifier(name)),
            parameter.base_name().map(|name| normalize_identifier(name)),
            self.transform_type(parameter.typ()),
        )
    }

    /// Normalize field names.
    fn transform_member_expr(&self, _typ: &Type, lhs: &Expr, field: &str) -> Expr {
        let transformed_lhs = self.transform_expr(lhs);
        transformed_lhs.member(&normalize_identifier(field), self.symbol_table())
    }

    /// Normalize name in identifier expression.
    fn transform_symbol_expr(&self, typ: &Type, identifier: &str) -> Expr {
        let transformed_typ = self.transform_type(typ);
        Expr::symbol_expression(normalize_identifier(identifier), transformed_typ)
    }

    /// Normalize union field names.
    fn transform_union_expr(&self, typ: &Type, value: &Expr, field: &str) -> Expr {
        let transformed_typ = self.transform_type(typ);
        let transformed_value = self.transform_expr(value);
        Expr::union_expr(
            transformed_typ,
            &normalize_identifier(field),
            transformed_value,
            self.symbol_table(),
        )
    }

    /// Normalize incomplete struct tag name.
    fn transform_incomplete_struct_type(&self, tag: &str) -> Type {
        Type::incomplete_struct(&normalize_identifier(tag))
    }

    /// Normalize incomplete union tag name.
    fn transform_incomplete_union_type(&self, tag: &str) -> Type {
        Type::incomplete_union(&normalize_identifier(tag))
    }

    /// Normalize union/struct component name.
    fn transform_datatype_component(&self, component: &DatatypeComponent) -> DatatypeComponent {
        match component {
            DatatypeComponent::Field { name, typ } => DatatypeComponent::Field {
                name: normalize_identifier(name),
                typ: self.transform_type(typ),
            },
            DatatypeComponent::Padding { name, bits } => {
                DatatypeComponent::Padding { name: normalize_identifier(name), bits: *bits }
            }
        }
    }

    /// Normalize struct type name.
    fn transform_struct_type(&self, tag: &str, components: &[DatatypeComponent]) -> Type {
        let transformed_components = components
            .iter()
            .map(|component| self.transform_datatype_component(component))
            .collect();
        Type::struct_type(&normalize_identifier(tag), transformed_components)
    }

    /// Normalize struct tag name.
    fn transform_struct_tag_type(&self, tag: &str) -> Type {
        Type::struct_tag_raw(&normalize_identifier(tag))
    }

    /// Normalize union type name.
    fn transform_union_type(&self, tag: &str, components: &[DatatypeComponent]) -> Type {
        let transformed_components = components
            .iter()
            .map(|component| self.transform_datatype_component(component))
            .collect();
        Type::union_type(&normalize_identifier(tag), transformed_components)
    }

    /// Normalize union tag name.
    fn transform_union_tag_type(&self, tag: &str) -> Type {
        Type::union_tag_raw(&normalize_identifier(tag))
    }

    /// Normalize goto label name.
    fn transform_goto_stmt(&self, label: &str) -> Stmt {
        Stmt::goto(normalize_identifier(label), Location::none())
    }

    /// Normalize label name.
    fn transform_label_stmt(&self, label: &str, body: &Stmt) -> Stmt {
        let transformed_body = self.transform_stmt(body);
        transformed_body.with_label(normalize_identifier(label))
    }

    /// Normalize symbol names.
    fn transform_symbol(&self, symbol: &Symbol) -> Symbol {
        let new_typ = self.transform_type(&symbol.typ);
        let new_value = self.transform_value(&symbol.value);
        let mut new_symbol = symbol.clone();
        new_symbol.value = new_value;
        new_symbol.typ = new_typ;
        new_symbol.name = normalize_identifier(&new_symbol.name);
        new_symbol.base_name = new_symbol.base_name.map(|name| normalize_identifier(&name));
        new_symbol.pretty_name = new_symbol.pretty_name.map(|name| normalize_identifier(&name));
        new_symbol
    }

    /// Perform cleanup necessary to make C code valid.
    fn postprocess(&mut self) {
        // Remove predefined macros that cause issues being re-declared.
        let memcpy = self.mut_symbol_table().remove("memcpy");
        assert!(memcpy.is_some());
        let memmove = self.mut_symbol_table().remove("memmove");
        assert!(memmove.is_some());

        // Redefine main function to return an `int`.
        // Moves `main` to `main_`, and create `main` to call now `main_`.
        let old_main = self.symbol_table().lookup("main_");
        if let Some(old_main) = old_main {
            let new_main = Symbol::function(
                "main",
                Type::code(Vec::new(), Type::CInteger(CIntType::Int)),
                Some(Stmt::block(
                    vec![
                        Stmt::code_expression(
                            Expr::symbol_expression("main_".to_string(), old_main.typ.clone())
                                .call(Vec::new()),
                            Location::none(),
                        ),
                        Stmt::ret(
                            Some(Expr::int_constant(0, Type::CInteger(CIntType::Int))),
                            Location::none(),
                        ),
                    ],
                    Location::none(),
                )),
                Some("main".to_string()),
                Location::none(),
            );
            self.mut_symbol_table().insert(new_main);
        }
    }
}
