// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

use super::super::{
    DatatypeComponent, Expr, Location, Stmt, Symbol, SymbolTable, SymbolValues, Type,
};
use super::Transformer;
use std::collections::{BTreeMap, HashSet};

/// Struct for performing the identity transformation on a symbol table.
/// Mainly used as a demo/for testing.
pub struct IdentityTransformer {
    new_symbol_table: SymbolTable,
}

impl IdentityTransformer {
    /// Perform an identity transformation on the given symbol table.
    pub fn transform(original_symbol_table: &SymbolTable) -> SymbolTable {
        let new_symbol_table = SymbolTable::new(original_symbol_table.machine_model().clone());
        IdentityTransformer { new_symbol_table }.transform_symbol_table(original_symbol_table)
    }
}

impl Transformer for IdentityTransformer {
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
}

#[cfg(test)]
mod tests {
    use super::{
        super::super::{
            super::{MachineModel, RoundingMode},
            DatatypeComponent, Expr, Location, Stmt, SwitchCase, Symbol, SymbolTable, Type,
        },
        IdentityTransformer,
    };
    fn default_machine_model() -> MachineModel {
        MachineModel::new(
            1,
            "x86_64",
            8,
            false,
            8,
            64,
            32,
            32,
            false,
            128,
            64,
            64,
            4,
            true,
            64,
            RoundingMode::ToNearest,
            16,
            32,
            false,
            32,
            32,
        )
    }

    fn empty_symtab() -> SymbolTable {
        SymbolTable::new(default_machine_model())
    }

    fn assert_transform_eq(original: SymbolTable) {
        let transformed = IdentityTransformer::transform(&original);
        assert_eq!(original.to_irep(), transformed.to_irep());
    }

    #[test]
    fn empty() {
        let original = empty_symtab();
        assert_transform_eq(original);
    }

    #[test]
    fn types() {
        let mut original = empty_symtab();
        let mut curr_var = 0;
        {
            let mut add_sym = |typ| {
                let name = curr_var.to_string();
                original.insert(Symbol::typedef(&name, &name, typ, Location::none()));
                curr_var += 1;
            };
            add_sym(Type::bool().array_of(3));
            add_sym(Type::float().array_of(5));
            add_sym(Type::bool());
            add_sym(Type::signed_int(16).as_bitfield(8));
            add_sym(Type::c_int());
            add_sym(Type::c_bool());
            add_sym(Type::c_char());
            add_sym(Type::code_with_unnamed_parameters(
                vec![Type::bool(), Type::c_int()],
                Type::float(),
            ));
            add_sym(Type::constructor());
            add_sym(Type::double());
            add_sym(Type::empty());
            add_sym(Type::double().flexible_array_of());
            add_sym(Type::float());
            add_sym(Type::incomplete_struct("a"));
            add_sym(Type::incomplete_union("b"));
            add_sym(Type::float().infinite_array_of());
            add_sym(Type::double().to_pointer());
            add_sym(Type::signed_int(8));
            add_sym(Type::empty_struct("c"));
            add_sym(Type::struct_tag("d"));
            add_sym(Type::empty_union("e"));
            add_sym(Type::union_tag("f"));
            add_sym(Type::unsigned_int(8));
            add_sym(Type::variadic_code_with_unnamed_parameters(
                vec![Type::float(), Type::c_int()],
                Type::signed_int(8),
            ));
            add_sym(Type::vector(Type::double(), 6));
        }

        assert_transform_eq(original);
    }

    #[test]
    fn struct_types() {
        let mut original = empty_symtab();
        let mut curr_var = 0;
        {
            let mut add_sym = |typ| {
                let name = curr_var.to_string();
                original.insert(Symbol::typedef(&name, &name, typ, Location::none()));
                curr_var += 1;
            };

            let struct_tag = Type::struct_tag("s-t");
            add_sym(struct_tag);

            let incomplete_struct = Type::incomplete_struct("i-s");
            add_sym(incomplete_struct);

            let struct_type = Type::struct_type(
                "s",
                vec![
                    DatatypeComponent::Field { name: "a".to_string(), typ: Type::float() },
                    DatatypeComponent::Padding { name: "b".to_string(), bits: 4 },
                    DatatypeComponent::Field { name: "c".to_string(), typ: Type::double() },
                    DatatypeComponent::Padding { name: "d".to_string(), bits: 5 },
                    DatatypeComponent::Field { name: "e".to_string(), typ: Type::c_int() },
                    DatatypeComponent::Padding { name: "f".to_string(), bits: 6 },
                ],
            );
            add_sym(struct_type);
        }

        assert_transform_eq(original);
    }

    #[test]
    fn union_types() {
        let mut original = empty_symtab();
        let mut curr_var = 0;
        {
            let mut add_sym = |typ| {
                let name = curr_var.to_string();
                original.insert(Symbol::typedef(&name, &name, typ, Location::none()));
                curr_var += 1;
            };

            let union_tag = Type::union_tag("u-t");
            add_sym(union_tag);

            let incomplete_union = Type::incomplete_union("i-u");
            add_sym(incomplete_union);

            let union_type = Type::union_type(
                "u",
                vec![
                    DatatypeComponent::Field { name: "a".to_string(), typ: Type::float() },
                    DatatypeComponent::Field { name: "c".to_string(), typ: Type::double() },
                    DatatypeComponent::Field { name: "e".to_string(), typ: Type::c_int() },
                ],
            );
            add_sym(union_type);
        }

        assert_transform_eq(original);
    }

    #[test]
    fn exprs() {
        let mut original = empty_symtab();
        let mut curr_var = 0;
        {
            let mut add_sym = |value| {
                let name = curr_var.to_string();
                original.insert(Symbol::constant(&name, &name, &name, value, Location::none()));
                curr_var += 1;
            };

            add_sym(Expr::symbol_expression("a".to_string(), Type::c_int()).address_of());
            add_sym(Expr::int_constant(5, Type::c_int()).array_constant(10));
            add_sym(Expr::array_expr(
                Type::bool().array_of(2),
                vec![Expr::bool_true(), Expr::bool_false()],
            ));
            add_sym(Expr::bool_constant(true));
            add_sym(Expr::bool_false());
            add_sym(Expr::bool_true());
            add_sym(Expr::c_bool_constant(true));
            add_sym(Expr::c_false());
            add_sym(Expr::c_true());
            add_sym(Expr::c_true().cast_to(Type::c_int()));
            add_sym(
                Expr::symbol_expression("a".to_string(), Type::c_int()).address_of().dereference(),
            );
            add_sym(Expr::double_constant(1.0));
            add_sym(Expr::float_constant(1.0));
            add_sym(
                Expr::array_expr(
                    Type::bool().array_of(2),
                    vec![Expr::bool_true(), Expr::bool_false()],
                )
                .index_array(Expr::int_constant(1, Type::c_int())),
            );
            add_sym(Expr::int_constant(1, Type::c_int()));
            add_sym(
                Expr::symbol_expression(
                    "a".to_string(),
                    Type::code_with_unnamed_parameters(
                        vec![Type::bool(), Type::float()],
                        Type::double(),
                    ),
                )
                .call(vec![Expr::bool_true(), Expr::float_constant(1.0)]),
            );
            add_sym(Expr::nondet(Type::bool()));
            add_sym(Expr::pointer_constant(128, Type::bool().to_pointer()));
            add_sym(Expr::statement_expression(
                vec![
                    Stmt::skip(Location::none()),
                    Stmt::code_expression(Expr::float_constant(1.0), Location::none()),
                ],
                Type::float(),
            ));
            add_sym(Expr::symbol_expression("x".to_string(), Type::bool()));
            add_sym(
                Expr::bool_true().ternary(Expr::float_constant(1.0), Expr::float_constant(2.0)),
            );
            add_sym(
                Expr::int_constant(1, Type::c_int())
                    .add_overflow_p(Expr::int_constant(2, Type::c_int())),
            );
            add_sym(Expr::bool_true().and(Expr::bool_false()));
            add_sym(Expr::int_constant(1, Type::c_int()).postincr());
            add_sym(Expr::bool_true().not());
        }
        assert_transform_eq(original);
    }

    #[test]
    fn struct_exprs() {
        let mut original = empty_symtab();

        let struct_type = Symbol::struct_type(
            "s",
            vec![
                DatatypeComponent::Field { name: "a".to_string(), typ: Type::float() },
                DatatypeComponent::Padding { name: "b".to_string(), bits: 4 },
                DatatypeComponent::Field { name: "c".to_string(), typ: Type::double() },
                DatatypeComponent::Padding { name: "d".to_string(), bits: 5 },
                DatatypeComponent::Field { name: "e".to_string(), typ: Type::c_int() },
                DatatypeComponent::Padding { name: "f".to_string(), bits: 6 },
            ],
        );
        original.insert(struct_type);

        let struct_expr = Expr::struct_expr_from_values(
            Type::struct_tag("s"),
            vec![
                Expr::float_constant(1.0),
                Expr::double_constant(2.0),
                Expr::int_constant(3, Type::c_int()),
            ],
            &original,
        );

        original.insert(Symbol::constant("se", "se", "se", struct_expr.clone(), Location::none()));

        let struct_member = struct_expr.member("a", &original);
        original.insert(Symbol::constant("sm", "sm", "sm", struct_member, Location::none()));

        assert_transform_eq(original);
    }

    #[test]
    fn union_exprs() {
        let mut original = empty_symtab();

        let union_type = Symbol::union_type(
            "u",
            vec![
                DatatypeComponent::Field { name: "a".to_string(), typ: Type::float() },
                DatatypeComponent::Field { name: "c".to_string(), typ: Type::double() },
                DatatypeComponent::Field { name: "e".to_string(), typ: Type::c_int() },
            ],
        );
        original.insert(union_type);

        let union_expr =
            Expr::union_expr(Type::union_tag("u"), "a", Expr::float_constant(1.0), &original);

        original.insert(Symbol::constant("ue", "ue", "ue", union_expr.clone(), Location::none()));

        let union_member = union_expr.member("a", &original);
        original.insert(Symbol::constant("um", "um", "um", union_member, Location::none()));

        assert_transform_eq(original);
    }

    #[test]
    fn transmute_to_expr() {
        let mut original = empty_symtab();
        let sym = Symbol::constant(
            "tt",
            "tt",
            "tt",
            Expr::array_expr(Type::c_int().array_of(1), vec![Expr::int_constant(3, Type::c_int())])
                .transmute_to(Type::c_int(), &original),
            Location::none(),
        );
        original.insert(sym);

        assert_transform_eq(original);
    }

    #[test]
    fn stmts() {
        let mut original = empty_symtab();
        let mut curr_var = 0;
        {
            let mut add_sym = |body| {
                let name = curr_var.to_string();
                original.insert(Symbol::function(
                    &name,
                    Type::code_with_unnamed_parameters(vec![], Type::empty()),
                    Some(body),
                    None,
                    Location::none(),
                ));
                curr_var += 1;
            };

            add_sym(Stmt::assign(
                Expr::symbol_expression("a".to_string(), Type::bool()),
                Expr::bool_true(),
                Location::none(),
            ));
            add_sym(Stmt::assert(Expr::bool_true(), "", Location::none()));
            add_sym(Stmt::assume(Expr::bool_false(), Location::none()));
            add_sym(Stmt::atomic_block(
                vec![Stmt::assert_false("", Location::none())],
                Location::none(),
            ));
            add_sym(Stmt::block(vec![Stmt::assert_false("", Location::none())], Location::none()));
            add_sym(Stmt::break_stmt(Location::none()));
            add_sym(Stmt::continue_stmt(Location::none()));
            add_sym(Stmt::decl(
                Expr::symbol_expression("a".to_string(), Type::bool()),
                Some(Expr::bool_true()),
                Location::none(),
            ));
            add_sym(Stmt::code_expression(Expr::bool_true(), Location::none()));
            add_sym(Stmt::for_loop(
                Stmt::decl(
                    Expr::symbol_expression("a".to_string(), Type::bool()),
                    Some(Expr::bool_true()),
                    Location::none(),
                ),
                Expr::bool_true(),
                Stmt::assign(
                    Expr::symbol_expression("a".to_string(), Type::bool()),
                    Expr::bool_false(),
                    Location::none(),
                ),
                Stmt::continue_stmt(Location::none()),
                Location::none(),
            ));
            add_sym(Stmt::function_call(
                Some(Expr::symbol_expression("a".to_string(), Type::bool())),
                Expr::symbol_expression(
                    "b".to_string(),
                    Type::code_with_unnamed_parameters(vec![Type::c_int()], Type::bool()),
                ),
                vec![Expr::int_constant(5, Type::c_int())],
                Location::none(),
            ));
            add_sym(Stmt::goto("tag1".to_string(), Location::none()));
            add_sym(Stmt::if_then_else(
                Expr::bool_true(),
                Stmt::continue_stmt(Location::none()),
                Some(Stmt::continue_stmt(Location::none())),
                Location::none(),
            ));
            add_sym(Stmt::ret(Some(Expr::bool_true()), Location::none()));
            add_sym(Stmt::skip(Location::none()));
            add_sym(Stmt::switch(
                Expr::int_constant(3, Type::c_int()),
                vec![
                    SwitchCase::new(
                        Expr::int_constant(1, Type::c_int()),
                        Stmt::ret(None, Location::none()),
                    ),
                    SwitchCase::new(
                        Expr::int_constant(2, Type::c_int()),
                        Stmt::ret(None, Location::none()),
                    ),
                    SwitchCase::new(
                        Expr::int_constant(3, Type::c_int()),
                        Stmt::ret(None, Location::none()),
                    ),
                    SwitchCase::new(
                        Expr::int_constant(4, Type::c_int()),
                        Stmt::ret(None, Location::none()),
                    ),
                ],
                Some(Stmt::goto("tag1".to_string(), Location::none())),
                Location::none(),
            ));
            add_sym(Stmt::while_loop(
                Expr::bool_true(),
                Stmt::skip(Location::none()),
                Location::none(),
            ));
            add_sym(Stmt::assert_false("", Location::none()).with_label("tag1".to_string()));
        }

        assert_transform_eq(original);
    }
}
