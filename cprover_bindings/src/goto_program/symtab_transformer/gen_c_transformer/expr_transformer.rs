// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::ops::{BitAnd, Shl, Shr};

use super::super::Transformer;
use crate::goto_program::{
    BinaryOperand, CIntType, Expr, Location, Parameter, Stmt, Symbol, SymbolTable, SymbolValues,
    Type,
};
use crate::InternedString;
use num::bigint::BigInt;
use std::collections::HashMap;

/// Create an expr from an int constant using only values <= u64::MAX;
/// this is needed because gcc allows 128 bit int variables, but not 128 bit constants
fn bignum_to_expr(num: &BigInt, typ: &Type) -> Expr {
    // CInteger types should already be valid in C
    if typ.is_c_integer() {
        return Expr::int_constant(num.clone(), typ.clone());
    }

    // Only need to handle type wider than 64 bits
    if let Some(width) = typ.width() {
        if width <= 64 {
            return Expr::int_constant(num.clone(), typ.clone());
        }
    }

    // Only types that should be left are i128 and u128
    assert_eq!(typ.width(), Some(128), "Unexpected int width: {:?}", typ.width());

    // Work with unsigned ints, and cast to original type at end
    let unsigned_typ = Type::unsigned_int(128);

    // To transform a 128 bit num, we break it into two 64 bit nums

    // left_mask = 11..1100..00 (64 1's followed by 64 0's)
    // left_mask = 00..0011..11 (64 0's followed by 64 1's)
    let left_mask = BigInt::from(u64::MAX).shl(64);
    let right_mask = BigInt::from(u64::MAX);

    // Construct the two 64 bit ints such that
    // num = (left_half << 64) | right_half
    //     = (left_half * 2^64) + right_half
    let left_half = {
        // Split into two parts to help type inference
        let temp: BigInt = num.bitand(left_mask);
        temp.shr(64)
    };
    let right_half = num.bitand(right_mask);

    // Create CBMC constants for the left and right halfs
    let left_constant = Expr::int_constant(left_half, unsigned_typ.clone());
    let right_constant = Expr::int_constant(right_half, unsigned_typ);

    // Construct CBMC expression: (typ) ((left << 64) | right)
    left_constant
        .shl(Expr::int_constant(64, Type::c_int()))
        .bitor(right_constant)
        .cast_to(typ.clone())
}

/// Struct for handling the expression replacement transformations for --gen-c-runnable.
pub struct ExprTransformer {
    new_symbol_table: SymbolTable,
    /// The `empty_statics` field is used to track extern static variables;
    /// when such a symbol is encountered, we add it to this map;
    /// in postprocessing, we initialize each of these variables
    /// with a default value to emphasize that these are externally defined.
    empty_statics: HashMap<InternedString, Expr>,
}

impl ExprTransformer {
    /// Replace expressions which lead to invalid C with alternatives.
    pub fn transform(original_symbol_table: &SymbolTable) -> SymbolTable {
        let new_symbol_table = SymbolTable::new(original_symbol_table.machine_model().clone());
        ExprTransformer { new_symbol_table, empty_statics: HashMap::default() }
            .transform_symbol_table(original_symbol_table)
    }

    /// Extract `empty_statics` map for final processing.
    fn empty_statics_owned(&mut self) -> HashMap<InternedString, Expr> {
        std::mem::replace(&mut self.empty_statics, HashMap::default())
    }

    /// Add identifier to a transformed parameter if it's missing;
    /// necessary when function wasn't originally a definition, e.g. extern functions,
    /// so that we can give them a function body.
    fn add_parameter_identifier(&mut self, parameter: &Parameter) -> Parameter {
        if parameter.identifier().is_some() {
            parameter.clone()
        } else {
            let name = format!("__{}", parameter.typ().to_identifier());
            let parameter_sym = self.mut_symbol_table().ensure(&name, |_symtab, name| {
                Symbol::variable(name, name, parameter.typ().clone(), Location::none())
            });
            parameter_sym.to_function_parameter()
        }
    }
}

impl Transformer for ExprTransformer {
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

    /// Translate Implies into Or/Not.
    fn transform_expr_bin_op(
        &mut self,
        _typ: &Type,
        op: &BinaryOperand,
        lhs: &Expr,
        rhs: &Expr,
    ) -> Expr {
        let lhs = self.transform_expr(lhs);
        let rhs = self.transform_expr(rhs);

        match op {
            BinaryOperand::Implies => lhs.not().bitor(rhs).cast_to(Type::bool()),
            _ => lhs.binop(*op, rhs),
        }
    }

    /// Prevent error for too large constants with u128.
    fn transform_expr_int_constant(&mut self, typ: &Type, value: &BigInt) -> Expr {
        let transformed_typ = self.transform_type(typ);
        bignum_to_expr(value, &transformed_typ)
    }

    /// When indexing into a SIMD vector, cast to a pointer first to make legal indexing in C.
    /// `typ __attribute__((vector_size (size * sizeof(typ)))) var;`
    /// `((typ*) &var)[index]`
    /// Tracking issue: https://github.com/model-checking/kani/issues/444
    fn transform_expr_index(&mut self, _typ: &Type, array: &Expr, index: &Expr) -> Expr {
        let transformed_array = self.transform_expr(array);
        let transformed_index = self.transform_expr(index);
        if transformed_array.typ().is_vector() {
            let base_type = transformed_array.typ().base_type().unwrap().clone();
            transformed_array.address_of().cast_to(base_type.to_pointer()).index(transformed_index)
        } else {
            transformed_array.index(transformed_index)
        }
    }

    /// Replace `extern` functions and values with `nondet` so linker doesn't break.
    fn transform_symbol(&mut self, symbol: &Symbol) -> Symbol {
        let mut new_symbol = symbol.clone();

        if symbol.is_extern {
            if symbol.typ.is_code() || symbol.typ.is_variadic_code() {
                // Replace `extern` function with one which returns `nondet`
                assert!(symbol.value.is_none(), "Extern function should have no body.");

                let transformed_typ = self.transform_type(&symbol.typ);

                // Fill missing parameter names with dummy name
                let parameters = transformed_typ
                    .parameters()
                    .unwrap()
                    .iter()
                    .map(|parameter| self.add_parameter_identifier(parameter))
                    .collect();
                let ret_typ = transformed_typ.return_type().unwrap().clone();
                let new_typ = if transformed_typ.is_code() {
                    Type::code(parameters, ret_typ.clone())
                } else {
                    Type::variadic_code(parameters, ret_typ.clone())
                };

                // Create body, which returns nondet
                let ret_e = if ret_typ.is_empty() { None } else { Some(Expr::nondet(ret_typ)) };
                let body = Stmt::ret(ret_e, Location::none());
                let new_value = SymbolValues::Stmt(body);

                new_symbol.is_extern = false;
                new_symbol.typ = new_typ;
                new_symbol.value = new_value;
            } else {
                // Replace `extern static`s and initialize in `main`
                assert!(
                    symbol.is_static_lifetime,
                    "Extern objects that aren't functions should be static variables."
                );
                let new_typ = self.transform_type(&symbol.typ);
                self.empty_statics.insert(symbol.name, Expr::nondet(new_typ.clone()));

                // Symbol is no longer extern
                new_symbol.is_extern = false;

                // Set location to none so that it is a global static
                new_symbol.location = Location::none();

                new_symbol.typ = new_typ;
                new_symbol.value = SymbolValues::None;
            }
        } else if symbol.name == "main" {
            // Replace `main` with `main_` since it has the wrong return type
            new_symbol.name = "main_".into();
            new_symbol.base_name = Some("main_".into());
            new_symbol.pretty_name = Some("main_".into());

            let new_typ = self.transform_type(&symbol.typ);
            let new_value = self.transform_value(&symbol.value);

            new_symbol.typ = new_typ;
            new_symbol.value = new_value;
        } else {
            // Handle all other symbols normally
            let new_typ = self.transform_type(&symbol.typ);
            let new_value = self.transform_value(&symbol.value);
            new_symbol.typ = new_typ;
            new_symbol.value = new_value;
        }

        new_symbol
    }

    /// Move `main` to `main_`, and create a wrapper `main` to initialize statics and return `int`.
    fn postprocess(&mut self) {
        // The body of the new `main` function
        let mut main_body = Vec::new();

        // Initialize statics
        for (name, value) in self.empty_statics_owned() {
            let sym_expr = Expr::symbol_expression(name, value.typ().clone());
            main_body.push(Stmt::assign(sym_expr, value, Location::none()));
        }

        // `main_();`, if it is present
        if let Some(main_) = self.symbol_table().lookup("main_") {
            main_body
                .push(Stmt::code_expression(main_.to_expr().call(Vec::new()), Location::none()));
        }

        // `return 0;`
        main_body.push(Stmt::ret(
            Some(Expr::int_constant(0, Type::CInteger(CIntType::Int))),
            Location::none(),
        ));

        // Create `main` symbol
        let new_main = Symbol::function(
            "main",
            Type::code(Vec::new(), Type::CInteger(CIntType::Int)),
            Some(Stmt::block(main_body, Location::none())),
            Some("main"),
            Location::none(),
        );

        self.mut_symbol_table().insert(new_main);
    }
}
