// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
use super::typ::{is_pointer, pointee_type, TypeExt};
use crate::codegen_cprover_gotoc::codegen::PropertyClass;
use crate::codegen_cprover_gotoc::utils::{dynamic_fat_ptr, slice_fat_ptr};
use crate::codegen_cprover_gotoc::{GotocCtx, VtableCtx};
use crate::{emit_concurrency_warning, unwrap_or_return_codegen_unimplemented};
use cbmc::goto_program::{Expr, Location, Stmt, Symbol, Type};
use cbmc::MachineModel;
use cbmc::{btree_string_map, InternString, InternedString};
use num::bigint::BigInt;
use rustc_middle::mir::{AggregateKind, BinOp, CastKind, NullOp, Operand, Place, Rvalue, UnOp};
use rustc_middle::ty::adjustment::PointerCast;
use rustc_middle::ty::layout::LayoutOf;
use rustc_middle::ty::{self, Instance, IntTy, Ty, TyCtxt, UintTy, VtblEntry};
use rustc_target::abi::{FieldsShape, TagEncoding, Variants};
use tracing::{debug, warn};

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
                    format!("Reached unstable vtable comparison '{:?}'", op).as_str(),
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

    /// Given a mir object denoted by a mir place, codegen a pointer to this object.
    fn codegen_rvalue_ref(&mut self, place: &Place<'tcx>, result_mir_type: Ty<'tcx>) -> Expr {
        let place_mir_type = self.place_ty(place);
        let projection = unwrap_or_return_codegen_unimplemented!(self, self.codegen_place(place));

        debug!("codegen_rvalue_ref: place: {:?}", place);
        debug!("codegen_rvalue_ref: place type: {:?}", place_mir_type);
        debug!("codegen_rvalue_ref: place kind: {:?}", place_mir_type.kind());
        debug!("codegen_rvalue_ref: projection: {:?}", projection);

        assert!(
            is_pointer(result_mir_type),
            "Constructing a pointer of the type {:?} to the value of the place {:?}",
            result_mir_type,
            place
        );
        let result_goto_type = self.codegen_ty(result_mir_type);

        // The goto expr for the value of this place
        let place_goto_expr = projection.goto_expr;

        /*
         * Construct a thin pointer to the value of this place
         */

        if self.use_thin_pointer(place_mir_type) {
            return place_goto_expr.address_of();
        }

        /*
         * Construct a fat pointer to the value of this place
         */

        // skip constructing a fat ptr if this place is already one
        if place_goto_expr.typ().is_rust_fat_ptr(&self.symbol_table) {
            return place_goto_expr;
        }

        // In the sequence of projections leading to this place, we dereferenced
        // this fat pointer.
        let intermediate_fat_pointer = projection.fat_ptr_goto_expr.unwrap();

        // The thin pointer in the resulting fat pointer is a pointer to the value
        let thin_pointer = if place_goto_expr.typ().is_pointer() {
            // The value is itself a pointer, just use this pointer
            place_goto_expr
        } else if place_goto_expr.typ().is_array_like() {
            // The value is an array (eg, a flexible struct member), point to the first array element
            place_goto_expr.array_to_ptr()
        } else {
            // The value is of any other type (eg, a struct), just point to it
            place_goto_expr.address_of()
        };

        // The metadata in the resulting fat pointer comes from the intermediate fat pointer
        let metadata = if self.use_slice_fat_pointer(place_mir_type) {
            intermediate_fat_pointer.member("len", &self.symbol_table)
        } else if self.use_vtable_fat_pointer(place_mir_type) {
            intermediate_fat_pointer.member("vtable", &self.symbol_table)
        } else {
            unreachable!()
        };

        if self.use_slice_fat_pointer(place_mir_type) {
            slice_fat_ptr(result_goto_type, thin_pointer, metadata, &self.symbol_table)
        } else if self.use_vtable_fat_pointer(place_mir_type) {
            dynamic_fat_ptr(result_goto_type, thin_pointer, metadata, &self.symbol_table)
        } else {
            unreachable!();
        }
    }

    /// Codegens expressions of the type `let a  = [4u8; 6];`
    fn codegen_rvalue_repeat(
        &mut self,
        op: &Operand<'tcx>,
        sz: &ty::Const<'tcx>,
        res_ty: Ty<'tcx>,
        loc: Location,
    ) -> Expr {
        let res_t = self.codegen_ty(res_ty);
        let op_expr = self.codegen_operand(op);
        let width = sz.try_eval_usize(self.tcx, ty::ParamEnv::reveal_all()).unwrap();
        Expr::struct_expr(
            res_t,
            btree_string_map![("0", op_expr.array_constant(width))],
            &self.symbol_table,
        )
        .with_location(loc)
    }

    fn codegen_rvalue_len(&mut self, p: &Place<'tcx>) -> Expr {
        let pt = self.place_ty(p);
        match pt.kind() {
            ty::Array(_, sz) => self.codegen_const(*sz, None),
            ty::Slice(_) => unwrap_or_return_codegen_unimplemented!(self, self.codegen_place(p))
                .fat_ptr_goto_expr
                .unwrap()
                .member("len", &self.symbol_table),
            _ => unreachable!("Len(_) called on type that has no length: {:?}", pt),
        }
    }

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
                let res = ce1.add_overflow(ce2);
                Expr::struct_expr_from_values(
                    self.codegen_ty(res_ty),
                    vec![res.result, res.overflowed.cast_to(Type::c_bool())],
                    &self.symbol_table,
                )
            }
            BinOp::Sub => {
                let res = ce1.sub_overflow(ce2);
                Expr::struct_expr_from_values(
                    self.codegen_ty(res_ty),
                    vec![res.result, res.overflowed.cast_to(Type::c_bool())],
                    &self.symbol_table,
                )
            }
            BinOp::Mul => {
                let res = ce1.mul_overflow(ce2);
                Expr::struct_expr_from_values(
                    self.codegen_ty(res_ty),
                    vec![res.result, res.overflowed.cast_to(Type::c_bool())],
                    &self.symbol_table,
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
        op: &BinOp,
        e1: &Operand<'tcx>,
        e2: &Operand<'tcx>,
        loc: Location,
    ) -> Expr {
        match op {
            BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Shl | BinOp::Shr => {
                self.codegen_scalar_binop(op, e1, e2)
            }
            BinOp::Div | BinOp::Rem | BinOp::BitXor | BinOp::BitAnd | BinOp::BitOr => {
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
                ce1.plus(ce2)
            }
        }
    }

    fn codegen_rvalue_aggregate(
        &mut self,
        k: &AggregateKind<'tcx>,
        operands: &[Operand<'tcx>],
        res_ty: Ty<'tcx>,
    ) -> Expr {
        match *k {
            AggregateKind::Array(et) => {
                if et.is_unit() {
                    Expr::struct_expr_from_values(
                        self.codegen_ty(res_ty),
                        vec![],
                        &self.symbol_table,
                    )
                } else {
                    Expr::struct_expr_from_values(
                        self.codegen_ty(res_ty),
                        vec![Expr::array_expr(
                            self.codegen_ty_raw_array(res_ty),
                            operands.iter().map(|o| self.codegen_operand(o)).collect(),
                        )],
                        &self.symbol_table,
                    )
                }
            }
            AggregateKind::Tuple => Expr::struct_expr_from_values(
                self.codegen_ty(res_ty),
                operands
                    .iter()
                    .filter_map(|o| {
                        let oty = self.operand_ty(o);
                        if oty.is_unit() { None } else { Some(self.codegen_operand(o)) }
                    })
                    .collect(),
                &self.symbol_table,
            ),
            AggregateKind::Adt(_, _, _, _, _) => unimplemented!(),
            AggregateKind::Closure(_, _) => unimplemented!(),
            AggregateKind::Generator(_, _, _) => unimplemented!(),
        }
    }

    pub fn codegen_rvalue(&mut self, rv: &Rvalue<'tcx>, loc: Location) -> Expr {
        let res_ty = self.rvalue_ty(rv);
        debug!(?rv, "codegen_rvalue");
        match rv {
            Rvalue::Use(p) => self.codegen_operand(p),
            Rvalue::Repeat(op, sz) => self.codegen_rvalue_repeat(op, sz, res_ty, loc),
            Rvalue::Ref(_, _, p) | Rvalue::AddressOf(_, p) => self.codegen_rvalue_ref(p, res_ty),
            Rvalue::Len(p) => self.codegen_rvalue_len(p),
            // Rust has begun distinguishing "ptr -> num" and "num -> ptr" (providence-relevant casts) but we do not yet:
            // Should we? Tracking ticket: https://github.com/model-checking/kani/issues/1274
            Rvalue::Cast(
                CastKind::Misc
                | CastKind::PointerExposeAddress
                | CastKind::PointerFromExposedAddress,
                e,
                t,
            ) => {
                let t = self.monomorphize(*t);
                self.codegen_misc_cast(e, t)
            }
            Rvalue::Cast(CastKind::Pointer(k), e, t) => {
                let t = self.monomorphize(*t);
                self.codegen_pointer_cast(k, e, t, loc)
            }
            Rvalue::BinaryOp(op, box (ref e1, ref e2)) => {
                self.codegen_rvalue_binary_op(op, e1, e2, loc)
            }
            Rvalue::CheckedBinaryOp(op, box (ref e1, ref e2)) => {
                self.codegen_rvalue_checked_binary_op(op, e1, e2, res_ty)
            }
            Rvalue::NullaryOp(k, t) => {
                let t = self.monomorphize(*t);
                let layout = self.layout_of(t);
                match k {
                    NullOp::SizeOf => Expr::int_constant(layout.size.bytes_usize(), Type::size_t()),
                    NullOp::AlignOf => Expr::int_constant(layout.align.abi.bytes(), Type::size_t()),
                }
            }
            Rvalue::ShallowInitBox(ref operand, content_ty) => {
                // The behaviour of ShallowInitBox is simply transmuting *mut u8 to Box<T>.
                // See https://github.com/rust-lang/compiler-team/issues/460 for more details.
                let operand = self.codegen_operand(operand);
                let t = self.monomorphize(*content_ty);
                let box_ty = self.tcx.mk_box(t);
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
                self.codegen_rvalue_aggregate(k, operands, res_ty)
            }
            Rvalue::ThreadLocalRef(def_id) => {
                // Since Kani is single-threaded, we treat a thread local like a static variable:
                emit_concurrency_warning!("thread local", loc, "a static variable");
                self.codegen_static_pointer(*def_id, true)
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
        let expr = if ty.is_generator() {
            // Generators are translated somewhat differently from enums (see [`GotoCtx::codegen_ty_generator`]).
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
                TagEncoding::Niche { dataful_variant, niche_variants, niche_start } => {
                    // This code follows the logic in the ssa codegen backend:
                    // https://github.com/rust-lang/rust/blob/fee75fbe11b1fad5d93c723234178b2a329a3c03/compiler/rustc_codegen_ssa/src/mir/place.rs#L247
                    // See also the cranelift backend:
                    // https://github.com/rust-lang/rust/blob/05d22212e89588e7c443cc6b9bc0e4e02fdfbc8d/compiler/rustc_codegen_cranelift/src/discriminant.rs#L116
                    let offset = match &layout.fields {
                        FieldsShape::Arbitrary { offsets, .. } => offsets[0],
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
                        Expr::int_constant(dataful_variant.as_u32(), result_type),
                    )
                }
            },
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
                    ty::Slice(_) | ty::Str | ty::Dynamic(_, _) => {
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
    /// See the [`PointerCast`] type for specifics.
    /// Note that this does not include all casts involving pointers,
    /// many of which are instead handled by [`Self::codegen_misc_cast`] instead.
    fn codegen_pointer_cast(
        &mut self,
        k: &PointerCast,
        o: &Operand<'tcx>,
        t: Ty<'tcx>,
        loc: Location,
    ) -> Expr {
        match k {
            PointerCast::ReifyFnPointer => match self.operand_ty(o).kind() {
                ty::FnDef(def_id, substs) => {
                    let instance =
                        Instance::resolve(self.tcx, ty::ParamEnv::reveal_all(), *def_id, substs)
                            .unwrap()
                            .unwrap();
                    // We need to handle this case in a special way because `codegen_operand` compiles FnDefs to dummy structs.
                    // (cf. the function documentation)
                    self.codegen_func_expr(instance, None).address_of()
                }
                _ => unreachable!(),
            },
            PointerCast::UnsafeFnPointer => self.codegen_operand(o),
            PointerCast::ClosureFnPointer(_) => {
                let dest_typ = self.codegen_ty(t);
                self.codegen_unimplemented_expr(
                    "PointerCast::ClosureFnPointer",
                    dest_typ,
                    loc,
                    "https://github.com/model-checking/kani/issues/274",
                )
            }
            PointerCast::MutToConstPointer => self.codegen_operand(o),
            PointerCast::ArrayToPointer => {
                // TODO: I am not sure whether it is correct or not.
                //
                // some reasoning is as follows.
                // the trouble is to understand whether we have to handle fat pointers and my claim is no.
                // if we had to, then [o] necessarily has type [T; n] where *T is a fat pointer, meaning
                // T is either [T] or str. but neither type is sized, which shouldn't participate in
                // codegen.
                match self.operand_ty(o).kind() {
                    ty::RawPtr(ty::TypeAndMut { ty, .. }) => {
                        // ty must be an array
                        if let ty::Array(_, _) = ty.kind() {
                            let oe = self.codegen_operand(o);
                            oe.dereference() // : struct [T; n]
                                .member("0", &self.symbol_table) // : T[n]
                                .array_to_ptr() // : T*
                        } else {
                            unreachable!()
                        }
                    }
                    _ => unreachable!(),
                }
            }
            PointerCast::Unsize => {
                let src_goto_expr = self.codegen_operand(o);
                let src_mir_type = self.operand_ty(o);
                let dst_mir_type = t;
                self.cast_to_unsized_expr(src_goto_expr.clone(), src_mir_type, dst_mir_type)
                    .unwrap_or(src_goto_expr)
            }
        }
    }

    fn cast_to_unsized_expr(
        &mut self,
        src_goto_expr: Expr,
        src_mir_type: Ty<'tcx>,
        dst_mir_type: Ty<'tcx>,
    ) -> Option<Expr> {
        // Check if the cast is from a vtable fat pointer to another
        // vtable fat pointer (which can happen with auto trait fat pointers)
        if self.is_vtable_fat_pointer(src_mir_type) {
            self.cast_unsized_dyn_trait_to_unsized_dyn_trait(
                src_goto_expr,
                src_mir_type,
                dst_mir_type,
            )
        } else {
            // Recursively cast the source expression into an unsized expression.
            // This will include thin pointers, slices, and Adt.
            self.cast_expr_to_unsized_expr(src_goto_expr, src_mir_type, dst_mir_type)
        }
    }

    fn codegen_vtable_method_field(
        &mut self,
        instance: Instance<'tcx>,
        t: Ty<'tcx>,
        idx: usize,
    ) -> Expr {
        debug!(?instance, typ=?t, %idx, "codegen_vtable_method_field");
        let vtable_field_name = self.vtable_field_name(instance.def_id(), idx);
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
            // We skip an entire submodule of the standard library, so drop is missing
            // for it. Build and insert a function that just calls an unimplemented block
            // to maintain soundness.
            let drop_sym_name = format!("drop_unimplemented_{}", self.symbol_name(drop_instance));
            let pretty_name =
                format!("drop_unimplemented<{}>", self.readable_instance_name(drop_instance));
            let drop_sym = self.ensure(&drop_sym_name, |ctx, name| {
                // Function body
                let unimplemented = ctx.codegen_unimplemented_stmt(
                    format!("drop_in_place for {}", drop_instance).as_str(),
                    Location::none(),
                    "https://github.com/model-checking/kani/issues/281",
                );

                // Declare symbol for the single, self parameter
                let param_typ = ctx.codegen_ty(trait_ty).to_pointer();
                let param_sym = ctx.gen_function_parameter(0, &drop_sym_name, param_typ);

                // Build and insert the function itself
                Symbol::function(
                    name,
                    Type::code(vec![param_sym.to_function_parameter()], Type::empty()),
                    Some(Stmt::block(vec![unimplemented], Location::none())),
                    pretty_name,
                    Location::none(),
                )
            });
            drop_sym.to_expr().address_of().cast_to(trait_fn_ty)
        }
    }

    /// The size and alignment for the vtable is of the underlying type.
    /// When we get the size and align of a ty::Ref, the TyCtxt::layout_of
    /// returns the correct size to match rustc vtable values. Checked via
    /// Kani-compile-time and CBMC assertions in check_vtable_size.
    fn codegen_vtable_size_and_align(&self, operand_type: Ty<'tcx>) -> (Expr, Expr) {
        debug!("vtable_size_and_align {:?}", operand_type.kind());
        let vtable_layout = self.layout_of(operand_type);
        assert!(!vtable_layout.is_unsized(), "Can't create a vtable for an unsized type");
        let vt_size = Expr::int_constant(vtable_layout.size.bytes(), Type::size_t());
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
                "Expected Never, got: {:?}",
                operand_type
            );
            Type::size_t().zero()
        } else {
            Expr::object_size(temp_var.address_of())
        };
        let check = Expr::eq(cbmc_size, vt_size);
        let assert_msg =
            format!("Correct CBMC vtable size for {:?} (MIR type {:?})", ty, operand_type.kind());
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
        assert!(trait_type.is_trait(), "VTable trait type {} must be a trait type", trait_type);
        let binders = match trait_type.kind() {
            ty::Dynamic(binders, ..) => binders,
            _ => unimplemented!("Cannot codegen_vtable for type {:?}", dst_mir_type.kind()),
        };

        let src_name = self.ty_mangled_name(src_mir_type);
        // The name needs to be the same as inserted in typ.rs
        let vtable_name = self.vtable_name(trait_type).intern();
        let vtable_impl_name = format!("{}_impl_for_{}", vtable_name, src_name);

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

    /// Fat pointers to dynamic auto trait objects can be the src of casts.
    /// For example, this cast is legal, because Send is an auto trait with
    /// no associated function:
    ///
    ///     &(dyn Any + Send) as &dyn Any
    ///
    /// This cast is legal because without any changes to the set of virtual
    /// functions, the underlying vtable does not need to change.
    ///
    /// Cast a pointer from one usized dynamic trait object to another. The
    /// result  of the cast will be a fat pointer with the same data and
    /// vtable, but the new type. Returns None if no cast is needed.
    fn cast_unsized_dyn_trait_to_unsized_dyn_trait(
        &mut self,
        src_goto_expr: Expr,
        src_mir_type: Ty<'tcx>,
        dst_mir_type: Ty<'tcx>,
    ) -> Option<Expr> {
        if src_mir_type.kind() == dst_mir_type.kind() {
            return None; // no cast required, nothing to do
        }
        debug!(?src_goto_expr, ?src_mir_type, ?dst_mir_type, "cast_unsized_dyn_trait");

        // The source destination must be a fat pointers to a dyn trait object
        assert!(self.is_vtable_fat_pointer(src_mir_type));
        assert!(self.is_vtable_fat_pointer(dst_mir_type));

        let dst_goto_type = self.codegen_ty(dst_mir_type);

        // Cast the data type.
        let dst_mir_dyn_ty = pointee_type(dst_mir_type).unwrap();
        let dst_data_type = self.codegen_trait_data_pointer(dst_mir_dyn_ty);
        let data =
            src_goto_expr.to_owned().member("data", &self.symbol_table).cast_to(dst_data_type);

        // Retrieve the vtable and cast the vtable type.
        let vtable_name = self.vtable_name(dst_mir_dyn_ty);
        let vtable_ty = Type::struct_tag(vtable_name).to_pointer();
        let vtable = src_goto_expr.member("vtable", &self.symbol_table).cast_to(vtable_ty);

        // Construct a fat pointer with the same (casted) fields and new type
        Some(dynamic_fat_ptr(dst_goto_type, data, vtable, &self.symbol_table))
    }

    /// Cast an object / thin pointer to a fat pointer or an ADT with a nested fat pointer.
    /// Return the result of the cast as Some(expr) and return None if no cast was required.
    fn cast_expr_to_unsized_expr(
        &mut self,
        src_goto_expr: Expr,
        src_mir_type: Ty<'tcx>,
        dst_mir_type: Ty<'tcx>,
    ) -> Option<Expr> {
        if src_mir_type.kind() == dst_mir_type.kind() {
            return None; // no cast required, nothing to do
        }

        match (src_mir_type.kind(), dst_mir_type.kind()) {
            (ty::Ref(..), ty::Ref(..)) => {
                self.cast_sized_pointer_to_fat_pointer(src_goto_expr, src_mir_type, dst_mir_type)
            }
            (ty::Ref(..), ty::RawPtr(..)) => {
                self.cast_sized_pointer_to_fat_pointer(src_goto_expr, src_mir_type, dst_mir_type)
            }
            (ty::RawPtr(..), ty::Ref(..)) => {
                self.cast_sized_pointer_to_fat_pointer(src_goto_expr, src_mir_type, dst_mir_type)
            }
            (ty::RawPtr(..), ty::RawPtr(..)) => {
                self.cast_sized_pointer_to_fat_pointer(src_goto_expr, src_mir_type, dst_mir_type)
            }
            (ty::Adt(..), ty::Adt(..)) => {
                self.cast_adt_to_unsized_adt(src_goto_expr, src_mir_type, dst_mir_type)
            }
            (src_kind, dst_kind) => {
                unreachable!(
                    "In this case, {:?} and {:?} should have the same type (a case already handled)",
                    src_kind, dst_kind
                )
            }
        }
    }

    /// Cast a pointer to a sized object to a fat pointer to an unsized object.
    /// Return the result of the cast as Some(expr) and return None if no cast
    /// was required.
    /// Note: This seems conceptually wrong. If we are converting sized to unsized, how come
    /// source and destination can have the same type? Also, how come destination can be a thin
    /// pointer?
    /// TODO: Fix the cast code structure:
    /// <https://github.com/model-checking/kani/issues/1531>
    fn cast_sized_pointer_to_fat_pointer(
        &mut self,
        src_goto_expr: Expr,
        src_mir_type: Ty<'tcx>,
        dst_mir_type: Ty<'tcx>,
    ) -> Option<Expr> {
        // treat type equality as a no op
        if src_mir_type.kind() == dst_mir_type.kind() {
            return None;
        };

        // The src type cannot be a pointer to a dynamic trait object, otherwise
        // we should have called cast_unsized_dyn_trait_to_unsized_dyn_trait
        assert!(!self.is_vtable_fat_pointer(src_mir_type));

        // extract pointee types from pointer types, panic if type is not a
        // pointer type.
        let src_pointee_type = pointee_type(src_mir_type).unwrap();
        let dst_pointee_type = pointee_type(dst_mir_type).unwrap();

        if self.use_thin_pointer(dst_pointee_type) {
            assert_eq!(src_pointee_type, dst_pointee_type);
            None
        } else if self.use_slice_fat_pointer(dst_pointee_type) {
            self.cast_sized_pointer_to_slice_fat_pointer(
                src_goto_expr,
                src_mir_type,
                dst_mir_type,
                src_pointee_type,
                dst_pointee_type,
            )
        } else if self.use_vtable_fat_pointer(dst_pointee_type) {
            self.cast_sized_pointer_to_trait_fat_pointer(
                src_goto_expr,
                src_mir_type,
                dst_mir_type,
                src_pointee_type,
                dst_pointee_type,
            )
        } else {
            unreachable!(
                "A pointer is either a thin pointer, slice fat pointer, or vtable fat pointer."
            );
        }
    }

    /// Cast a pointer to a sized object to a fat pointer to a slice. Return the
    /// result of the cast as Some(expr) and return None if no cast was
    /// required.
    fn cast_sized_pointer_to_slice_fat_pointer(
        &mut self,
        src_goto_expr: Expr,
        _src_mir_type: Ty<'tcx>,
        dst_mir_type: Ty<'tcx>,
        src_pointee_type: Ty<'tcx>,
        dst_pointee_type: Ty<'tcx>,
    ) -> Option<Expr> {
        match (src_pointee_type.kind(), dst_pointee_type.kind()) {
            (ty::Array(src_elt_type, src_elt_count), ty::Slice(dst_elt_type)) => {
                assert_eq!(src_elt_type, dst_elt_type);
                let dst_goto_type = self.codegen_ty(dst_mir_type);
                let dst_goto_expr = // cast from an array type to a pointer type
                    src_goto_expr.cast_to(self.codegen_ty(*src_elt_type).to_pointer());
                let dst_goto_len = self.codegen_const(*src_elt_count, None);
                Some(slice_fat_ptr(dst_goto_type, dst_goto_expr, dst_goto_len, &self.symbol_table))
            }
            (src_kind, dst_kind) => panic!(
                "Only an array can be cast to a slice.  Found types {:?} and {:?}",
                src_kind, dst_kind
            ),
        }
    }

    /// Cast a pointer to a sized object to a fat pointer to a trait object.
    /// Return the result of the cast as Some(expr) and return None if no cast
    /// was required.
    fn cast_sized_pointer_to_trait_fat_pointer(
        &mut self,
        src_goto_expr: Expr,
        src_mir_type: Ty<'tcx>,
        dst_mir_type: Ty<'tcx>,
        src_pointee_type: Ty<'tcx>,
        dst_pointee_type: Ty<'tcx>,
    ) -> Option<Expr> {
        tracing::trace!(?src_pointee_type, ?dst_pointee_type, "cast_thin_2_fat_ptr");
        tracing::trace!(?src_mir_type, ?dst_mir_type, "cast_thin_2_fat_ptr");
        if let Some((concrete_type, trait_type)) =
            self.nested_pair_of_concrete_and_trait_types(src_pointee_type, dst_pointee_type)
        {
            tracing::trace!(?concrete_type, ?trait_type, "cast_thin_2_fat_ptr");
            let dst_goto_expr =
                src_goto_expr.cast_to(self.codegen_ty(dst_pointee_type).to_pointer());
            let dst_goto_type = self.codegen_ty(dst_mir_type);
            let vtable = self.codegen_vtable(concrete_type, trait_type);
            let vtable_expr = vtable.address_of();
            Some(dynamic_fat_ptr(dst_goto_type, dst_goto_expr, vtable_expr, &self.symbol_table))
        } else {
            None
        }
    }

    /// Cast an ADT (sized or unsized) to an unsized ADT (an ADT with a nested fat pointer).
    /// Return the result of the cast as Some(expr) and return None if no cast
    /// was required.
    fn cast_adt_to_unsized_adt(
        &mut self,
        src_goto_expr: Expr,
        src_mir_type: Ty<'tcx>,
        dst_mir_type: Ty<'tcx>,
    ) -> Option<Expr> {
        // Map field names to field values (goto expressions) and field types (mir types)
        let mut src_goto_field_values = src_goto_expr.struct_field_exprs(&self.symbol_table);
        let src_mir_field_types = self.mir_struct_field_types(src_mir_type);
        let dst_mir_field_types = self.mir_struct_field_types(dst_mir_type);

        // Assert that the struct expression and struct types have the same field names
        assert!(src_goto_field_values.keys().eq(src_mir_field_types.keys()));
        assert!(src_goto_field_values.keys().eq(dst_mir_field_types.keys()));

        // Cast each field and collect the fields for which a cast was required
        let mut cast_required: Vec<(InternedString, Expr)> = vec![];
        for field in src_goto_field_values.keys() {
            if let Some(expr) = self.cast_to_unsized_expr(
                src_goto_field_values.get(field).unwrap().clone(),
                *src_mir_field_types.get(field).unwrap(),
                *dst_mir_field_types.get(field).unwrap(),
            ) {
                cast_required.push((*field, expr));
            }
        }
        // Return None for a struct with fields if none of the fields require a cast.
        //
        // Note that a struct with no fields may still require a cast.
        // PhantomData is a zero-sized type that is a struct with no fields, and
        // hence with no fields that require a cast.  But PhantomData takes a
        // type as an generic parameter, and when casting a sized [u8; 4] to an
        // unsized [u8], we have to change the type of PhantomData from
        // PhantomData<[u8; 4]> to PhantomData<[u8]>.
        if !dst_mir_field_types.is_empty() && cast_required.is_empty() {
            return None;
        }

        for (field, expr) in cast_required {
            // Replace the field expression with the cast expression
            src_goto_field_values.insert(field, expr.clone());
        }
        let dst_goto_expr = Expr::struct_expr(
            self.codegen_ty(dst_mir_type),
            src_goto_field_values,
            &self.symbol_table,
        );
        Some(dst_goto_expr)
    }

    /// Find the trait type and corresponding concrete type in a pair of ADTs.
    ///
    /// Given two ADTs with types src and dst, the goal is to cast a thin
    /// pointer to src to a fat pointer to dst.  Dst has nested within it a
    /// trait type (a Dynamic).  Src has nested within it at the corresponding
    /// position a concrete type. This function returns the pair (concrete type,
    /// trait type) that we can use to build the vtable for the concrete type
    /// implementation of the trait type.
    fn nested_pair_of_concrete_and_trait_types(
        &self,
        src_mir_type: Ty<'tcx>,
        dst_mir_type: Ty<'tcx>,
    ) -> Option<(Ty<'tcx>, Ty<'tcx>)> {
        // We are walking an ADT searching for a trait type in this ADT.  We can
        // terminate a walk down a path when we hit a primitive type or which we hit
        // a pointer type (that would take us out of this ADT and into another type).
        if dst_mir_type.is_primitive() || is_pointer(dst_mir_type) {
            return None;
        }

        match (src_mir_type.kind(), dst_mir_type.kind()) {
            (_, ty::Dynamic(..)) => Some((src_mir_type, dst_mir_type)),
            (ty::Adt(..), ty::Adt(..)) => {
                let src_fields = self.mir_struct_field_types(src_mir_type);
                let dst_fields = self.mir_struct_field_types(dst_mir_type);
                assert!(src_fields.keys().eq(dst_fields.keys()));

                let mut matching_types: Option<(Ty<'tcx>, Ty<'tcx>)> = None;
                for field in src_fields.keys() {
                    let pair = self.nested_pair_of_concrete_and_trait_types(
                        *src_fields.get(field).unwrap(),
                        *dst_fields.get(field).unwrap(),
                    );
                    if pair.is_some() {
                        assert!(
                            matching_types.is_none(),
                            "Searching for pairs of concrete and trait types, found multiple pairs in {:?} and {:?}",
                            src_mir_type,
                            dst_mir_type
                        );
                        matching_types = pair;
                    }
                }
                matching_types
            }
            // In the context of
            //    handling Result::<&i32, ()>::unwrap, std::result::Result::<T, E>::unwrap
            //    let _1: std::result::Result<&i32, ()>
            //    let _3: ()
            //    let _6: &dyn std::fmt::Debug
            //    let _7: &()
            //    let _8: &()
            //    _3 = move ((_1 as Err).0: E)
            //    _8 = &_3
            //    _7 = _8
            //    _6 = move _7 as &dyn std::fmt::Debug (Pointer(Unsize))
            // we find rustc trying to cast () to a trait type.
            //
            // (ty::Tuple(ref types), ty::Dynamic(..)) if types.is_empty() => {
            //     Some((src_mir_type.clone(), dst_mir_type.clone()))
            // }
            _ => panic!(
                "Found unexpected types while searching for pairs of concrete and trait types in {:?} and {:?}",
                src_mir_type, dst_mir_type
            ),
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
