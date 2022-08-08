// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use super::super::Transformer;
use crate::goto_program::{Expr, Location, Stmt, Symbol, SymbolTable, Type};
use std::collections::HashMap;

/// Struct for handling the nondet transformations for --gen-c-runnable.
pub struct NondetTransformer {
    new_symbol_table: SymbolTable,
    nondet_types: HashMap<String, Type>,
    poison_types: HashMap<String, Type>,
}

impl NondetTransformer {
    /// Transform all identifiers in the symbol table to be valid C identifiers;
    /// perform other clean-up operations to make valid C code.
    pub fn transform(original_symbol_table: &SymbolTable) -> SymbolTable {
        let new_symbol_table = SymbolTable::new(original_symbol_table.machine_model().clone());
        NondetTransformer {
            new_symbol_table,
            nondet_types: HashMap::default(),
            poison_types: HashMap::default(),
        }
        .transform_symbol_table(original_symbol_table)
    }

    /// Extract `nondet_types` map for final processing.
    pub fn nondet_types_owned(&mut self) -> HashMap<String, Type> {
        std::mem::take(&mut self.nondet_types)
    }

    pub fn poison_types_owned(&mut self) -> HashMap<String, Type> {
        std::mem::take(&mut self.poison_types)
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
    /// Given: `let x: u32 = __nondet();`
    /// Transformed:
    /// ```
    ///   unsigned int x = non_det_unsigned_bv_32();
    /// ...
    /// unsigned int non_det_unsigned_bv_32(void) {
    ///     unsigned int ret;
    ///     return ret;
    /// }
    /// ```
    fn transform_expr_nondet(&mut self, typ: &Type) -> Expr {
        let transformed_typ = self.transform_type(typ);

        let identifier = format!("non_det_{}", transformed_typ.to_identifier());
        let function_type = Type::code(vec![], transformed_typ);

        // Create non_det function which returns default value in postprocessing
        self.nondet_types.insert(identifier.clone(), function_type.clone());

        Expr::symbol_expression(identifier, function_type).call(vec![])
    }

    fn transform_expr_poison(&mut self, typ: &Type) -> Expr {
        let transformed_type = self.transform_type(typ);
        let identifier = format!("poison_{}", transformed_type.to_identifier());
        let function_type = Type::code(vec![], transformed_type);

        self.poison_types.insert(identifier.clone(), function_type.clone());

        Expr::symbol_expression(identifier, function_type).call(vec![])
    }

    /// Don't transform padding fields so that they are ignored by CBMC --dump-c.
    /// If we don't ignore padding fields, we get code that looks like
    /// ```
    ///   var_7 = size;
    ///   var_8 = l;
    ///   unsigned __CPROVER_bitvector[56] return_value_non_det_unsigned_bv_56=non_det_unsigned_bv_56();
    ///   var_9 = (struct _usize__bool_){ ._0=var_7 * var_8, ._1=overflow("*", unsigned long int, var_7, var_8) };
    /// ```
    /// If we do ignore the padding fields, the third line is removed.
    fn transform_expr_struct(&mut self, typ: &Type, values: &[Expr]) -> Expr {
        let transformed_typ = self.transform_type(typ);
        assert!(
            transformed_typ.is_struct_tag(),
            "Transformed StructTag must be StructTag; got {:?}",
            transformed_typ
        );

        // Instead of just mapping `self.transform_expr` over the values,
        // only transform those which are true fields, not padding
        let fields = transformed_typ.lookup_components(self.symbol_table()).unwrap().clone();
        let transformed_values: Vec<_> = fields
            .into_iter()
            .zip(values.iter())
            .map(
                |(field, value)| {
                    if field.is_padding() { value.clone() } else { self.transform_expr(value) }
                },
            )
            .collect();

        Expr::struct_expr_from_padded_values(
            transformed_typ,
            transformed_values,
            self.symbol_table(),
        )
    }

    /// Create non_det functions which return default value for type.
    fn postprocess(&mut self) {
        for (identifier, typ) in
            self.nondet_types_owned().into_iter().chain(self.poison_types_owned().into_iter())
        {
            // Create function body which initializes variable and returns it
            let ret_type = typ.return_type().unwrap();
            assert!(!ret_type.is_empty(), "Cannot generate nondet of type `void`.");
            let ret_name = format!("{}_ret", &identifier);
            let ret_expr = Expr::symbol_expression(&ret_name, ret_type.clone());
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
            let func_sym =
                Symbol::function(&identifier, typ, Some(body), &identifier, Location::none());
            self.mut_symbol_table().insert(func_sym);
        }
    }
}
