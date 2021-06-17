// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
use super::cbmc::goto_program::{BuiltinFn, Expr, Location, Stmt, Symbol, Type};
use super::cbmc::utils::{aggr_name, BUG_REPORT_URL};
use super::cbmc::MachineModel;
use super::metadata::*;
use super::typ::{is_pointer, pointee_type};
use super::utils::{dynamic_fat_ptr, slice_fat_ptr};
use crate::btree_string_map;
use num::bigint::BigInt;
use rustc_middle::mir::{AggregateKind, BinOp, CastKind, NullOp, Operand, Place, Rvalue, UnOp};
use rustc_middle::ty::adjustment::PointerCast;
use rustc_middle::ty::{self, Binder, Instance, IntTy, TraitRef, Ty, UintTy};
use rustc_span::def_id::DefId;
use rustc_target::abi::{FieldsShape, LayoutOf, Primitive, TagEncoding, Variants};
use tracing::{debug, warn};

impl<'tcx> GotocCtx<'tcx> {
    fn codegen_comparison(&mut self, op: &BinOp, e1: &Operand<'tcx>, e2: &Operand<'tcx>) -> Expr {
        match op {
            BinOp::Eq => {
                if self.operand_ty(e1).is_floating_point() {
                    self.codegen_operand(e1).feq(self.codegen_operand(e2))
                } else {
                    self.codegen_operand(e1).eq(self.codegen_operand(e2))
                }
            }
            BinOp::Lt => self.codegen_operand(e1).lt(self.codegen_operand(e2)),
            BinOp::Le => self.codegen_operand(e1).le(self.codegen_operand(e2)),
            BinOp::Ne => {
                if self.operand_ty(e1).is_floating_point() {
                    self.codegen_operand(e1).fneq(self.codegen_operand(e2))
                } else {
                    self.codegen_operand(e1).neq(self.codegen_operand(e2))
                }
            }
            BinOp::Ge => self.codegen_operand(e1).ge(self.codegen_operand(e2)),
            BinOp::Gt => self.codegen_operand(e1).gt(self.codegen_operand(e2)),
            _ => unreachable!(),
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
            BinOp::Div => ce1.div(ce2),
            BinOp::Rem => ce1.rem(ce2),
            BinOp::BitXor => {
                if self.operand_ty(e1).is_bool() {
                    ce1.xor(ce2)
                } else {
                    ce1.bitxor(ce2)
                }
            }
            BinOp::BitAnd => {
                if self.operand_ty(e1).is_bool() {
                    ce1.and(ce2)
                } else {
                    ce1.bitand(ce2)
                }
            }
            BinOp::BitOr => {
                if self.operand_ty(e1).is_bool() {
                    ce1.or(ce2)
                } else {
                    ce1.bitor(ce2)
                }
            }
            _ => unreachable!(),
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
    pub fn codegen_rvalue_ref(&mut self, place: &Place<'tcx>, result_mir_type: Ty<'tcx>) -> Expr {
        let place_mir_type = self.place_ty(place);
        let projection = self.codegen_place(place);

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

        // In the sequence of projections leading to this place, we dereferenced
        // this fat pointer.
        let intermediate_fat_pointer = projection.fat_ptr_goto_expr.unwrap();

        // The thin pointer in the resulting fat pointer is a pointer to the value
        let thin_pointer = if place_goto_expr.typ().is_pointer() {
            // The value is itself a pointer (eg, a void pointer), just use this pointer
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
            dynamic_fat_ptr(
                result_goto_type,
                thin_pointer.cast_to(Type::void_pointer()),
                metadata,
                &self.symbol_table,
            )
        } else {
            unreachable!();
        }
    }

    fn codegen_rvalue_repeat(
        &mut self,
        op: &Operand<'tcx>,
        sz: &&'tcx ty::Const<'tcx>,
        res_ty: Ty<'tcx>,
    ) -> Expr {
        let func_name = format!("gen-repeat<{}>", self.ty_mangled_name(res_ty));
        self.ensure(&func_name, |tcx, _| {
            let paramt = tcx.codegen_ty(tcx.operand_ty(op));
            let res_t = tcx.codegen_ty(res_ty);
            let inp = tcx.gen_function_local_variable(1, &func_name, paramt);
            let res = tcx.gen_function_local_variable(2, &func_name, res_t.clone()).to_expr();
            let idx = tcx.gen_function_local_variable(3, &func_name, Type::size_t()).to_expr();
            let mut body = vec![
                Stmt::decl(res.clone(), None, Location::none()),
                Stmt::decl(idx.clone(), Some(Type::size_t().zero()), Location::none()),
            ];

            let lbody = Stmt::block(
                vec![
                    tcx.codegen_idx_array(res.clone(), idx.clone())
                        .assign(inp.to_expr(), Location::none()),
                ],
                Location::none(),
            );
            body.push(Stmt::for_loop(
                Stmt::skip(Location::none()),
                idx.clone().lt(tcx.codegen_const(sz, None)),
                idx.postincr().as_stmt(Location::none()),
                lbody,
                Location::none(),
            ));
            body.push(res.ret(Location::none()));
            Symbol::function(
                &func_name,
                Type::code(vec![inp.to_function_parameter()], res_t),
                Some(Stmt::block(body, Location::none())),
                None,
                Location::none(),
            )
        });
        self.find_function(&func_name).unwrap().call(vec![self.codegen_operand(op)])
    }

    pub fn codegen_rvalue_len(&mut self, p: &Place<'tcx>) -> Expr {
        let pt = self.place_ty(p);
        match pt.kind() {
            ty::Array(_, sz) => self.codegen_const(sz, None),
            ty::Slice(_) => {
                self.codegen_place(p).fat_ptr_goto_expr.unwrap().member("len", &self.symbol_table)
            }
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
                    IntTy::Isize => Expr::int_constant(mm.pointer_width() - 1, Type::ssize_t()),
                },
                ty::Uint(k) => match k {
                    UintTy::U8 => Expr::int_constant(7, Type::unsigned_int(8)),
                    UintTy::U16 => Expr::int_constant(15, Type::unsigned_int(16)),
                    UintTy::U32 => Expr::int_constant(31, Type::unsigned_int(32)),
                    UintTy::U64 => Expr::int_constant(63, Type::unsigned_int(64)),
                    UintTy::U128 => Expr::int_constant(127, Type::unsigned_int(128)),
                    UintTy::Usize => Expr::int_constant(mm.pointer_width() - 1, Type::size_t()),
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
    ) -> Expr {
        match op {
            BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Shl | BinOp::Shr => {
                self.codegen_scalar_binop(op, e1, e2)
            }
            BinOp::Div | BinOp::Rem | BinOp::BitXor | BinOp::BitAnd | BinOp::BitOr => {
                self.codegen_unchecked_scalar_binop(op, e1, e2)
            }
            BinOp::Eq | BinOp::Lt | BinOp::Le | BinOp::Ne | BinOp::Ge | BinOp::Gt => {
                self.codegen_comparison(op, e1, e2)
            }
            // https://doc.rust-lang.org/std/primitive.pointer.html#method.offset
            BinOp::Offset => {
                let ce1 = self.codegen_operand(e1);
                let ce2 = self.codegen_operand(e2);
                ce1.plus(ce2)
            }
        }
    }

    pub fn codegen_rvalue_aggregate(
        &mut self,
        k: &AggregateKind<'tcx>,
        operands: &Vec<Operand<'tcx>>,
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

    pub fn codegen_rvalue(&mut self, rv: &Rvalue<'tcx>) -> Expr {
        let res_ty = self.rvalue_ty(rv);
        match rv {
            Rvalue::Use(p) => self.codegen_operand(p),
            Rvalue::Repeat(op, sz) => self.codegen_rvalue_repeat(op, sz, res_ty),
            Rvalue::Ref(_, _, p) | Rvalue::AddressOf(_, p) => self.codegen_rvalue_ref(p, res_ty),
            Rvalue::Len(p) => self.codegen_rvalue_len(p),
            Rvalue::Cast(CastKind::Misc, e, t) => {
                let t = self.monomorphize(*t);
                self.codegen_misc_cast(e, t)
            }
            Rvalue::Cast(CastKind::Pointer(k), e, t) => {
                let t = self.monomorphize(*t);
                self.codegen_pointer_cast(k, e, t)
            }
            Rvalue::BinaryOp(op, box (ref e1, ref e2)) => self.codegen_rvalue_binary_op(op, e1, e2),
            Rvalue::CheckedBinaryOp(op, box (ref e1, ref e2)) => {
                self.codegen_rvalue_checked_binary_op(op, e1, e2, res_ty)
            }
            Rvalue::NullaryOp(k, t) => match k {
                NullOp::SizeOf => {
                    let t = self.monomorphize(*t);
                    let layout = self.layout_of(t);
                    Expr::int_constant(layout.size.bytes_usize(), Type::size_t())
                }
                NullOp::Box => {
                    let t = self.monomorphize(*t);
                    let layout = self.layout_of(t);
                    let size = layout.size.bytes_usize();
                    let box_ty = self.tcx.mk_box(t);
                    let box_ty = self.codegen_ty(box_ty);
                    let cbmc_t = self.codegen_ty(t);
                    let box_contents = BuiltinFn::Malloc
                        .call(vec![Expr::int_constant(size, Type::size_t())], Location::none())
                        .cast_to(cbmc_t.to_pointer());
                    self.box_value(box_contents, box_ty)
                }
            },
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
                let place = self.codegen_place(p).goto_expr;
                let pt = self.place_ty(p);
                self.codegen_get_discriminant(place, pt, res_ty)
            }
            Rvalue::Aggregate(ref k, operands) => {
                self.codegen_rvalue_aggregate(&*k, operands, res_ty)
            }
            Rvalue::ThreadLocalRef(_) => unimplemented!(),
        }
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
            Variants::Multiple { tag, tag_encoding, .. } => match tag_encoding {
                TagEncoding::Direct => {
                    e.member("case", &self.symbol_table).cast_to(self.codegen_ty(res_ty))
                }
                TagEncoding::Niche { dataful_variant, niche_variants, niche_start } => {
                    let offset = match &layout.fields {
                        FieldsShape::Arbitrary { offsets, .. } => offsets[0].bytes_usize(),
                        _ => unreachable!("niche encoding must have arbitrary fields"),
                    };
                    let discr_ty = self.codegen_enum_discr_typ(ty);
                    let discr_ty = self.codegen_ty(discr_ty);
                    let niche_val = self.codegen_get_niche(e, offset, discr_ty.clone());
                    let relative_discr = if *niche_start == 0 {
                        niche_val
                    } else {
                        niche_val.sub(Expr::int_constant(*niche_start, discr_ty.clone()))
                    };
                    let relative_max =
                        niche_variants.end().as_u32() - niche_variants.start().as_u32();
                    let is_niche = if tag.value == Primitive::Pointer {
                        discr_ty.null().eq(relative_discr.clone())
                    } else {
                        relative_discr
                            .clone()
                            .cast_to(Type::unsigned_int(64))
                            .le(Expr::int_constant(relative_max, Type::unsigned_int(64)))
                    };
                    let niche_discr = {
                        let relative_discr = if relative_max == 0 {
                            self.codegen_ty(res_ty).zero()
                        } else {
                            relative_discr.cast_to(self.codegen_ty(res_ty))
                        };
                        relative_discr.plus(Expr::int_constant(
                            niche_variants.start().as_u32(),
                            self.codegen_ty(res_ty),
                        ))
                    };
                    is_niche.ternary(
                        niche_discr,
                        Expr::int_constant(dataful_variant.as_u32(), self.codegen_ty(res_ty)),
                    )
                }
            },
        }
    }

    pub fn codegen_fat_ptr_to_fat_ptr_cast(
        &mut self,
        src: &Operand<'tcx>,
        dst_t: Ty<'tcx>,
    ) -> Expr {
        debug!("codegen_fat_ptr_to_fat_ptr_cast |{:?}| |{:?}|", src, dst_t);
        let src_goto_expr = self.codegen_operand(src);
        let dst_goto_typ = self.codegen_ty(dst_t);
        let dst_data_type =
            self.symbol_table.lookup_field_type_in_type(&dst_goto_typ, "data").unwrap();
        let dst_data_field = (
            "data",
            src_goto_expr.clone().member("data", &self.symbol_table).cast_to(dst_data_type.clone()),
        );

        let dst_metadata_field = if let Some(vtable_typ) =
            self.symbol_table.lookup_field_type_in_type(&dst_goto_typ, "vtable")
        {
            (
                "vtable",
                src_goto_expr.member("vtable", &self.symbol_table).cast_to(vtable_typ.clone()),
            )
        } else if let Some(len_typ) =
            self.symbol_table.lookup_field_type_in_type(&dst_goto_typ, "len")
        {
            ("len", src_goto_expr.member("len", &self.symbol_table).cast_to(len_typ.clone()))
        } else {
            unreachable!("fat pointer with neither vtable nor len. {:?} {:?}", src, dst_t);
        };
        Expr::struct_expr(
            dst_goto_typ,
            btree_string_map![dst_data_field, dst_metadata_field],
            &self.symbol_table,
        )
    }

    pub fn codegen_fat_ptr_to_thin_ptr_cast(
        &mut self,
        src: &Operand<'tcx>,
        dst_t: Ty<'tcx>,
    ) -> Expr {
        debug!("codegen_fat_ptr_to_thin_ptr_cast |{:?}| |{:?}|", src, dst_t);
        let src_goto_expr = self.codegen_operand(src);
        let dst_goto_typ = self.codegen_ty(dst_t);
        // In a vtable fat pointer, the data member is a void pointer,
        // so ensure the pointer has the correct type before dereferencing it.
        src_goto_expr.member("data", &self.symbol_table).cast_to(dst_goto_typ)
    }

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
        if self.is_ref_of_unsized(src_t) && self.is_ref_of_unsized(dst_t) {
            return self.codegen_fat_ptr_to_fat_ptr_cast(src, dst_t);
        }

        if self.is_ref_of_unsized(src_t) && self.is_ref_of_sized(dst_t) {
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
                                ty::Slice(_) | ty::Str | ty::Dynamic(..) => {
                                    return self
                                        .codegen_operand(src)
                                        .member("data", &self.symbol_table)
                                        .cast_to(self.codegen_ty(dst_t));
                                }
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

    pub fn codegen_pointer_cast(
        &mut self,
        k: &PointerCast,
        o: &Operand<'tcx>,
        t: Ty<'tcx>,
    ) -> Expr {
        match k {
            PointerCast::ReifyFnPointer => self.codegen_operand(o).address_of(),
            PointerCast::UnsafeFnPointer => self.codegen_operand(o),
            PointerCast::ClosureFnPointer(_) => unimplemented!(),
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
                let dst_goto_expr = self.cast_sized_expr_to_unsized_expr(
                    src_goto_expr.clone(),
                    src_mir_type,
                    dst_mir_type,
                );
                dst_goto_expr.unwrap_or(src_goto_expr)
            }
        }
    }

    fn codegen_vtable_method_field(
        &mut self,
        def_id: DefId,
        _substs: ty::subst::SubstsRef<'tcx>,
        trait_ref_t: Binder<'_, TraitRef<'tcx>>,
        t: Ty<'tcx>,
    ) -> Expr {
        let vtable_field_name = self.vtable_field_name(def_id);
        let vtable_type_name = aggr_name(&self.vtable_name(t));
        let field_type = self
            .symbol_table
            .lookup_field_type(&vtable_type_name, &vtable_field_name)
            .cloned()
            .unwrap();

        // We use Instance::resolve to more closely match Rust proper behavior. The comment
        // there says "used to find the precise code that will run for a trait method invocation"
        // and it is used (in a more indirect way) to generate vtables.
        let instance = Instance::resolve(
            self.tcx,
            ty::ParamEnv::reveal_all(),
            def_id,
            trait_ref_t.skip_binder().substs,
        )
        .unwrap()
        .unwrap();

        // Lookup in the symbol table using the full symbol table name/key
        let fn_name = self.symbol_name(instance);
        if let Some(fn_symbol) = self.symbol_table.lookup(&fn_name) {
            // Create a pointer to the method
            // Note that the method takes a self* as the first argument, but the vtable field type has a void* as the first arg.
            // So we need to cast it at the end.
            Expr::symbol_expression(fn_symbol.name.clone(), fn_symbol.typ.clone())
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

    fn codegen_vtable_drop_in_place(&self) -> Expr {
        // TODO: The following still needs to be done for the destructor (drop_in_place method), but before that
        // the right type for drop_in_place needs to be set in typ.rs in codegen_ty_ref method as well.
        // Atm we pass a nullpointer instead (Type::empty().pointer(ctx.ptr_width()).null() in vtable_base)
        // This is sound, because CBMC will give a pointer error if we ever actually use it.
        //
        // let drop_instance =
        //     Instance::resolve_drop_in_place(ctx.tcx, operand_type);
        // let drop_method_name = self.symbol_name(drop_instance);
        // let drop_inst_type =
        //     drop_instance.ty(self.tcx, ty::ParamEnv::reveal_all());
        // let drop_irep = data::Variable {
        //     data: drop_method_name.clone(),
        //     typ: self.codegen_ty(drop_inst_type),
        //     // typ: output_type,
        //     location: data::Irep::nil(),
        // }
        // .to_expr()
        // .address_of();
        Type::void_pointer().null()
    }

    fn codegen_vtable_methods(
        &mut self,
        trait_ref_t: Binder<'tcx, TraitRef<'tcx>>,
        t: Ty<'tcx>,
    ) -> Vec<Expr> {
        //DSN This assumes that we always get the methods back in the same order.
        let methods = self
            .tcx
            .vtable_methods(trait_ref_t)
            .iter()
            .cloned()
            // if `optional_method` is None then the method cannot be called from this object
            // in this case, do not emit a field
            .filter_map(|optional_method| {
                optional_method.map(|(def_id, substs)| {
                    self.codegen_vtable_method_field(def_id, substs, trait_ref_t, t)
                })
            })
            .collect();
        methods
    }

    /// The size and alignment for the vtable is of the underlying type.
    /// When we get the size and align of a ty::Ref, the TyCtxt::layout_of
    /// returns the correct size to match rustc vtable values. Checked via
    /// RMC-compile-time and CBMC assertions in check_vtable_size.
    ///
    /// As per tautschn@, the way to get the sizeof an object in gotoc is
    /// `__CPROVER_POINTER_OFFSET(&((int *)0)[1])`
    fn codegen_vtable_size_and_align(&self, operand_type: Ty<'tcx>) -> (Expr, Expr) {
        debug!("vtable_size_and_align {:?}", operand_type.kind());
        let vtable_layout = self.layout_of(operand_type);
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
        let temp_var = self.gen_temp_variable(ty, Location::none()).to_expr();
        let decl = Stmt::decl(temp_var.clone(), None, Location::none());
        let check = Expr::eq(Expr::object_size(temp_var.address_of()), vt_size.clone());
        let assert_msg = format!("Correct CBMC vtable size for {:?}", operand_type.kind());
        let size_assert =
            Stmt::assert_sanity_check(check, &assert_msg, BUG_REPORT_URL, Location::none());
        Stmt::block(vec![decl, size_assert], Location::none())
    }

    fn codegen_vtable(&mut self, src_mir_type: Ty<'tcx>, dst_mir_type: Ty<'tcx>) -> Expr {
        let trait_type = match dst_mir_type.kind() {
            // dst is pointer type
            ty::Ref(_, pointee_type, ..) => pointee_type,
            // dst is box type
            ty::Adt(adt_def, adt_subst) if adt_def.is_box() => {
                adt_subst.first().unwrap().expect_ty()
            }
            // dst is dynamic type
            ty::Dynamic(..) => dst_mir_type,
            _ => unimplemented!("Cannot codegen_vtable for type {:?}", dst_mir_type.kind()),
        };
        assert!(trait_type.is_trait(), "VTable trait type {} must be a trait type", trait_type);
        let binders = match trait_type.kind() {
            ty::Dynamic(binders, ..) => binders,
            _ => unimplemented!("Cannot codegen_vtable for type {:?}", dst_mir_type.kind()),
        };

        let src_name = self.ty_mangled_name(src_mir_type);
        // name needs to be the same as inserted in typ.rs
        let vtable_name = self.vtable_name(trait_type);
        let vtable_impl_name = format!("{}_impl_for_{}", vtable_name, src_name);

        self.ensure_global_var(
            &vtable_impl_name,
            true, // REVISIT: static-scope https://github.com/model-checking/rmc/issues/10
            Type::struct_tag(&vtable_name),
            Location::none(),
            |ctx, var| {
                // Build the vtable
                // See compiler/rustc_codegen_llvm/src/gotoc/typ.rs `trait_vtable_field_types` for field order
                let drop_irep = ctx.codegen_vtable_drop_in_place();
                let (vt_size, vt_align) = ctx.codegen_vtable_size_and_align(&src_mir_type);
                let size_assert = ctx.check_vtable_size(&src_mir_type, vt_size.clone());
                let mut vtable_fields = vec![drop_irep, vt_size, vt_align];

                // The virtual methods on the trait ref. Some auto traits have no methods.
                if let Some(principal) = binders.principal() {
                    let concrete_type = principal.with_self_ty(ctx.tcx, src_mir_type);
                    let mut methods = ctx.codegen_vtable_methods(concrete_type, trait_type);
                    vtable_fields.append(&mut methods);
                }
                let vtable = Expr::struct_expr_from_values(
                    Type::struct_tag(&vtable_name),
                    vtable_fields,
                    &ctx.symbol_table,
                );
                let body = var.assign(vtable, Location::none());
                let block = Stmt::block(vec![size_assert, body], Location::none());
                Some(block)
            },
        )
    }

    /// Cast a sized object to an unsized object: the result of the cast will be
    /// a fat pointer or an ADT with a nested fat pointer.  Return the result of
    /// the cast as Some(expr) and return None if no cast was required.
    fn cast_sized_expr_to_unsized_expr(
        &mut self,
        src_goto_expr: Expr,
        src_mir_type: Ty<'tcx>,
        dst_mir_type: Ty<'tcx>,
    ) -> Option<Expr> {
        if src_mir_type.kind() == dst_mir_type.kind() {
            return None; // no cast required, nothing to do
        }

        // The src type will be sized, but the dst type may not be unsized.  If
        // the dst is an adt containing a pointer to a trait object nested
        // within the adt, the trait object will be unsized and the pointer will
        // be a fat pointer, but the adt (containing the fat pointer) will
        // itself be sized.
        assert!(
            src_mir_type.is_sized(self.tcx.at(rustc_span::DUMMY_SP), ty::ParamEnv::reveal_all())
        );

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
                self.cast_sized_adt_to_unsized_adt(src_goto_expr, src_mir_type, dst_mir_type)
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
                    src_goto_expr.cast_to(self.codegen_ty(src_elt_type).to_pointer());
                let dst_goto_len = self.codegen_const(src_elt_count, None);
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
        _src_mir_type: Ty<'tcx>,
        dst_mir_type: Ty<'tcx>,
        src_pointee_type: Ty<'tcx>,
        dst_pointee_type: Ty<'tcx>,
    ) -> Option<Expr> {
        if let Some((concrete_type, trait_type)) =
            self.nested_pair_of_concrete_and_trait_types(src_pointee_type, dst_pointee_type)
        {
            let dst_goto_expr = src_goto_expr.cast_to(Type::void_pointer());
            let dst_goto_type = self.codegen_ty(dst_mir_type);
            let vtable = self.codegen_vtable(concrete_type, trait_type);
            let vtable_expr = vtable.address_of();
            Some(dynamic_fat_ptr(dst_goto_type, dst_goto_expr, vtable_expr, &self.symbol_table))
        } else {
            None
        }
    }

    /// Cast a sized ADT to an unsized ADT (an ADT with a nested fat pointer).
    /// Return the result of the cast as Some(expr) and return None if no cast
    /// was required.
    fn cast_sized_adt_to_unsized_adt(
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
        let mut cast_required: Vec<(String, Expr)> = vec![];
        for field in src_goto_field_values.keys() {
            if let Some(expr) = self.cast_sized_expr_to_unsized_expr(
                src_goto_field_values.get(field).unwrap().clone(),
                src_mir_field_types.get(field).unwrap(),
                dst_mir_field_types.get(field).unwrap(),
            ) {
                cast_required.push((field.to_string(), expr));
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
            src_goto_field_values.insert(field.clone(), expr.clone());
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
    pub fn nested_pair_of_concrete_and_trait_types(
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
            (_, ty::Dynamic(..)) => Some((src_mir_type.clone(), dst_mir_type.clone())),
            (ty::Adt(..), ty::Adt(..)) => {
                let src_fields = self.mir_struct_field_types(src_mir_type);
                let dst_fields = self.mir_struct_field_types(dst_mir_type);
                assert!(src_fields.keys().eq(dst_fields.keys()));

                let mut matching_types: Option<(Ty<'tcx>, Ty<'tcx>)> = None;
                for field in src_fields.keys() {
                    let pair = self.nested_pair_of_concrete_and_trait_types(
                        src_fields.get(field).unwrap(),
                        dst_fields.get(field).unwrap(),
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
