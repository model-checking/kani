// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

use super::super::super::{
    DatatypeComponent, Expr, Location, Parameter, Stmt, Symbol, SymbolTable, Type,
};
use super::super::Transformer;
use rustc_data_structures::fx::{FxHashMap, FxHashSet};

/// Struct for replacing names with valid C identifiers for --gen-c-runnable.
pub struct NameTransformer {
    new_symbol_table: SymbolTable,
    mapped_names: FxHashMap<String, String>,
    used_names: FxHashSet<String>,
}

impl NameTransformer {
    /// Transform all identifiers in the symbol table to be valid C identifiers.
    pub fn transform(original_symbol_table: &SymbolTable) -> SymbolTable {
        let new_symbol_table = SymbolTable::new(original_symbol_table.machine_model().clone());
        NameTransformer {
            new_symbol_table,
            mapped_names: FxHashMap::default(),
            used_names: FxHashSet::default(),
        }
        .transform_symbol_table(original_symbol_table)
    }

    /// Converts an arbitrary identifier into a valid C identifier.
    fn normalize_identifier(&mut self, orig_name: &str) -> String {
        assert!(!orig_name.is_empty(), "Received empty identifier.");

        // If name already encountered, return same result
        match self.mapped_names.get(orig_name).cloned() {
            Some(result) => return result.clone(),
            None => (),
        }

        // Don't tranform the `tag-` prefix of identifiers
        let (prefix, name) = if orig_name.starts_with("tag-") {
            (&orig_name[..4], &orig_name[4..])
        } else {
            ("", orig_name)
        };

        // Convert non-(alphanumeric + underscore) characters to underscore
        let valid_chars =
            name.replace(|ch: char| !(ch.is_alphanumeric() || ch == '_' || ch == '$'), "_");

        // If the first character is a number, prefix with underscore
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

        // Replace reserved names with alternatives
        let mut illegal_names: FxHashMap<_, _> = [("case", "case_"), ("default", "_default")]
            .iter()
            .map(|(key, value)| (key.to_string(), value.to_string()))
            .collect();
        let result = illegal_names.remove(&new_name).unwrap_or(new_name);

        // Ensure result has not been used before
        let result = if self.used_names.contains(&result) {
            let mut suffix = 0;
            loop {
                let result = format!("{}_{}", result, suffix);
                if !self.used_names.contains(&result) {
                    break result;
                }
                suffix += 1;
            }
        } else {
            result
        };

        // Add `tag-` back in if it was present
        let result = {
            let mut prefix = prefix.to_string();
            prefix.push_str(&result);
            prefix
        };

        // Remember result and return
        self.used_names.insert(result.clone());
        self.mapped_names.insert(orig_name.to_string(), result);
        self.mapped_names.get(orig_name).unwrap().clone()
    }
}

impl Transformer for NameTransformer {
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
    fn transform_type_parameter(&mut self, parameter: &Parameter) -> Parameter {
        Type::parameter(
            parameter.identifier().map(|name| self.normalize_identifier(name)),
            parameter.base_name().map(|name| self.normalize_identifier(name)),
            self.transform_type(parameter.typ()),
        )
    }

    /// Normalize field names.
    fn transform_expr_member(&mut self, _typ: &Type, lhs: &Expr, field: &str) -> Expr {
        let transformed_lhs = self.transform_expr(lhs);
        transformed_lhs.member(&self.normalize_identifier(field), self.symbol_table())
    }

    /// Normalize name in identifier expression.
    fn transform_expr_symbol(&mut self, typ: &Type, identifier: &str) -> Expr {
        let transformed_typ = self.transform_type(typ);
        Expr::symbol_expression(self.normalize_identifier(identifier), transformed_typ)
    }

    /// Normalize union field names.
    fn transform_expr_union(&mut self, typ: &Type, value: &Expr, field: &str) -> Expr {
        let transformed_typ = self.transform_type(typ);
        let transformed_value = self.transform_expr(value);
        Expr::union_expr(
            transformed_typ,
            &self.normalize_identifier(field),
            transformed_value,
            self.symbol_table(),
        )
    }

    /// Normalize incomplete struct tag name.
    fn transform_type_incomplete_struct(&mut self, tag: &str) -> Type {
        Type::incomplete_struct(&self.normalize_identifier(tag))
    }

    /// Normalize incomplete union tag name.
    fn transform_type_incomplete_union(&mut self, tag: &str) -> Type {
        Type::incomplete_union(&self.normalize_identifier(tag))
    }

    /// Normalize union/struct component name.
    fn transform_datatype_component(&mut self, component: &DatatypeComponent) -> DatatypeComponent {
        match component {
            DatatypeComponent::Field { name, typ } => DatatypeComponent::Field {
                name: self.normalize_identifier(name),
                typ: self.transform_type(typ),
            },
            DatatypeComponent::Padding { name, bits } => {
                DatatypeComponent::Padding { name: self.normalize_identifier(name), bits: *bits }
            }
        }
    }

    /// Normalize struct type name.
    fn transform_type_struct(&mut self, tag: &str, components: &[DatatypeComponent]) -> Type {
        let transformed_components = components
            .iter()
            .map(|component| self.transform_datatype_component(component))
            .collect();
        Type::struct_type(&self.normalize_identifier(tag), transformed_components)
    }

    /// Normalize struct tag name.
    fn transform_type_struct_tag(&mut self, tag: &str) -> Type {
        Type::struct_tag_raw(&self.normalize_identifier(tag))
    }

    /// Normalize union type name.
    fn transform_type_union(&mut self, tag: &str, components: &[DatatypeComponent]) -> Type {
        let transformed_components = components
            .iter()
            .map(|component| self.transform_datatype_component(component))
            .collect();
        Type::union_type(&self.normalize_identifier(tag), transformed_components)
    }

    /// Normalize union tag name.
    fn transform_type_union_tag(&mut self, tag: &str) -> Type {
        Type::union_tag_raw(&self.normalize_identifier(tag))
    }

    /// Normalize goto label name.
    fn transform_stmt_goto(&mut self, label: &str) -> Stmt {
        Stmt::goto(self.normalize_identifier(label), Location::none())
    }

    /// Normalize label name.
    fn transform_stmt_label(&mut self, label: &str, body: &Stmt) -> Stmt {
        let transformed_body = self.transform_stmt(body);
        transformed_body.with_label(self.normalize_identifier(label))
    }

    /// Normalize symbol names.
    fn transform_symbol(&mut self, symbol: &Symbol) -> Symbol {
        let mut new_symbol = symbol.clone();
        new_symbol.typ = self.transform_type(&symbol.typ);
        new_symbol.value = self.transform_value(&symbol.value);

        new_symbol.name = self.normalize_identifier(&new_symbol.name);
        new_symbol.base_name = new_symbol.base_name.map(|name| self.normalize_identifier(&name));
        new_symbol.pretty_name =
            new_symbol.pretty_name.map(|name| self.normalize_identifier(&name));
        new_symbol
    }
}
