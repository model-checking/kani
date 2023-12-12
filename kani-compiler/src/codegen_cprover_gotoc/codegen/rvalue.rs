// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use super::typ::pointee_type;
use crate::codegen_cprover_gotoc::codegen::place::ProjectedPlace;
use crate::codegen_cprover_gotoc::codegen::PropertyClass;
use crate::codegen_cprover_gotoc::utils::{dynamic_fat_ptr, slice_fat_ptr};
use crate::codegen_cprover_gotoc::{GotocCtx, VtableCtx};
use crate::kani_middle::coercion::{
    extract_unsize_casting, CoerceUnsizedInfo, CoerceUnsizedIterator, CoercionBase,
};
use crate::unwrap_or_return_codegen_unimplemented;
use cbmc::goto_program::{
    arithmetic_overflow_result_type, BinaryOperator, Expr, Location, Stmt, Type,
    ARITH_OVERFLOW_OVERFLOWED_FIELD, ARITH_OVERFLOW_RESULT_FIELD,
};
use cbmc::MachineModel;
use cbmc::{btree_string_map, InternString, InternedString};
use num::bigint::BigInt;
use rustc_index::IndexVec;
use rustc_middle::mir::{AggregateKind, BinOp, CastKind, NullOp, Operand, Place, Rvalue, UnOp};
use rustc_middle::ty::adjustment::PointerCoercion;
use rustc_middle::ty::layout::LayoutOf;
use rustc_middle::ty::{self, Instance, IntTy, Ty, TyCtxt, UintTy, VtblEntry};
use rustc_smir::rustc_internal;
use rustc_target::abi::{FieldIdx, FieldsShape, Size, TagEncoding, VariantIdx, Variants};
use std::collections::BTreeMap;
use tracing::{debug, trace, warn};

impl<'tcx> GotocCtx<'tcx> {
    fn codegen_comparison(&mut self, op: &BinOp, e1: &Operand<'tcx>, e2: &Operand<'tcx>) -> Expr {
        let left_op = self.codegen_operand(e1);
        let right_op = self.codegen_operand(e2);
        let is_float = self.operand_ty(e1).is_floating_point();
        comparison_expr(op, left_op, right_op, is_float)
    }

    /// This function codegen comparison for fat pointers.
    /// Fat pointer comparison must compare the raw data pointer as well as its metadata portion.
    ///
    /// Since vtable pointer comparison is not well defined and it has many nuances, we decided to
    /// fail if the user code performs such comparison.
    ///
    /// See <https://github.com/model-checking/kani/issues/327> for more details.
    fn codegen_comparison_fat_ptr(
        &mut self,
        op: &BinOp,
        left_op: &Operand<'tcx>,
        right_op: &Operand<'tcx>,
        loc: Location,
    ) -> Expr {
        debug!(?op, ?left_op, ?right_op, "codegen_comparison_fat_ptr");
        let left_typ = self.operand_ty(left_op);
        let right_typ = self.operand_ty(left_op);
        assert_eq!(left_typ, right_typ, "Cannot compare pointers of different types");
        assert!(self.is_fat_pointer(left_typ));

        if self.is_vtable_fat_pointer(left_typ) {
            // Codegen an assertion failure since vtable comparison is not stable.
            let ret_type = Type::Bool;
            let body = vec![
                self.codegen_assert_assume_false(
                    PropertyClass::SafetyCheck,
                    format!("Reached unstable vtable comparison '{op:?}'").as_str(),
                    loc,
                ),
                ret_type.nondet().as_stmt(loc).with_location(loc),
            ];

            Expr::statement_expression(body, ret_type).with_location(loc)
        } else {
            // Compare data pointer.
            let left_ptr = self.codegen_operand(left_op);
            let left_data = left_ptr.clone().member("data", &self.symbol_table);
            let right_ptr = self.codegen_operand(right_op);
            let right_data = right_ptr.clone().member("data", &self.symbol_table);
            let data_cmp = comparison_expr(op, left_data.clone(), right_data.clone(), false);

            // Compare the slice metadata (this logic could be adapted to compare vtable if needed).
            let left_len = left_ptr.member("len", &self.symbol_table);
            let right_len = right_ptr.member("len", &self.symbol_table);
            let metadata_cmp = comparison_expr(op, left_len, right_len, false);

            // Join the results.
            // https://github.com/rust-lang/rust/pull/29781
            match op {
                // Only equal if both parts are equal.
                BinOp::Eq => data_cmp.and(metadata_cmp),
                // It is different if any is different.
                BinOp::Ne => data_cmp.or(metadata_cmp),
                // If data is different, only compare data.
                // If data is equal, apply operator to metadata.
                BinOp::Lt | BinOp::Le | BinOp::Ge | BinOp::Gt => {
                    let data_eq =
                        comparison_expr(&BinOp::Eq, left_data.clone(), right_data.clone(), false);
                    let data_strict_comp =
                        comparison_expr(&get_strict_operator(op), left_data, right_data, false);
                    data_strict_comp.or(data_eq.and(metadata_cmp))
                }
                _ => unreachable!("Unexpected operator {:?}", op),
            }
        }
    }

    fn codegen_unchecked_scalar_binop(
        &mut self,
        op: &BinOp,
        e1: &Operand<'tcx>,
        e2: &Operand<'tcx>,
    ) -> Expr {
        let ce1 = self.codegen_operand(e1);
        let ce2 = self.codegen_operand(e2);
        match op {
            BinOp::BitAnd => ce1.bitand(ce2),
            BinOp::BitOr => ce1.bitor(ce2),
            BinOp::BitXor => ce1.bitxor(ce2),
            BinOp::Div => ce1.div(ce2),
            BinOp::Rem => ce1.rem(ce2),
            BinOp::ShlUnchecked => ce1.shl(ce2),
            BinOp::ShrUnchecked => {
                if self.operand_ty(e1).is_signed() {
                    ce1.ashr(ce2)
                } else {
                    ce1.lshr(ce2)
                }
            }
            _ => unreachable!("Unexpected {:?}", op),
        }
    }

    fn codegen_scalar_binop(&mut self, op: &BinOp, e1: &Operand<'tcx>, e2: &Operand<'tcx>) -> Expr {
        let ce1 = self.codegen_operand(e1);
        let ce2 = self.codegen_operand(e2);
        match op {
            BinOp::Add => ce1.plus(ce2),
            BinOp::Sub => ce1.sub(ce2),
            BinOp::Mul => ce1.mul(ce2),
            BinOp::Shl => ce1.shl(ce2),
            BinOp::Shr => {
                if self.operand_ty(e1).is_signed() {
                    ce1.ashr(ce2)
                } else {
                    ce1.lshr(ce2)
                }
            }
            _ => unreachable!(),
        }
    }

    /// Codegens expressions of the type `let a  = [4u8; 6];`
    fn codegen_rvalue_repeat(
        &mut self,
        op: &Operand<'tcx>,
        sz: ty::Const<'tcx>,
        loc: Location,
    ) -> Expr {
        let op_expr = self.codegen_operand(op);
        let width = sz.try_eval_target_usize(self.tcx, ty::ParamEnv::reveal_all()).unwrap();
        op_expr.array_constant(width).with_location(loc)
    }

    fn codegen_rvalue_len(&mut self, p: &Place<'tcx>) -> Expr {
        let pt = self.place_ty(p);
        match pt.kind() {
            ty::Array(_, sz) => self.codegen_const_internal(*sz, None),
            ty::Slice(_) => unwrap_or_return_codegen_unimplemented!(self, self.codegen_place(p))
                .fat_ptr_goto_expr
                .unwrap()
                .member("len", &self.symbol_table),
            _ => unreachable!("Len(_) called on type that has no length: {:?}", pt),
        }
    }

    /// Generate code for a binary operation with an overflow check.
    fn codegen_binop_with_overflow_check(
        &mut self,
        op: &BinOp,
        left_op: &Operand<'tcx>,
        right_op: &Operand<'tcx>,
        loc: Location,
    ) -> Expr {
        debug!(?op, "codegen_binop_with_overflow_check");
        let left = self.codegen_operand(left_op);
        let right = self.codegen_operand(right_op);
        let ret_type = left.typ().clone();
        let (bin_op, op_name) = match op {
            BinOp::AddUnchecked => (BinaryOperator::OverflowResultPlus, "unchecked_add"),
            BinOp::SubUnchecked => (BinaryOperator::OverflowResultMinus, "unchecked_sub"),
            BinOp::MulUnchecked => (BinaryOperator::OverflowResultMult, "unchecked_mul"),
            _ => unreachable!("Expected Add/Sub/Mul but got {op:?}"),
        };
        // Create CBMC result type and add to the symbol table.
        let res_type = arithmetic_overflow_result_type(left.typ().clone());
        let tag = res_type.tag().unwrap();
        let struct_tag =
            self.ensure_struct(tag, tag, |_, _| res_type.components().unwrap().clone());
        let res = left.overflow_op(bin_op, right);
        // store the result in a temporary variable
        let (var, decl) = self.decl_temp_variable(struct_tag, Some(res), loc);
        // cast into result type
        let check = self.codegen_assert(
            var.clone()
                .member(ARITH_OVERFLOW_OVERFLOWED_FIELD, &self.symbol_table)
                .cast_to(Type::c_bool())
                .not(),
            PropertyClass::ArithmeticOverflow,
            format!("attempt to compute `{op_name}` which would overflow").as_str(),
            loc,
        );
        Expr::statement_expression(
            vec![
                decl,
                check,
                var.member(ARITH_OVERFLOW_RESULT_FIELD, &self.symbol_table).as_stmt(loc),
            ],
            ret_type,
        )
    }

    /// Generate code for a binary operation with an overflow and returns a tuple (res, overflow).
    pub fn codegen_binop_with_overflow(
        &mut self,
        bin_op: BinaryOperator,
        left: Expr,
        right: Expr,
        expected_typ: Type,
        loc: Location,
    ) -> Expr {
        // Create CBMC result type and add to the symbol table.
        let res_type = arithmetic_overflow_result_type(left.typ().clone());
        let tag = res_type.tag().unwrap();
        let struct_tag =
            self.ensure_struct(tag, tag, |_, _| res_type.components().unwrap().clone());
        let res = left.overflow_op(bin_op, right);
        // store the result in a temporary variable
        let (var, decl) = self.decl_temp_variable(struct_tag, Some(res), loc);
        // cast into result type
        let cast = Expr::struct_expr_from_values(
            expected_typ.clone(),
            vec![
                var.clone().member(ARITH_OVERFLOW_RESULT_FIELD, &self.symbol_table),
                var.member(ARITH_OVERFLOW_OVERFLOWED_FIELD, &self.symbol_table)
                    .cast_to(Type::c_bool()),
            ],
            &self.symbol_table,
        );
        Expr::statement_expression(vec![decl, cast.as_stmt(loc)], expected_typ)
    }

    /// Generate code for a binary arithmetic operation with UB / overflow checks in place.
    fn codegen_rvalue_checked_binary_op(
        &mut self,
        op: &BinOp,
        e1: &Operand<'tcx>,
        e2: &Operand<'tcx>,
        res_ty: Ty<'tcx>,
    ) -> Expr {
        let ce1 = self.codegen_operand(e1);
        let ce2 = self.codegen_operand(e2);

        fn shift_max(t: Ty<'_>, mm: &MachineModel) -> Expr {
            match t.kind() {
                ty::Int(k) => match k {
                    IntTy::I8 => Expr::int_constant(7, Type::signed_int(8)),
                    IntTy::I16 => Expr::int_constant(15, Type::signed_int(16)),
                    IntTy::I32 => Expr::int_constant(31, Type::signed_int(32)),
                    IntTy::I64 => Expr::int_constant(63, Type::signed_int(64)),
                    IntTy::I128 => Expr::int_constant(127, Type::signed_int(128)),
                    IntTy::Isize => Expr::int_constant(mm.pointer_width - 1, Type::ssize_t()),
                },
                ty::Uint(k) => match k {
                    UintTy::U8 => Expr::int_constant(7, Type::unsigned_int(8)),
                    UintTy::U16 => Expr::int_constant(15, Type::unsigned_int(16)),
                    UintTy::U32 => Expr::int_constant(31, Type::unsigned_int(32)),
                    UintTy::U64 => Expr::int_constant(63, Type::unsigned_int(64)),
                    UintTy::U128 => Expr::int_constant(127, Type::unsigned_int(128)),
                    UintTy::Usize => Expr::int_constant(mm.pointer_width - 1, Type::size_t()),
                },
                _ => unreachable!(),
            }
        }

        match op {
            BinOp::Add => {
                let res_type = self.codegen_ty(res_ty);
                self.codegen_binop_with_overflow(
                    BinaryOperator::OverflowResultPlus,
                    ce1,
                    ce2,
                    res_type,
                    Location::None,
                )
            }
            BinOp::Sub => {
                let res_type = self.codegen_ty(res_ty);
                self.codegen_binop_with_overflow(
                    BinaryOperator::OverflowResultMinus,
                    ce1,
                    ce2,
                    res_type,
                    Location::None,
                )
            }
            BinOp::Mul => {
                let res_type = self.codegen_ty(res_ty);
                self.codegen_binop_with_overflow(
                    BinaryOperator::OverflowResultMult,
                    ce1,
                    ce2,
                    res_type,
                    Location::None,
                )
            }
            BinOp::Shl => {
                let t1 = self.operand_ty(e1);
                let max = shift_max(t1, self.symbol_table.machine_model());
                Expr::struct_expr_from_values(
                    self.codegen_ty(res_ty),
                    vec![
                        ce1.shl(ce2.clone()),
                        ce2.cast_to(self.codegen_ty(t1)).gt(max).cast_to(Type::c_bool()),
                    ],
                    &self.symbol_table,
                )
            }
            BinOp::Shr => {
                let t1 = self.operand_ty(e1);
                let max = shift_max(t1, self.symbol_table.machine_model());
                Expr::struct_expr_from_values(
                    self.codegen_ty(res_ty),
                    vec![
                        if t1.is_signed() { ce1.ashr(ce2.clone()) } else { ce1.lshr(ce2.clone()) },
                        ce2.cast_to(self.codegen_ty(t1)).gt(max).cast_to(Type::c_bool()),
                    ],
                    &self.symbol_table,
                )
            }
            _ => unreachable!(),
        }
    }

    fn codegen_rvalue_binary_op(
        &mut self,
        ty: Ty<'tcx>,
        op: &BinOp,
        e1: &Operand<'tcx>,
        e2: &Operand<'tcx>,
        loc: Location,
    ) -> Expr {
        match op {
            BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Shl | BinOp::Shr => {
                self.codegen_scalar_binop(op, e1, e2)
            }
            BinOp::ShlUnchecked | BinOp::ShrUnchecked => {
                let result = self.codegen_unchecked_scalar_binop(op, e1, e2);
                let check = self.check_unchecked_shift_distance(e1, e2, loc);
                Expr::statement_expression(
                    vec![check, result.clone().as_stmt(loc)],
                    result.typ().clone(),
                )
            }
            BinOp::AddUnchecked | BinOp::MulUnchecked | BinOp::SubUnchecked => {
                self.codegen_binop_with_overflow_check(op, e1, e2, loc)
            }
            BinOp::Div | BinOp::Rem => {
                let result = self.codegen_unchecked_scalar_binop(op, e1, e2);
                if self.operand_ty(e1).is_integral() {
                    let is_rem = matches!(op, BinOp::Rem);
                    let check = self.check_div_overflow(e1, e2, is_rem, loc);
                    Expr::statement_expression(
                        vec![check, result.clone().as_stmt(loc)],
                        result.typ().clone(),
                    )
                } else {
                    result
                }
            }
            BinOp::BitXor | BinOp::BitAnd | BinOp::BitOr => {
                self.codegen_unchecked_scalar_binop(op, e1, e2)
            }
            BinOp::Eq | BinOp::Lt | BinOp::Le | BinOp::Ne | BinOp::Ge | BinOp::Gt => {
                if self.is_fat_pointer(self.operand_ty(e1)) {
                    self.codegen_comparison_fat_ptr(op, e1, e2, loc)
                } else {
                    self.codegen_comparison(op, e1, e2)
                }
            }
            // https://doc.rust-lang.org/std/primitive.pointer.html#method.offset
            BinOp::Offset => {
                let ce1 = self.codegen_operand(e1);
                let ce2 = self.codegen_operand(e2);

                // Check that computing `offset` in bytes would not overflow
                let (offset_bytes, bytes_overflow_check) = self.count_in_bytes(
                    ce2.clone().cast_to(Type::ssize_t()),
                    ty,
                    Type::ssize_t(),
                    "offset",
                    loc,
                );

                // Check that the computation would not overflow an `isize` which is UB:
                // https://doc.rust-lang.org/std/primitive.pointer.html#method.offset
                // These checks may allow a wrapping-around behavior in CBMC:
                // https://github.com/model-checking/kani/issues/1150
                let overflow_res = ce1.clone().cast_to(Type::ssize_t()).add_overflow(offset_bytes);
                let overflow_check = self.codegen_assert_assume(
                    overflow_res.overflowed.not(),
                    PropertyClass::ArithmeticOverflow,
                    "attempt to compute offset which would overflow",
                    loc,
                );
                let res = ce1.clone().plus(ce2);
                Expr::statement_expression(
                    vec![bytes_overflow_check, overflow_check, res.as_stmt(loc)],
                    ce1.typ().clone(),
                )
            }
        }
    }

    /// Check that a division does not overflow.
    /// For integer types, division by zero is UB, as is MIN / -1 for signed.
    /// Note that the compiler already inserts these checks for regular division.
    /// However, since <https://github.com/rust-lang/rust/pull/112168>, unchecked divisions are
    /// lowered to `BinOp::Div`. Prefer adding duplicated checks for now.
    fn check_div_overflow(
        &mut self,
        dividend: &Operand<'tcx>,
        divisor: &Operand<'tcx>,
        is_remainder: bool,
        loc: Location,
    ) -> Stmt {
        let divisor_expr = self.codegen_operand(divisor);
        let msg = if is_remainder {
            "attempt to calculate the remainder with a divisor of zero"
        } else {
            "attempt to divide by zero"
        };
        let div_by_zero_check = self.codegen_assert_assume(
            divisor_expr.clone().is_zero().not(),
            PropertyClass::ArithmeticOverflow,
            msg,
            loc,
        );
        if self.operand_ty(dividend).is_signed() {
            let dividend_expr = self.codegen_operand(dividend);
            let overflow_msg = if is_remainder {
                "attempt to calculate the remainder with overflow"
            } else {
                "attempt to divide with overflow"
            };
            let overflow_expr = dividend_expr
                .clone()
                .eq(dividend_expr.typ().min_int_expr(self.symbol_table.machine_model()))
                .and(divisor_expr.clone().eq(Expr::int_constant(-1, divisor_expr.typ().clone())));
            let overflow_check = self.codegen_assert_assume(
                overflow_expr.not(),
                PropertyClass::ArithmeticOverflow,
                overflow_msg,
                loc,
            );
            Stmt::block(vec![overflow_check, div_by_zero_check], loc)
        } else {
            div_by_zero_check
        }
    }

    /// Check for valid unchecked shift distance.
    /// Shifts on an integer of type T are UB if shift distance < 0 or >= T::BITS.
    fn check_unchecked_shift_distance(
        &mut self,
        value: &Operand<'tcx>,
        distance: &Operand<'tcx>,
        loc: Location,
    ) -> Stmt {
        let value_expr = self.codegen_operand(value);
        let distance_expr = self.codegen_operand(distance);
        let value_width = value_expr.typ().sizeof_in_bits(&self.symbol_table);
        let value_width_expr = Expr::int_constant(value_width, distance_expr.typ().clone());

        let excessive_distance_check = self.codegen_assert_assume(
            distance_expr.clone().lt(value_width_expr),
            PropertyClass::ArithmeticOverflow,
            "attempt to shift by excessive shift distance",
            loc,
        );

        if distance_expr.typ().is_signed(self.symbol_table.machine_model()) {
            let negative_distance_check = self.codegen_assert_assume(
                distance_expr.is_non_negative(),
                PropertyClass::ArithmeticOverflow,
                "attempt to shift by negative distance",
                loc,
            );
            Stmt::block(vec![negative_distance_check, excessive_distance_check], loc)
        } else {
            excessive_distance_check
        }
    }

    /// Create an initializer for a coroutine struct.
    fn codegen_rvalue_coroutine(
        &mut self,
        operands: &IndexVec<FieldIdx, Operand<'tcx>>,
        ty: Ty<'tcx>,
    ) -> Expr {
        let layout = self.layout_of(ty);
        let discriminant_field = match &layout.variants {
            Variants::Multiple { tag_encoding: TagEncoding::Direct, tag_field, .. } => tag_field,
            _ => unreachable!(
                "Expected coroutines to have multiple variants and direct encoding, but found: {layout:?}"
            ),
        };
        let overall_t = self.codegen_ty(ty);
        let direct_fields = overall_t.lookup_field("direct_fields", &self.symbol_table).unwrap();
        let direct_fields_expr = Expr::struct_expr_from_values(
            direct_fields.typ(),
            layout
                .fields
                .index_by_increasing_offset()
                .map(|idx| {
                    let field_ty = layout.field(self, idx).ty;
                    if idx == *discriminant_field {
                        Expr::int_constant(0, self.codegen_ty(field_ty))
                    } else {
                        self.codegen_operand(&operands[idx.into()])
                    }
                })
                .collect(),
            &self.symbol_table,
        );
        Expr::union_expr(overall_t, "direct_fields", direct_fields_expr, &self.symbol_table)
    }

    /// This code will generate an expression that initializes an enumeration.
    ///
    /// It will first create a temporary variant with the same enum type.
    /// Initialize the case structure and set its discriminant.
    /// Finally, it will return the temporary value.
    fn codegen_rvalue_enum_aggregate(
        &mut self,
        variant_index: VariantIdx,
        operands: &IndexVec<FieldIdx, Operand<'tcx>>,
        res_ty: Ty<'tcx>,
        loc: Location,
    ) -> Expr {
        let mut stmts = vec![];
        let typ = self.codegen_ty(res_ty);
        // 1- Create a temporary value of the enum type.
        tracing::debug!(?typ, ?res_ty, "aggregate_enum");
        let (temp_var, decl) = self.decl_temp_variable(typ.clone(), None, loc);
        stmts.push(decl);
        if !operands.is_empty() {
            // 2- Initialize the members of the temporary variant.
            let initial_projection =
                ProjectedPlace::try_new_internal(temp_var.clone(), res_ty, self).unwrap();
            let variant_proj = self
                .codegen_variant_lvalue(initial_projection, rustc_internal::stable(variant_index));
            let variant_expr = variant_proj.goto_expr.clone();
            let layout = self.layout_of(res_ty);
            let fields = match &layout.variants {
                Variants::Single { index } => {
                    if *index != variant_index {
                        // This may occur if all variants except for the one pointed by
                        // index can never be constructed. Generic code might still try
                        // to initialize the non-existing invariant.
                        trace!(?res_ty, ?variant_index, "Unreachable invariant");
                        return Expr::nondet(typ);
                    }
                    &layout.fields
                }
                Variants::Multiple { variants, .. } => &variants[variant_index].fields,
            };

            trace!(?variant_expr, ?fields, ?operands, "codegen_aggregate enum");
            let init_struct = Expr::struct_expr_from_values(
                variant_expr.typ().clone(),
                fields
                    .index_by_increasing_offset()
                    .map(|idx| self.codegen_operand(&operands[idx.into()]))
                    .collect(),
                &self.symbol_table,
            );
            let assign_case = variant_proj.goto_expr.assign(init_struct, loc);
            stmts.push(assign_case);
        }
        // 3- Set discriminant.
        let set_discriminant =
            self.codegen_set_discriminant(res_ty, temp_var.clone(), variant_index, loc);
        stmts.push(set_discriminant);
        // 4- Return temporary variable.
        stmts.push(temp_var.as_stmt(loc));
        Expr::statement_expression(stmts, typ)
    }

    fn codegen_rvalue_aggregate(
        &mut self,
        aggregate: &AggregateKind<'tcx>,
        operands: &IndexVec<FieldIdx, Operand<'tcx>>,
        res_ty: Ty<'tcx>,
        loc: Location,
    ) -> Expr {
        match *aggregate {
            AggregateKind::Array(_et) => {
                let typ = self.codegen_ty(res_ty);
                Expr::array_expr(typ, operands.iter().map(|o| self.codegen_operand(o)).collect())
            }
            AggregateKind::Adt(_, _, _, _, Some(active_field_index)) => {
                assert!(res_ty.is_union());
                assert_eq!(operands.len(), 1);
                let typ = self.codegen_ty(res_ty);
                let components = typ.lookup_components(&self.symbol_table).unwrap();
                Expr::union_expr(
                    typ,
                    components[active_field_index.as_usize()].name(),
                    self.codegen_operand(&operands[0usize.into()]),
                    &self.symbol_table,
                )
            }
            AggregateKind::Adt(_, _, _, _, _) if res_ty.is_simd() => {
                let typ = self.codegen_ty(res_ty);
                let layout = self.layout_of(res_ty);
                trace!(shape=?layout.fields, "codegen_rvalue_aggregate");
                assert!(operands.len() > 0, "SIMD vector cannot be empty");
                if operands.len() == 1 {
                    let data = self.codegen_operand(&operands[0u32.into()]);
                    if data.typ().is_array() {
                        // Array-based SIMD representation.
                        data.transmute_to(typ, &self.symbol_table)
                    } else {
                        // Multi field-based representation with one field.
                        Expr::vector_expr(typ, vec![data])
                    }
                } else {
                    // Multi field SIMD representation.
                    Expr::vector_expr(
                        typ,
                        layout
                            .fields
                            .index_by_increasing_offset()
                            .map(|idx| self.codegen_operand(&operands[idx.into()]))
                            .collect(),
                    )
                }
            }
            AggregateKind::Adt(_, variant_index, ..) if res_ty.is_enum() => {
                self.codegen_rvalue_enum_aggregate(variant_index, operands, res_ty, loc)
            }
            AggregateKind::Adt(..) | AggregateKind::Closure(..) | AggregateKind::Tuple => {
                let typ = self.codegen_ty(res_ty);
                let layout = self.layout_of(res_ty);
                Expr::struct_expr_from_values(
                    typ,
                    layout
                        .fields
                        .index_by_increasing_offset()
                        .map(|idx| self.codegen_operand(&operands[idx.into()]))
                        .collect(),
                    &self.symbol_table,
                )
            }
            AggregateKind::Coroutine(_, _, _) => self.codegen_rvalue_coroutine(&operands, res_ty),
        }
    }

    pub fn codegen_rvalue(&mut self, rv: &Rvalue<'tcx>, loc: Location) -> Expr {
        let res_ty = self.rvalue_ty(rv);
        debug!(?rv, "codegen_rvalue");
        match rv {
            Rvalue::Use(p) => self.codegen_operand(p),
            Rvalue::Repeat(op, sz) => {
                let sz = self.monomorphize(*sz);
                self.codegen_rvalue_repeat(op, sz, loc)
            }
            Rvalue::Ref(_, _, p) | Rvalue::AddressOf(_, p) => self.codegen_place_ref(p),
            Rvalue::Len(p) => self.codegen_rvalue_len(p),
            // Rust has begun distinguishing "ptr -> num" and "num -> ptr" (providence-relevant casts) but we do not yet:
            // Should we? Tracking ticket: https://github.com/model-checking/kani/issues/1274
            Rvalue::Cast(
                CastKind::IntToInt
                | CastKind::FloatToFloat
                | CastKind::FloatToInt
                | CastKind::IntToFloat
                | CastKind::FnPtrToPtr
                | CastKind::PtrToPtr
                | CastKind::PointerExposeAddress
                | CastKind::PointerFromExposedAddress,
                e,
                t,
            ) => {
                let t = self.monomorphize(*t);
                self.codegen_misc_cast(e, t)
            }
            Rvalue::Cast(CastKind::DynStar, _, _) => {
                let ty = self.codegen_ty(res_ty);
                self.codegen_unimplemented_expr(
                    "CastKind::DynStar",
                    ty,
                    loc,
                    "https://github.com/model-checking/kani/issues/1784",
                )
            }
            Rvalue::Cast(CastKind::PointerCoercion(k), e, t) => {
                let t = self.monomorphize(*t);
                self.codegen_pointer_cast(k, e, t, loc)
            }
            Rvalue::Cast(CastKind::Transmute, operand, ty) => {
                let goto_typ = self.codegen_ty(self.monomorphize(*ty));
                self.codegen_operand(operand).transmute_to(goto_typ, &self.symbol_table)
            }
            Rvalue::BinaryOp(op, box (ref e1, ref e2)) => {
                self.codegen_rvalue_binary_op(res_ty, op, e1, e2, loc)
            }
            Rvalue::CheckedBinaryOp(op, box (ref e1, ref e2)) => {
                self.codegen_rvalue_checked_binary_op(op, e1, e2, res_ty)
            }
            Rvalue::NullaryOp(k, t) => {
                let t = self.monomorphize(*t);
                let layout = self.layout_of(t);
                match k {
                    NullOp::SizeOf => Expr::int_constant(layout.size.bytes_usize(), Type::size_t())
                        .with_size_of_annotation(self.codegen_ty(t)),
                    NullOp::AlignOf => Expr::int_constant(layout.align.abi.bytes(), Type::size_t()),
                    NullOp::OffsetOf(fields) => Expr::int_constant(
                        layout.offset_of_subfield(self, fields.iter()).bytes(),
                        Type::size_t(),
                    ),
                }
            }
            Rvalue::ShallowInitBox(ref operand, content_ty) => {
                // The behaviour of ShallowInitBox is simply transmuting *mut u8 to Box<T>.
                // See https://github.com/rust-lang/compiler-team/issues/460 for more details.
                let operand = self.codegen_operand(operand);
                let t = self.monomorphize(*content_ty);
                let box_ty = Ty::new_box(self.tcx, t);
                let box_ty = self.codegen_ty(box_ty);
                let cbmc_t = self.codegen_ty(t);
                let box_contents = operand.cast_to(cbmc_t.to_pointer());
                self.box_value(box_contents, box_ty)
            }
            Rvalue::UnaryOp(op, e) => match op {
                UnOp::Not => {
                    if self.operand_ty(e).is_bool() {
                        self.codegen_operand(e).not()
                    } else {
                        self.codegen_operand(e).bitnot()
                    }
                }
                UnOp::Neg => self.codegen_operand(e).neg(),
            },
            Rvalue::Discriminant(p) => {
                let place =
                    unwrap_or_return_codegen_unimplemented!(self, self.codegen_place(p)).goto_expr;
                let pt = self.place_ty(p);
                self.codegen_get_discriminant(place, pt, res_ty)
            }
            Rvalue::Aggregate(ref k, operands) => {
                self.codegen_rvalue_aggregate(k, operands, res_ty, loc)
            }
            Rvalue::ThreadLocalRef(def_id) => {
                // Since Kani is single-threaded, we treat a thread local like a static variable:
                self.store_concurrent_construct("thread local (replaced by static variable)", loc);
                self.codegen_thread_local_pointer(*def_id)
            }
            // A CopyForDeref is equivalent to a read from a place at the codegen level.
            // https://github.com/rust-lang/rust/blob/1673f1450eeaf4a5452e086db0fe2ae274a0144f/compiler/rustc_middle/src/mir/syntax.rs#L1055
            Rvalue::CopyForDeref(place) => {
                unwrap_or_return_codegen_unimplemented!(self, self.codegen_place(place)).goto_expr
            }
        }
    }

    pub fn codegen_discriminant_field(&self, place: Expr, ty: Ty<'tcx>) -> Expr {
        let layout = self.layout_of(ty);
        assert!(
            matches!(
                &layout.variants,
                Variants::Multiple { tag_encoding: TagEncoding::Direct, .. }
            ),
            "discriminant field (`case`) only exists for multiple variants and direct encoding"
        );
        let expr = if ty.is_coroutine() {
            // Coroutines are translated somewhat differently from enums (see [`GotoCtx::codegen_ty_coroutine`]).
            // As a consequence, the discriminant is accessed as `.direct_fields.case` instead of just `.case`.
            place.member("direct_fields", &self.symbol_table)
        } else {
            place
        };
        expr.member("case", &self.symbol_table)
    }

    /// e: ty
    /// get the discriminant of e, of type res_ty
    pub fn codegen_get_discriminant(&mut self, e: Expr, ty: Ty<'tcx>, res_ty: Ty<'tcx>) -> Expr {
        let layout = self.layout_of(ty);
        match &layout.variants {
            Variants::Single { index } => {
                let discr_val = layout
                    .ty
                    .discriminant_for_variant(self.tcx, *index)
                    .map_or(index.as_u32() as u128, |discr| discr.val);
                Expr::int_constant(discr_val, self.codegen_ty(res_ty))
            }
            Variants::Multiple { tag_encoding, .. } => match tag_encoding {
                TagEncoding::Direct => {
                    self.codegen_discriminant_field(e, ty).cast_to(self.codegen_ty(res_ty))
                }
                TagEncoding::Niche { untagged_variant, niche_variants, niche_start } => {
                    // This code follows the logic in the ssa codegen backend:
                    // https://github.com/rust-lang/rust/blob/fee75fbe11b1fad5d93c723234178b2a329a3c03/compiler/rustc_codegen_ssa/src/mir/place.rs#L247
                    // See also the cranelift backend:
                    // https://github.com/rust-lang/rust/blob/05d22212e89588e7c443cc6b9bc0e4e02fdfbc8d/compiler/rustc_codegen_cranelift/src/discriminant.rs#L116
                    let offset = match &layout.fields {
                        FieldsShape::Arbitrary { offsets, .. } => offsets[0usize.into()],
                        _ => unreachable!("niche encoding must have arbitrary fields"),
                    };

                    // Compute relative discriminant value (`niche_val - niche_start`).
                    //
                    // "We remap `niche_start..=niche_start + n` (which may wrap around) to
                    // (non-wrap-around) `0..=n`, to be able to check whether the discriminant
                    // corresponds to a niche variant with one comparison."
                    // https://github.com/rust-lang/rust/blob/fee75fbe11b1fad5d93c723234178b2a329a3c03/compiler/rustc_codegen_ssa/src/mir/place.rs#L247
                    //
                    // Note: niche_variants can only represent values that fit in a u32.
                    let result_type = self.codegen_ty(res_ty);
                    let discr_mir_ty = self.codegen_enum_discr_typ(ty);
                    let discr_type = self.codegen_ty(discr_mir_ty);
                    let niche_val = self.codegen_get_niche(e, offset, discr_type);
                    let relative_discr =
                        wrapping_sub(&niche_val, u64::try_from(*niche_start).unwrap());
                    let relative_max =
                        niche_variants.end().as_u32() - niche_variants.start().as_u32();
                    let is_niche = if relative_max == 0 {
                        relative_discr.clone().is_zero()
                    } else {
                        relative_discr
                            .clone()
                            .le(Expr::int_constant(relative_max, relative_discr.typ().clone()))
                    };
                    let niche_discr = {
                        let relative_discr = if relative_max == 0 {
                            result_type.zero()
                        } else {
                            relative_discr.cast_to(result_type.clone())
                        };
                        relative_discr.plus(Expr::int_constant(
                            niche_variants.start().as_u32(),
                            result_type.clone(),
                        ))
                    };
                    is_niche.ternary(
                        niche_discr,
                        Expr::int_constant(untagged_variant.as_u32(), result_type),
                    )
                }
            },
        }
    }

    /// Extract the niche value from `v`. This value should be of type `niche_ty` and located
    /// at byte offset `offset`
    pub fn codegen_get_niche(&self, v: Expr, offset: Size, niche_ty: Type) -> Expr {
        if offset == Size::ZERO {
            v.reinterpret_cast(niche_ty)
        } else {
            v // t: T
                .address_of() // &t: T*
                .cast_to(Type::unsigned_int(8).to_pointer()) // (u8 *)&t: u8 *
                .plus(Expr::int_constant(offset.bytes(), Type::size_t())) // ((u8 *)&t) + offset: u8 *
                .cast_to(niche_ty.to_pointer()) // (N *)(((u8 *)&t) + offset): N *
                .dereference() // *(N *)(((u8 *)&t) + offset): N
        }
    }

    fn codegen_fat_ptr_to_fat_ptr_cast(&mut self, src: &Operand<'tcx>, dst_t: Ty<'tcx>) -> Expr {
        debug!("codegen_fat_ptr_to_fat_ptr_cast |{:?}| |{:?}|", src, dst_t);
        let src_goto_expr = self.codegen_operand(src);
        let dst_goto_typ = self.codegen_ty(dst_t);
        let dst_data_type = dst_goto_typ.lookup_field_type("data", &self.symbol_table).unwrap();
        let dst_data_field = (
            "data",
            src_goto_expr.clone().member("data", &self.symbol_table).cast_to(dst_data_type),
        );

        let dst_metadata_field = if let Some(vtable_typ) =
            dst_goto_typ.lookup_field_type("vtable", &self.symbol_table)
        {
            ("vtable", src_goto_expr.member("vtable", &self.symbol_table).cast_to(vtable_typ))
        } else if let Some(len_typ) = dst_goto_typ.lookup_field_type("len", &self.symbol_table) {
            ("len", src_goto_expr.member("len", &self.symbol_table).cast_to(len_typ))
        } else {
            unreachable!("fat pointer with neither vtable nor len. {:?} {:?}", src, dst_t);
        };
        Expr::struct_expr(
            dst_goto_typ,
            btree_string_map![dst_data_field, dst_metadata_field],
            &self.symbol_table,
        )
    }

    fn codegen_fat_ptr_to_thin_ptr_cast(&mut self, src: &Operand<'tcx>, dst_t: Ty<'tcx>) -> Expr {
        debug!("codegen_fat_ptr_to_thin_ptr_cast |{:?}| |{:?}|", src, dst_t);
        let src_goto_expr = self.codegen_operand(src);
        let dst_goto_typ = self.codegen_ty(dst_t);
        // In a vtable fat pointer, the data member is a void pointer,
        // so ensure the pointer has the correct type before dereferencing it.
        src_goto_expr.member("data", &self.symbol_table).cast_to(dst_goto_typ)
    }

    /// This handles all kinds of casts, except a limited subset that are instead
    /// handled by [`Self::codegen_pointer_cast`].
    fn codegen_misc_cast(&mut self, src: &Operand<'tcx>, dst_t: Ty<'tcx>) -> Expr {
        let src_t = self.operand_ty(src);
        debug!(
            "codegen_misc_cast: casting operand {:?} from type {:?} to type {:?}",
            src, src_t, dst_t
        );

        // number casting
        if src_t.is_numeric() && dst_t.is_numeric() {
            return self.codegen_operand(src).cast_to(self.codegen_ty(dst_t));
        }

        // Behind the scenes, char is just a 32bit integer
        if (src_t.is_integral() && dst_t.is_char()) || (src_t.is_char() && dst_t.is_integral()) {
            return self.codegen_operand(src).cast_to(self.codegen_ty(dst_t));
        }

        // Cast an enum to its discriminant
        if src_t.is_enum() && dst_t.is_integral() {
            let operand = self.codegen_operand(src);
            return self.codegen_get_discriminant(operand, src_t, dst_t);
        }

        // Cast between fat pointers
        if self.is_fat_pointer(src_t) && self.is_fat_pointer(dst_t) {
            return self.codegen_fat_ptr_to_fat_ptr_cast(src, dst_t);
        }

        if self.is_fat_pointer(src_t) && !self.is_fat_pointer(dst_t) {
            return self.codegen_fat_ptr_to_thin_ptr_cast(src, dst_t);
        }

        // pointer casting. from a pointer / reference to another pointer / reference
        // notice that if fat pointer is involved, it cannot be the destination, which is t.
        match dst_t.kind() {
            ty::Ref(_, mut dst_subt, _) | ty::RawPtr(ty::TypeAndMut { ty: mut dst_subt, .. }) => {
                // this is a noop in the case dst_subt is a Projection or Opaque type
                dst_subt = self.tcx.normalize_erasing_regions(ty::ParamEnv::reveal_all(), dst_subt);
                match dst_subt.kind() {
                    ty::Slice(_) | ty::Str | ty::Dynamic(_, _, _) => {
                        //TODO: this does the wrong thing on Strings/fixme_boxed_str.rs
                        // if we cast to slice or string, then we know the source is also a slice or string,
                        // so there shouldn't be anything to do
                        //DSN The one time I've seen this for dynamic, it was just casting from const* to mut*
                        // TODO: see if it is accurate
                        self.codegen_operand(src)
                    }
                    _ => match src_t.kind() {
                        ty::Ref(_, mut src_subt, _)
                        | ty::RawPtr(ty::TypeAndMut { ty: mut src_subt, .. }) => {
                            // this is a noop in the case dst_subt is a Projection or Opaque type
                            src_subt = self
                                .tcx
                                .normalize_erasing_regions(ty::ParamEnv::reveal_all(), src_subt);
                            match src_subt.kind() {
                                ty::Slice(_) | ty::Str | ty::Dynamic(..) => self
                                    .codegen_operand(src)
                                    .member("data", &self.symbol_table)
                                    .cast_to(self.codegen_ty(dst_t)),
                                _ => self.codegen_operand(src).cast_to(self.codegen_ty(dst_t)),
                            }
                        }
                        ty::Int(_) | ty::Uint(_) | ty::FnPtr(..) => {
                            self.codegen_operand(src).cast_to(self.codegen_ty(dst_t))
                        }
                        _ => unreachable!(),
                    },
                }
            }
            ty::Int(_) | ty::Uint(_) => self.codegen_operand(src).cast_to(self.codegen_ty(dst_t)),
            _ => unreachable!(),
        }
    }

    /// "Pointer casts" are particular kinds of pointer-to-pointer casts.
    /// See the [`PointerCoercion`] type for specifics.
    /// Note that this does not include all casts involving pointers,
    /// many of which are instead handled by [`Self::codegen_misc_cast`] instead.
    fn codegen_pointer_cast(
        &mut self,
        k: &PointerCoercion,
        operand: &Operand<'tcx>,
        t: Ty<'tcx>,
        loc: Location,
    ) -> Expr {
        debug!(cast=?k, op=?operand, ?loc, "codegen_pointer_cast");
        match k {
            PointerCoercion::ReifyFnPointer => match self.operand_ty(operand).kind() {
                ty::FnDef(def_id, args) => {
                    let instance =
                        Instance::resolve(self.tcx, ty::ParamEnv::reveal_all(), *def_id, args)
                            .unwrap()
                            .unwrap();
                    // We need to handle this case in a special way because `codegen_operand` compiles FnDefs to dummy structs.
                    // (cf. the function documentation)
                    self.codegen_func_expr_internal(instance, None).address_of()
                }
                _ => unreachable!(),
            },
            PointerCoercion::UnsafeFnPointer => self.codegen_operand(operand),
            PointerCoercion::ClosureFnPointer(_) => {
                if let ty::Closure(def_id, args) = self.operand_ty(operand).kind() {
                    let instance =
                        Instance::resolve_closure(self.tcx, *def_id, args, ty::ClosureKind::FnOnce)
                            .expect("failed to normalize and resolve closure during codegen")
                            .polymorphize(self.tcx);
                    self.codegen_func_expr_internal(instance, None).address_of()
                } else {
                    unreachable!("{:?} cannot be cast to a fn ptr", operand)
                }
            }
            PointerCoercion::MutToConstPointer => self.codegen_operand(operand),
            PointerCoercion::ArrayToPointer => {
                // TODO: I am not sure whether it is correct or not.
                //
                // some reasoning is as follows.
                // the trouble is to understand whether we have to handle fat pointers and my claim is no.
                // if we had to, then [o] necessarily has type [T; n] where *T is a fat pointer, meaning
                // T is either [T] or str. but neither type is sized, which shouldn't participate in
                // codegen.
                match self.operand_ty(operand).kind() {
                    ty::RawPtr(ty::TypeAndMut { ty, .. }) => {
                        // ty must be an array
                        if let ty::Array(_, _) = ty.kind() {
                            let oe = self.codegen_operand(operand);
                            oe.dereference() // : struct [T; n]
                                .array_to_ptr() // : T*
                        } else {
                            unreachable!()
                        }
                    }
                    _ => unreachable!(),
                }
            }
            PointerCoercion::Unsize => {
                let src_goto_expr = self.codegen_operand(operand);
                let src_mir_type = self.operand_ty(operand);
                let dst_mir_type = t;
                self.codegen_unsized_cast(src_goto_expr, src_mir_type, dst_mir_type)
            }
        }
    }

    /// Generate code for unsized cast. This includes the following:
    /// -> (Built-in / Smart) Pointer from array to slice.
    /// -> (Built-in / Smart) Pointer from sized type to dyn trait.
    /// -> (Built-in / Smart) Pointer from dyn trait to dyn trait.
    ///     - E.g.: `&(dyn Any + Send)` to `&dyn Any`.
    /// -> All the cases above where the pointer refers to a parametrized struct where the type
    /// parameter is the target of the unsize casting.
    ///     - E.g.: `RcBox<String>` to `RcBox<dyn Any>`
    fn codegen_unsized_cast(
        &mut self,
        src_goto_expr: Expr,
        src_mir_type: Ty<'tcx>,
        dst_mir_type: Ty<'tcx>,
    ) -> Expr {
        // The MIR may include casting that isn't necessary. Detect this early on and return the
        // expression for the RHS.
        if src_mir_type == dst_mir_type {
            return src_goto_expr;
        }

        // Collect some information about the unsized coercion.
        let mut path = self.collect_unsized_cast_path(src_goto_expr, src_mir_type, dst_mir_type);
        debug!(cast=?path, "codegen_unsized_cast");

        // Handle the leaf which should always be a pointer.
        let (ptr_cast_info, ptr_src_expr) = path.pop().unwrap();
        let initial_expr = self.codegen_cast_to_fat_pointer(ptr_src_expr, ptr_cast_info);

        // Iterate from the back of the path initializing each struct that requires the coercion.
        // This code is required for handling smart pointers.
        path.into_iter().rfold(initial_expr, |coercion_expr, (info, src_expr)| {
            self.codegen_struct_unsized_coercion(src_expr, info, coercion_expr)
        })
    }

    /// Extract path that must be explicitly casted. Add to the tuple the expression type for
    /// the current source.
    fn collect_unsized_cast_path(
        &self,
        src_goto_expr: Expr,
        src_mir_type: Ty<'tcx>,
        dst_mir_type: Ty<'tcx>,
    ) -> Vec<(CoerceUnsizedInfo<'tcx>, Expr)> {
        let mut field_type = src_goto_expr;
        CoerceUnsizedIterator::new(self.tcx, src_mir_type, dst_mir_type)
            .map(|info| {
                let expr = if let Some(field_symbol) = info.field {
                    // Generate the expression for the current structure and save the type for
                    // the divergent field.
                    let field_name = field_symbol.as_str().intern();
                    let member_type = field_type.clone().member(field_name, &self.symbol_table);
                    std::mem::replace(&mut field_type, member_type)
                } else {
                    // The end of our traverse. Generate the expression for the current type.
                    field_type.clone()
                };
                (info, expr)
            })
            .collect::<Vec<_>>()
    }

    /// Generate a struct resulting from an unsized coercion.
    /// The resulting expression is basically a field by field assignment to from the
    /// source expression, except for the field being coerced.
    /// Coercion ignores phantom data structures, so do we.
    /// See <https://github.com/rust-lang/rust/issues/26905> for more details.
    fn codegen_struct_unsized_coercion(
        &mut self,
        src_expr: Expr,
        info: CoerceUnsizedInfo<'tcx>,
        member_coercion: Expr,
    ) -> Expr {
        assert!(info.src_ty.is_adt(), "Expected struct. Found {:?}", info.src_ty);
        assert!(info.dst_ty.is_adt(), "Expected struct. Found {:?}", info.dst_ty);
        let dst_goto_type = self.codegen_ty(info.dst_ty);
        let src_field_exprs = src_expr.struct_field_exprs(&self.symbol_table);
        let dst_field_exprs = src_field_exprs
            .into_iter()
            .map(|(key, val)| {
                let new_val = if info.field.unwrap().as_str().intern() == key {
                    // The type being coerced. Use the provided expression.
                    member_coercion.clone()
                } else {
                    let dst_member_type =
                        dst_goto_type.lookup_field_type(key, &self.symbol_table).unwrap();
                    if &dst_member_type != val.typ() {
                        // Phantom data is ignored during a coercion, but it's type may still
                        // change. So we just recreate the empty struct.
                        assert_eq!(dst_member_type.sizeof(&self.symbol_table), 0);
                        Expr::struct_expr(dst_member_type, BTreeMap::new(), &self.symbol_table)
                    } else {
                        // No coercion is required. Just assign dst.field = src.field.
                        val
                    }
                };
                (key, new_val)
            })
            .collect();

        let dst_expr = Expr::struct_expr(dst_goto_type, dst_field_exprs, &self.symbol_table);
        debug!(?dst_expr, "codegen_struct_unsized_coercion");
        dst_expr
    }

    fn codegen_vtable_method_field(
        &mut self,
        instance: Instance<'tcx>,
        t: Ty<'tcx>,
        idx: usize,
    ) -> Expr {
        debug!(?instance, typ=?t, %idx, "codegen_vtable_method_field");
        let vtable_field_name = self.vtable_field_name(idx);
        let vtable_type = Type::struct_tag(self.vtable_name(t));
        let field_type =
            vtable_type.lookup_field_type(vtable_field_name, &self.symbol_table).unwrap();
        debug!(?vtable_field_name, ?vtable_type, "codegen_vtable_method_field");

        // Lookup in the symbol table using the full symbol table name/key
        let fn_name = self.symbol_name(instance);

        if let Some(fn_symbol) = self.symbol_table.lookup(&fn_name) {
            if self.vtable_ctx.emit_vtable_restrictions {
                // Add to the possible method names for this trait type
                self.vtable_ctx.add_possible_method(
                    self.normalized_trait_name(t).into(),
                    idx,
                    fn_name.into(),
                );
            }

            // Create a pointer to the method
            // Note that the method takes a self* as the first argument, but the vtable field type has a void* as the first arg.
            // So we need to cast it at the end.
            debug!(?fn_symbol, fn_typ=?fn_symbol.typ, ?field_type, "codegen_vtable_method_field");
            Expr::symbol_expression(fn_symbol.name, fn_symbol.typ.clone())
                .address_of()
                .cast_to(field_type)
        } else {
            warn!(
                "Unable to find vtable symbol for virtual function {}, attempted lookup for symbol name: {}",
                self.readable_instance_name(instance),
                fn_name,
            );
            field_type.null()
        }
    }

    /// Generate a function pointer to drop_in_place for entry into the vtable
    fn codegen_vtable_drop_in_place(&mut self, ty: Ty<'tcx>, trait_ty: ty::Ty<'tcx>) -> Expr {
        let drop_instance = Instance::resolve_drop_in_place(self.tcx, ty).polymorphize(self.tcx);
        let drop_sym_name: InternedString = self.symbol_name(drop_instance).into();

        // The drop instance has the concrete object type, for consistency with
        // type codegen we need the trait type for the function parameter.
        let trait_fn_ty = self.trait_vtable_drop_type(trait_ty);

        if let Some(drop_sym) = self.symbol_table.lookup(drop_sym_name) {
            if self.vtable_ctx.emit_vtable_restrictions {
                // Add to the possible method names for this trait type
                self.vtable_ctx.add_possible_method(
                    self.normalized_trait_name(trait_ty).into(),
                    VtableCtx::drop_index(),
                    drop_sym_name,
                );
            }

            debug!(?ty, ?trait_ty, "codegen_drop_in_place");
            debug!(?drop_instance, ?trait_fn_ty, "codegen_drop_in_place");
            debug!(drop_sym=?drop_sym.clone().typ, "codegen_drop_in_place");

            Expr::symbol_expression(drop_sym_name, drop_sym.clone().typ)
                .address_of()
                .cast_to(trait_fn_ty)
        } else {
            unreachable!(
                "Missing drop implementation for {}",
                self.readable_instance_name(drop_instance)
            );
        }
    }

    /// The size and alignment for the vtable is of the underlying type.
    /// When we get the size and align of a ty::Ref, the TyCtxt::layout_of
    /// returns the correct size to match rustc vtable values. Checked via
    /// Kani-compile-time and CBMC assertions in check_vtable_size.
    fn codegen_vtable_size_and_align(&mut self, operand_type: Ty<'tcx>) -> (Expr, Expr) {
        debug!("vtable_size_and_align {:?}", operand_type.kind());
        let vtable_layout = self.layout_of(operand_type);
        assert!(!vtable_layout.is_unsized(), "Can't create a vtable for an unsized type");
        let vt_size = Expr::int_constant(vtable_layout.size.bytes(), Type::size_t())
            .with_size_of_annotation(self.codegen_ty(operand_type));
        let vt_align = Expr::int_constant(vtable_layout.align.abi.bytes(), Type::size_t());

        (vt_size, vt_align)
    }

    // Check the size are inserting in to the vtable against two sources of
    // truth: (1) the compile-time rustc sizeof functions, and (2) the CBMC
    //  __CPROVER_OBJECT_SIZE function.
    fn check_vtable_size(&mut self, operand_type: Ty<'tcx>, vt_size: Expr) -> Stmt {
        // Check against the size we get from the layout from the what we
        // get constructing a value of that type
        let ty: Type = self.codegen_ty(operand_type);
        let codegen_size = ty.sizeof(&self.symbol_table);
        assert_eq!(vt_size.int_constant_value().unwrap(), BigInt::from(codegen_size));

        // Insert a CBMC-time size check, roughly:
        //     <Ty> local_temp = nondet();
        //     assert(__CPROVER_OBJECT_SIZE(&local_temp) == vt_size);
        let (temp_var, decl) = self.decl_temp_variable(ty.clone(), None, Location::none());
        let cbmc_size = if ty.is_empty() {
            // CBMC errors on passing a pointer to void to __CPROVER_OBJECT_SIZE.
            // In practice, we have seen this with the Never type, which has size 0:
            // https://play.rust-lang.org/?version=nightly&mode=debug&edition=2018&gist=0f6eef4f6abeb279031444735e73d2e1
            assert!(
                matches!(operand_type.kind(), ty::Never),
                "Expected Never, got: {operand_type:?}"
            );
            Type::size_t().zero()
        } else {
            Expr::object_size(temp_var.address_of())
        };
        let check = Expr::eq(cbmc_size, vt_size);
        let assert_msg =
            format!("Correct CBMC vtable size for {ty:?} (MIR type {:?})", operand_type.kind());
        let size_assert = self.codegen_sanity(check, &assert_msg, Location::none());
        Stmt::block(vec![decl, size_assert], Location::none())
    }

    fn codegen_vtable(&mut self, src_mir_type: Ty<'tcx>, dst_mir_type: Ty<'tcx>) -> Expr {
        let trait_type = match dst_mir_type.kind() {
            // DST is pointer type
            ty::Ref(_, pointee_type, ..) => *pointee_type,
            // DST is box type
            ty::Adt(adt_def, adt_subst) if adt_def.is_box() => {
                adt_subst.first().unwrap().expect_ty()
            }
            // DST is dynamic type
            ty::Dynamic(..) => dst_mir_type,
            _ => unimplemented!("Cannot codegen_vtable for type {:?}", dst_mir_type.kind()),
        };
        assert!(trait_type.is_trait(), "VTable trait type {trait_type} must be a trait type");
        let binders = match trait_type.kind() {
            ty::Dynamic(binders, ..) => binders,
            _ => unimplemented!("Cannot codegen_vtable for type {:?}", dst_mir_type.kind()),
        };

        let src_name = self.ty_mangled_name(src_mir_type);
        // The name needs to be the same as inserted in typ.rs
        let vtable_name = self.vtable_name(trait_type).intern();
        let vtable_impl_name = format!("{vtable_name}_impl_for_{src_name}");

        self.ensure_global_var(
            vtable_impl_name,
            true,
            Type::struct_tag(vtable_name),
            Location::none(),
            |ctx, var| {
                // Build the vtable, using Rust's vtable_entries to determine field order
                let vtable_entries = if let Some(principal) = binders.principal() {
                    let trait_ref_binder = principal.with_self_ty(ctx.tcx, src_mir_type);
                    let trait_ref_binder = ctx.tcx.erase_regions(trait_ref_binder);

                    ctx.tcx.vtable_entries(trait_ref_binder)
                } else {
                    TyCtxt::COMMON_VTABLE_ENTRIES
                };

                let (vt_size, vt_align) = ctx.codegen_vtable_size_and_align(src_mir_type);
                let size_assert = ctx.check_vtable_size(src_mir_type, vt_size.clone());

                let vtable_fields: Vec<Expr> = vtable_entries
                    .iter()
                    .enumerate()
                    .filter_map(|(idx, entry)| match entry {
                        VtblEntry::MetadataDropInPlace => {
                            Some(ctx.codegen_vtable_drop_in_place(src_mir_type, trait_type))
                        }
                        VtblEntry::MetadataSize => Some(vt_size.clone()),
                        VtblEntry::MetadataAlign => Some(vt_align.clone()),
                        VtblEntry::Vacant => None,
                        // TODO: trait upcasting
                        // https://github.com/model-checking/kani/issues/358
                        VtblEntry::TraitVPtr(_trait_ref) => None,
                        VtblEntry::Method(instance) => {
                            Some(ctx.codegen_vtable_method_field(*instance, trait_type, idx))
                        }
                    })
                    .collect();

                let vtable = Expr::struct_expr_from_values(
                    Type::struct_tag(vtable_name),
                    vtable_fields,
                    &ctx.symbol_table,
                );
                let body = var.assign(vtable, Location::none());
                let block = Stmt::block(vec![size_assert, body], Location::none());
                Some(block)
            },
        )
    }

    /// Cast a pointer to a fat pointer.
    /// The fat pointer will have two elements:
    ///  1. `data` which will point to the same address as the source object.
    ///  2. `vtable` | `len` which corresponds to the coercion metadata.
    fn codegen_cast_to_fat_pointer(
        &mut self,
        src_goto_expr: Expr,
        coerce_info: CoerceUnsizedInfo<'tcx>,
    ) -> Expr {
        assert_ne!(coerce_info.src_ty.kind(), coerce_info.dst_ty.kind());

        // The fat pointer type.
        let fat_ptr_type = self.codegen_ty(coerce_info.dst_ty);

        // Extract the type conversion that will require metadata to be saved.
        let CoercionBase { src_ty: metadata_src_type, dst_ty: metadata_dst_type } =
            extract_unsize_casting(self.tcx, coerce_info.src_ty, coerce_info.dst_ty);

        // Extract information about the data pointer.
        let dst_pointee_ty = pointee_type(coerce_info.dst_ty).unwrap();
        let dst_data_type = self.codegen_ty(dst_pointee_ty).to_pointer();

        debug!(?coerce_info, ?metadata_src_type, ?metadata_dst_type, "codegen_thin_to_fat");
        // Generate the metadata and the fat pointer according to the target of this coercion.
        match (metadata_src_type.kind(), metadata_dst_type.kind()) {
            (ty::Array(src_elt_type, src_elt_count), ty::Slice(dst_elt_type)) => {
                // Cast to a slice fat pointer.
                assert_eq!(src_elt_type, dst_elt_type);
                let dst_goto_len = self.codegen_const_internal(*src_elt_count, None);
                let src_pointee_ty = pointee_type(coerce_info.src_ty).unwrap();
                let dst_data_expr = if src_pointee_ty.is_array() {
                    src_goto_expr.cast_to(self.codegen_ty(*src_elt_type).to_pointer())
                } else {
                    // A struct that contains the type being coerced to a slice.
                    // E.g.: Convert Src<[u8; 2]> to Src<[u8]> where struct Src<T> { member: T }
                    src_goto_expr.cast_to(dst_data_type)
                };
                slice_fat_ptr(fat_ptr_type, dst_data_expr, dst_goto_len, &self.symbol_table)
            }
            (ty::Dynamic(..), ty::Dynamic(..)) => {
                // Cast between fat pointers. Cast the data and the source
                let src_data = src_goto_expr.to_owned().member("data", &self.symbol_table);
                let dst_data = src_data.cast_to(dst_data_type);

                // Retrieve the vtable and cast the vtable type.
                let src_vtable = src_goto_expr.member("vtable", &self.symbol_table);
                let vtable_name = self.vtable_name(metadata_dst_type);
                let vtable_ty = Type::struct_tag(vtable_name).to_pointer();
                let dst_vtable = src_vtable.cast_to(vtable_ty);

                // Construct a fat pointer with the same (casted) fields and new type
                dynamic_fat_ptr(fat_ptr_type, dst_data, dst_vtable, &self.symbol_table)
            }
            (_, ty::Dynamic(..)) => {
                // Generate the data and vtable pointer that will be stored in the fat pointer.
                let dst_data_expr = src_goto_expr.cast_to(dst_data_type);
                let vtable = self.codegen_vtable(metadata_src_type, metadata_dst_type);
                let vtable_expr = vtable.address_of();
                dynamic_fat_ptr(fat_ptr_type, dst_data_expr, vtable_expr, &self.symbol_table)
            }
            (src_kind, dst_kind) => {
                unreachable!("Unexpected unsized cast from type {:?} to {:?}", src_kind, dst_kind)
            }
        }
    }
}

/// Perform a wrapping subtraction of an Expr with a constant "expr - constant"
/// where "-" is wrapping subtraction, i.e., the result should be interpreted as
/// an unsigned value (2's complement).
fn wrapping_sub(expr: &Expr, constant: u64) -> Expr {
    let unsigned_expr = if expr.typ().is_pointer() {
        expr.clone()
    } else {
        let unsigned = expr.typ().to_unsigned().unwrap();
        expr.clone().cast_to(unsigned)
    };
    if constant == 0 {
        // No need to subtract.
        // But we still need to make sure we return an unsigned value.
        unsigned_expr
    } else {
        let constant = Expr::int_constant(constant, unsigned_expr.typ().clone());
        unsigned_expr.sub(constant)
    }
}

fn comparison_expr(op: &BinOp, left: Expr, right: Expr, is_float: bool) -> Expr {
    match op {
        BinOp::Eq => {
            if is_float {
                left.feq(right)
            } else {
                left.eq(right)
            }
        }
        BinOp::Lt => left.lt(right),
        BinOp::Le => left.le(right),
        BinOp::Ne => {
            if is_float {
                left.fneq(right)
            } else {
                left.neq(right)
            }
        }
        BinOp::Ge => left.ge(right),
        BinOp::Gt => left.gt(right),
        _ => unreachable!(),
    }
}

/// Remove the equality from an operator. Translates `<=` to `<` and `>=` to `>`
fn get_strict_operator(op: &BinOp) -> BinOp {
    match op {
        BinOp::Le => BinOp::Lt,
        BinOp::Ge => BinOp::Gt,
        _ => *op,
    }
}
