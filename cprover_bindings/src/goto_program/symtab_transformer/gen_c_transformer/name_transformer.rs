// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use super::super::Transformer;
use crate::goto_program::{
    DatatypeComponent, Expr, Location, Parameter, Stmt, Symbol, SymbolTable, Type,
};
use crate::InternedString;
use std::collections::{HashMap, HashSet};
/// Struct for replacing names with valid C identifiers for --gen-c-runnable.
pub struct NameTransformer {
    new_symbol_table: SymbolTable,
    /// We want to ensure that the `normalize_identifier` function is both
    /// functional (each Rust name always maps to the same C name) and
    /// injective (two distinct Rust names map to two distinct C names).
    /// To do this, `mapped_names` remembers what each Rust name gets transformed to,
    /// and `used_names` keeps track of what C names have been used.
    // TODO: use InternedString to save memory.
    mapped_names: HashMap<String, String>,
    used_names: HashSet<String>,
}

impl NameTransformer {
    /// Transform all identifiers in the symbol table to be valid C identifiers.
    pub fn transform(original_symbol_table: &SymbolTable) -> SymbolTable {
        let new_symbol_table = SymbolTable::new(original_symbol_table.machine_model().clone());
        NameTransformer {
            new_symbol_table,
            mapped_names: HashMap::default(),
            used_names: HashSet::default(),
        }
        .transform_symbol_table(original_symbol_table)
    }

    fn normalize_identifier(&mut self, orig_name: InternedString) -> InternedString {
        self.normalize_identifier_inner(&orig_name.to_string()).into()
    }

    /// Converts an arbitrary identifier into a valid C identifier.
    fn normalize_identifier_inner(&mut self, orig_name: &str) -> String {
        assert!(!orig_name.is_empty(), "Received empty identifier.");

        // If name already encountered, return same result;
        // this is necessary for correctness to avoid a single name
        // being mapped to two different names later on
        if let Some(result) = self.mapped_names.get(orig_name) {
            return result.clone();
        }

        // Don't tranform the `tag-` prefix of identifiers
        let (prefix, name) = if let Some(tag) = orig_name.strip_prefix("tag-") {
            ("tag-", tag)
        } else {
            ("", orig_name)
        };

        // Separate function name from variable name for CBMC
        let (name, suffix) = {
            let mut parts = name.split("::1::");
            let name = parts.next().unwrap();
            let suffix = parts.next();
            assert!(parts.next().is_none(), "Found multiple occurrences of '::1::' in identifier.");
            (name, suffix)
        };

        // We separately call fix_name on the main part of the name
        // and the suffix. This allows us to use the :: separator
        // between the two parts, and also ensure that the
        // base name of a variable stays as the suffix of the unique name.
        fn fix_name(name: &str) -> String {
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
                None => "".to_string(),
            };

            // Replace reserved names with alternatives
            // This should really handle *all* reserved C names.
            // Tracking issue: https://github.com/model-checking/kani/issues/439
            let illegal_names = [("case", "case_"), ("default", "default_")];
            for (illegal, replacement) in illegal_names {
                if new_name.ends_with(illegal) {
                    return new_name.replace(illegal, replacement);
                }
            }

            new_name
        }

        let name = fix_name(name);
        let suffix = suffix.map(fix_name);

        // Add `tag-` back in if it was present
        let with_prefix = format!("{prefix}{name}");

        // Reattach the variable name
        let result = match suffix {
            None => with_prefix,
            Some(suffix) => format!("{with_prefix}::{suffix}"),
        };

        // Ensure result has not been used before
        let result = if self.used_names.contains(&result) {
            let mut suffix = 0;
            loop {
                let result = format!("{result}_{suffix}");
                if !self.used_names.contains(&result) {
                    break result;
                }
                suffix += 1;
            }
        } else {
            result
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
        self.transform_type(parameter.typ()).as_parameter(
            parameter.identifier().map(|name| self.normalize_identifier(name)),
            parameter.base_name().map(|name| self.normalize_identifier(name)),
        )
    }

    /// Normalize field names.
    fn transform_expr_member(&mut self, _typ: &Type, lhs: &Expr, field: InternedString) -> Expr {
        let transformed_lhs = self.transform_expr(lhs);
        transformed_lhs.member(self.normalize_identifier(field), self.symbol_table())
    }

    /// Normalize name in identifier expression.
    fn transform_expr_symbol(&mut self, typ: &Type, identifier: InternedString) -> Expr {
        let transformed_typ = self.transform_type(typ);
        Expr::symbol_expression(self.normalize_identifier(identifier), transformed_typ)
    }

    /// Normalize union field names.
    fn transform_expr_union(&mut self, typ: &Type, value: &Expr, field: InternedString) -> Expr {
        let transformed_typ = self.transform_type(typ);
        let transformed_value = self.transform_expr(value);
        Expr::union_expr(
            transformed_typ,
            self.normalize_identifier(field),
            transformed_value,
            self.symbol_table(),
        )
    }

    /// Normalize incomplete struct tag name.
    fn transform_type_incomplete_struct(&mut self, tag: InternedString) -> Type {
        Type::incomplete_struct(self.normalize_identifier(tag))
    }

    /// Normalize incomplete union tag name.
    fn transform_type_incomplete_union(&mut self, tag: InternedString) -> Type {
        Type::incomplete_union(self.normalize_identifier(tag))
    }

    /// Normalize union/struct component name.
    fn transform_datatype_component(&mut self, component: &DatatypeComponent) -> DatatypeComponent {
        match component {
            DatatypeComponent::Field { name, typ } => {
                DatatypeComponent::field(self.normalize_identifier(*name), self.transform_type(typ))
            }
            DatatypeComponent::Padding { name, bits } => {
                DatatypeComponent::padding(self.normalize_identifier(*name), *bits)
            }
        }
    }

    /// Normalize struct type name.
    fn transform_type_struct(
        &mut self,
        tag: InternedString,
        components: &[DatatypeComponent],
    ) -> Type {
        let transformed_components = components
            .iter()
            .map(|component| self.transform_datatype_component(component))
            .collect();
        Type::struct_type(self.normalize_identifier(tag), transformed_components)
    }

    /// Normalize struct tag name.
    fn transform_type_struct_tag(&mut self, tag: InternedString) -> Type {
        Type::struct_tag_raw(self.normalize_identifier(tag))
    }

    /// Normalize union type name.
    fn transform_type_union(
        &mut self,
        tag: InternedString,
        components: &[DatatypeComponent],
    ) -> Type {
        let transformed_components = components
            .iter()
            .map(|component| self.transform_datatype_component(component))
            .collect();
        Type::union_type(self.normalize_identifier(tag), transformed_components)
    }

    /// Normalize union tag name.
    fn transform_type_union_tag(&mut self, tag: InternedString) -> Type {
        Type::union_tag_raw(self.normalize_identifier(tag))
    }

    /// Normalize goto label name.
    fn transform_stmt_goto(&mut self, label: InternedString) -> Stmt {
        Stmt::goto(self.normalize_identifier(label), Location::none())
    }

    /// Normalize label name.
    fn transform_stmt_label(&mut self, label: InternedString, body: &Stmt) -> Stmt {
        let transformed_body = self.transform_stmt(body);
        transformed_body.with_label(self.normalize_identifier(label))
    }

    /// Normalize symbol names.
    fn transform_symbol(&mut self, symbol: &Symbol) -> Symbol {
        let mut new_symbol = symbol.clone();
        new_symbol.typ = self.transform_type(&symbol.typ);
        new_symbol.value = self.transform_value(&symbol.value);

        new_symbol.name = self.normalize_identifier(new_symbol.name);
        new_symbol.base_name = new_symbol.base_name.map(|name| self.normalize_identifier(name));
        new_symbol.pretty_name = new_symbol.pretty_name.map(|name| self.normalize_identifier(name));

        new_symbol
    }
}
