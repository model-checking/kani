// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

use super::super::super::{
    BinaryOperand, CIntType, Expr, Location, Parameter, Stmt, Symbol, SymbolTable, SymbolValues,
    Type,
};
use super::super::Transformer;
use super::common::type_to_string;
use num::bigint::BigInt;
use rustc_data_structures::fx::FxHashMap;

/// Create an expr from an int constant using only values <= u64::MAX.
fn bignum_to_expr(num: &BigInt, typ: &Type) -> Expr {
    let u64_bigint = BigInt::from(u64::MAX);
    if num <= &u64_bigint {
        Expr::int_constant(num.clone(), typ.clone())
    } else {
        let quotient = num / &u64_bigint;
        let remainder = num % &u64_bigint;

        let quotient_expr = bignum_to_expr(&quotient, typ);
        let remainder_expr = bignum_to_expr(&remainder, typ);
        Expr::int_constant(u64_bigint, typ.clone()).mul(quotient_expr).plus(remainder_expr)
    }
}

/// Struct for handling the expression replacement transformations for --gen-c-runnable.
pub struct ExprTransformer {
    new_symbol_table: SymbolTable,
    empty_statics: FxHashMap<String, Expr>,
}

impl ExprTransformer {
    /// Replace expressions which lead to invalid C with alternatives.
    pub fn transform(original_symbol_table: &SymbolTable) -> SymbolTable {
        let new_symbol_table = SymbolTable::new(original_symbol_table.machine_model().clone());
        ExprTransformer { new_symbol_table, empty_statics: FxHashMap::default() }
            .transform_symbol_table(original_symbol_table)
    }

    /// Extract `empty_statics` map for final processing.
    pub fn empty_statics_owned(&mut self) -> FxHashMap<String, Expr> {
        std::mem::replace(&mut self.empty_statics, FxHashMap::default())
    }

    /// Add identifier to a transformed parameter if it's missing.
    /// Necessary when function wasn't originally a definition.
    fn add_parameter_identifier(&mut self, parameter: &Parameter) -> Parameter {
        if parameter.identifier().is_some() {
            parameter.clone()
        } else {
            let new_name = format!("__{}", type_to_string(parameter.typ()));
            let parameter_sym = Symbol::variable(
                new_name.clone(),
                new_name.clone(),
                parameter.typ().clone(),
                Location::none(),
            );
            let parameter = parameter_sym.to_function_parameter();
            self.mut_symbol_table().insert(parameter_sym);
            parameter
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
            BinaryOperand::And => lhs.and(rhs),
            BinaryOperand::Ashr => lhs.ashr(rhs),
            BinaryOperand::Bitand => lhs.bitand(rhs),
            BinaryOperand::Bitor => lhs.bitor(rhs),
            BinaryOperand::Bitxor => lhs.bitxor(rhs),
            BinaryOperand::Div => lhs.div(rhs),
            BinaryOperand::Equal => lhs.eq(rhs),
            BinaryOperand::Ge => lhs.ge(rhs),
            BinaryOperand::Gt => lhs.gt(rhs),
            BinaryOperand::IeeeFloatEqual => lhs.feq(rhs),
            BinaryOperand::IeeeFloatNotequal => lhs.fneq(rhs),
            // `lhs ==> rhs` <==> `!lhs || rhs`
            BinaryOperand::Implies => lhs.not().or(rhs),
            BinaryOperand::Le => lhs.le(rhs),
            BinaryOperand::Lshr => lhs.lshr(rhs),
            BinaryOperand::Lt => lhs.lt(rhs),
            BinaryOperand::Minus => lhs.sub(rhs),
            BinaryOperand::Mod => lhs.rem(rhs),
            BinaryOperand::Mult => lhs.mul(rhs),
            BinaryOperand::Notequal => lhs.neq(rhs),
            BinaryOperand::Or => lhs.or(rhs),
            BinaryOperand::OverflowMinus => lhs.sub_overflow_p(rhs),
            BinaryOperand::OverflowMult => lhs.mul_overflow_p(rhs),
            BinaryOperand::OverflowPlus => lhs.add_overflow_p(rhs),
            BinaryOperand::Plus => lhs.plus(rhs),
            BinaryOperand::Rol => lhs.rol(rhs),
            BinaryOperand::Ror => lhs.ror(rhs),
            BinaryOperand::Shl => lhs.shl(rhs),
            BinaryOperand::Xor => lhs.xor(rhs),
        }
    }

    /// Prevent error for too large constants with u128.
    fn transform_expr_int_constant(&mut self, typ: &Type, value: &BigInt) -> Expr {
        let transformed_typ = self.transform_type(typ);
        bignum_to_expr(value, &transformed_typ)
    }

    /// When indexing into a SIMD vector, cast to a pointer first to make legal indexing in C.
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
        let (new_typ, new_value) = if symbol.is_extern {
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

                let ret_typ = transformed_typ.return_type().unwrap();
                let (ret_typ, body) = if ret_typ.type_name() == Some("tag-Unit".to_string()) {
                    // If return type is unit type, make return type `void` and use empty body
                    let ret_typ = Type::empty();
                    let body = Stmt::block(vec![], Location::none());
                    (ret_typ, body)
                } else {
                    // Otherwise, set body to return nondet
                    let nondet_expr = Expr::nondet(ret_typ.clone());
                    let body = Stmt::ret(Some(nondet_expr), Location::none());
                    (ret_typ.clone(), body)
                };

                let new_typ = if transformed_typ.is_code() {
                    Type::code(parameters, ret_typ)
                } else {
                    Type::variadic_code(parameters, ret_typ)
                };
                let new_value = SymbolValues::Stmt(body);

                (new_typ, new_value)
            } else {
                // Replace `extern static`s and initialize in `main`

                assert!(
                    symbol.is_static_lifetime,
                    "Extern objects that aren't functions should be static variables."
                );
                let new_typ = self.transform_type(&symbol.typ);
                let new_value = SymbolValues::Expr(Expr::nondet(new_typ.clone()));
                self.empty_statics.insert(symbol.name.clone(), Expr::nondet(new_typ.clone()));

                (new_typ, new_value)
            }
        } else {
            // Handle non-extern symbols normally
            let new_typ = self.transform_type(&symbol.typ);
            let new_value = self.transform_value(&symbol.value);
            (new_typ, new_value)
        };

        let mut new_symbol = symbol.clone();
        new_symbol.typ = new_typ;
        new_symbol.value = new_value;
        new_symbol.with_is_extern(false)
    }

    /// Move `main` to `main_`, and create a wrapper `main` to initialize statics and return `int`.
    fn postprocess(&mut self) {
        if let Some(old_main) = self.mut_symbol_table().remove("main") {
            // Rename `main` to `main_`
            let mut main_ = old_main;
            main_.name = "main_".to_string();
            main_.base_name = Some("main_".to_string());
            main_.pretty_name = Some("main_".to_string());

            let mut main_body = Vec::new();

            // Initialize statics
            for (name, value) in self.empty_statics_owned() {
                let sym_expr = Expr::symbol_expression(name, value.typ().clone());
                main_body.push(Stmt::assign(sym_expr, value, Location::none()));
            }

            main_body.extend(
                vec![
                    // `main_();`
                    Stmt::code_expression(main_.to_expr().call(Vec::new()), Location::none()),
                    // `return 0;`
                    Stmt::ret(
                        Some(Expr::int_constant(0, Type::CInteger(CIntType::Int))),
                        Location::none(),
                    ),
                ]
                .into_iter(),
            );

            // Create `main` symbol
            let new_main = Symbol::function(
                "main",
                Type::code(Vec::new(), Type::CInteger(CIntType::Int)),
                Some(Stmt::block(main_body, Location::none())),
                Some("main".to_string()),
                Location::none(),
            );

            self.mut_symbol_table().insert(main_);
            self.mut_symbol_table().insert(new_main);
        }
    }
}
