// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

use super::super::super::{Expr, Location, Stmt, Symbol, SymbolTable, Type};
use super::super::Transformer;
use super::common::type_to_string;
use rustc_data_structures::fx::FxHashMap;

/// Struct for handling the nondet transformations for --gen-c-runnable.
pub struct NondetTransformer {
    new_symbol_table: SymbolTable,
    nondet_types: FxHashMap<String, Type>,
}

impl NondetTransformer {
    /// Transform all identifiers in the symbol table to be valid C identifiers;
    /// perform other clean-up operations to make valid C code.
    pub fn transform(original_symbol_table: &SymbolTable) -> SymbolTable {
        let new_symbol_table = SymbolTable::new(original_symbol_table.machine_model().clone());
        NondetTransformer { new_symbol_table, nondet_types: FxHashMap::default() }
            .transform_symbol_table(original_symbol_table)
    }

    /// Extract `nondet_types` map for final processing.
    pub fn nondet_types_owned(&mut self) -> FxHashMap<String, Type> {
        std::mem::replace(&mut self.nondet_types, FxHashMap::default())
    }
}

impl Transformer for NondetTransformer {
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

    /// Transform nondets to create default values for the expected type.
    fn transform_expr_nondet(&mut self, typ: &Type) -> Expr {
        let transformed_typ = self.transform_type(typ);
        let typ_string = type_to_string(&transformed_typ);
        let identifier = format!("non_det_{}", typ_string);
        let function_type = Type::code(vec![], transformed_typ);

        // Create non_det function which returns default value in postprocessing
        self.nondet_types.insert(identifier.clone(), function_type.clone());

        Expr::symbol_expression(identifier, function_type).call(vec![])
    }

    /// Don't transform padding fields so that they are ignored by CBMC --dump-c.
    fn transform_expr_struct(&mut self, typ: &Type, values: &[Expr]) -> Expr {
        let transformed_typ = self.transform_type(typ);
        assert!(
            transformed_typ.is_struct_tag(),
            "Transformed StructTag must be StructTag; got {:?}",
            transformed_typ
        );

        // Instead of just mapping `self.transform_expr` over the values,
        // only transform those which are true fields, not padding
        let fields = self.symbol_table().lookup_fields_in_type(&transformed_typ).unwrap().clone();
        let mut transformed_values = Vec::new();
        for (field, value) in fields.into_iter().zip(values.into_iter()) {
            if field.is_padding() {
                transformed_values.push(value.clone());
            } else {
                transformed_values.push(self.transform_expr(value));
            }
        }

        Expr::struct_expr_from_padded_values(
            transformed_typ,
            transformed_values,
            self.symbol_table(),
        )
    }

    /// Create non_det functions which return default value for type.
    fn postprocess(&mut self) {
        for (identifier, typ) in self.nondet_types_owned() {
            // Create function body which initializes variable and returns it
            let ret_type = typ.return_type().unwrap();
            assert!(!ret_type.is_empty(), "Cannot generate nondet of type `void`.");
            let ret_name = format!("{}_ret", &identifier);
            let ret_expr = Expr::symbol_expression(ret_name.clone(), ret_type.clone());
            let body = Stmt::block(
                vec![
                    // <ret_type> var_ret;
                    Stmt::decl(ret_expr.clone(), None, Location::none()),
                    // return var_ret;
                    Stmt::ret(Some(ret_expr), Location::none()),
                ],
                Location::none(),
            );

            // Add return variable to symbol table
            let ret_sym =
                Symbol::variable(ret_name, "ret".to_string(), ret_type.clone(), Location::none());
            self.mut_symbol_table().insert(ret_sym);

            // Add function to symbol table
            let func_sym = Symbol::function(
                &identifier,
                typ,
                Some(body),
                Some(identifier.clone()),
                Location::none(),
            );
            self.mut_symbol_table().insert(func_sym);
        }
    }
}
