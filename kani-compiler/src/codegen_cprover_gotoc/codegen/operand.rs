// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
use crate::codegen_cprover_gotoc::utils::slice_fat_ptr;
use crate::codegen_cprover_gotoc::GotocCtx;
use crate::unwrap_or_return_codegen_unimplemented;
use cbmc::btree_string_map;
use cbmc::goto_program::{DatatypeComponent, Expr, Location, Stmt, Symbol, Type};
use rustc_ast::ast::Mutability;
use rustc_middle::mir::interpret::{
    read_target_uint, AllocId, Allocation, ConstValue, GlobalAlloc, Scalar,
};
use rustc_middle::mir::{Constant, ConstantKind, Operand};
use rustc_middle::ty::layout::LayoutOf;
use rustc_middle::ty::{self, Const, ConstKind, FloatTy, Instance, IntTy, Ty, Uint, UintTy};
use rustc_span::def_id::DefId;
use rustc_span::Span;
use rustc_target::abi::{Size, TagEncoding, Variants};
use tracing::{debug, trace};

enum AllocData<'a> {
    Bytes(&'a [u8]),
    Expr(Expr),
}

impl<'tcx> GotocCtx<'tcx> {
    pub fn codegen_operand(&mut self, o: &Operand<'tcx>) -> Expr {
        trace!(operand=?o, "codegen_operand");
        match o {
            Operand::Copy(d) | Operand::Move(d) =>
            // TODO: move is an opportunity to poison/nondet the original memory.
            {
                let projection =
                    unwrap_or_return_codegen_unimplemented!(self, self.codegen_place(d));
                // If the operand itself is a Dynamic (like when passing a boxed closure),
                // we need to pull off the fat pointer. In that case, the rustc kind() on
                // both the operand and the inner type are Dynamic.
                // Consider moving this check elsewhere in:
                // https://github.com/model-checking/kani/issues/277
                match self.operand_ty(o).kind() {
                    ty::Dynamic(..) => projection.fat_ptr_goto_expr.unwrap(),
                    _ => projection.goto_expr,
                }
            }
            Operand::Constant(c) => self.codegen_constant(c),
        }
    }

    fn codegen_constant(&mut self, c: &Constant<'tcx>) -> Expr {
        trace!(constant=?c, "codegen_constant");
        let const_ = match self.monomorphize(c.literal) {
            ConstantKind::Ty(ct) => ct,
            ConstantKind::Val(val, ty) => return self.codegen_const_value(val, ty, Some(&c.span)),
        };

        self.codegen_const(const_, Some(&c.span))
    }

    pub fn codegen_const(&mut self, lit: Const<'tcx>, span: Option<&Span>) -> Expr {
        debug!("found literal: {:?}", lit);
        let lit = self.monomorphize(lit);

        match lit.kind() {
            // evaluate constant if it has no been evaluated yet
            ConstKind::Unevaluated(unevaluated) => {
                debug!("The literal was a Unevaluated");
                let const_val = self
                    .tcx
                    .const_eval_resolve(ty::ParamEnv::reveal_all(), unevaluated, None)
                    .unwrap();
                self.codegen_const_value(const_val, lit.ty(), span)
            }

            ConstKind::Value(valtree) => {
                let value = self.tcx.valtree_to_const_val((lit.ty(), valtree));
                debug!("The literal was a ConstValue {:?}", value);
                self.codegen_const_value(value, lit.ty(), span)
            }
            _ => {
                unreachable!(
                    "monomorphized item shouldn't have this constant value: {:?}",
                    lit.kind()
                )
            }
        }
    }

    fn codegen_slice_value(
        &mut self,
        v: ConstValue<'tcx>,
        lit_ty: Ty<'tcx>,
        span: Option<&Span>,
        data: &Allocation,
        start: usize,
        end: usize,
    ) -> Expr {
        if let ty::Ref(_, ref_ty, _) = lit_ty.kind() {
            match ref_ty.kind() {
                ty::Str => {
                    let slice = data.inspect_with_uninit_and_ptr_outside_interpreter(start..end);
                    let s = ::std::str::from_utf8(slice).expect("non utf8 str from miri");
                    return Expr::struct_expr_from_values(
                        self.codegen_ty(lit_ty),
                        vec![Expr::string_constant(s), Expr::int_constant(s.len(), Type::size_t())],
                        &self.symbol_table,
                    );
                }
                ty::Slice(slice_ty) => {
                    if let Uint(UintTy::U8) = slice_ty.kind() {
                        // The case where we have a slice of u8 is easy enough: make an array of u8
                        let slice =
                            data.inspect_with_uninit_and_ptr_outside_interpreter(start..end);
                        let vec_of_bytes: Vec<Expr> = slice
                            .iter()
                            .map(|b| Expr::int_constant(*b, Type::unsigned_int(8)))
                            .collect();
                        let len = vec_of_bytes.len();
                        let array_expr =
                            Expr::array_expr(Type::unsigned_int(8).array_of(len), vec_of_bytes);
                        let data_expr = array_expr.array_to_ptr();
                        let len_expr = Expr::int_constant(len, Type::size_t());
                        return slice_fat_ptr(
                            self.codegen_ty(lit_ty),
                            data_expr,
                            len_expr,
                            &self.symbol_table,
                        );
                    } else {
                        // TODO: Handle cases with other types such as tuples and larger integers.
                        let loc = self.codegen_span_option(span.cloned());
                        let typ = self.codegen_ty(lit_ty);
                        return self.codegen_unimplemented_expr(
                            "Constant slice value with 2+ bytes",
                            typ,
                            loc,
                            "https://github.com/model-checking/kani/issues/1339",
                        );
                    }
                }
                _ => {}
            }
        }
        unimplemented!("\nv {:?}\nlit_ty {:?}\nspan {:?}", v, lit_ty.kind(), span);
    }

    pub fn codegen_const_value(
        &mut self,
        v: ConstValue<'tcx>,
        lit_ty: Ty<'tcx>,
        span: Option<&Span>,
    ) -> Expr {
        trace!(val=?v, ?lit_ty, "codegen_const_value");
        match v {
            ConstValue::Scalar(s) => self.codegen_scalar(s, lit_ty, span),
            ConstValue::Slice { data, start, end } => {
                self.codegen_slice_value(v, lit_ty, span, data.inner(), start, end)
            }
            ConstValue::ByRef { alloc, offset } => {
                debug!("ConstValue by ref {:?} {:?}", alloc, offset);
                let mem_var = self
                    .codegen_allocation_auto_imm_name(alloc.inner(), |tcx| tcx.next_global_name());
                mem_var
                    .cast_to(Type::unsigned_int(8).to_pointer())
                    .plus(Expr::int_constant(offset.bytes(), Type::unsigned_int(64)))
                    .cast_to(self.codegen_ty(lit_ty).to_pointer())
                    .dereference()
            }
            ConstValue::ZeroSized => match lit_ty.kind() {
                ty::FnDef(d, substs) => self.codegen_fndef(*d, substs, span),
                _ => unimplemented!(),
            },
        }
    }

    fn codegen_scalar(&mut self, s: Scalar, ty: Ty<'tcx>, span: Option<&Span>) -> Expr {
        debug!(scalar=?s, ?ty, kind=?ty.kind(), ?span, "codegen_scalar");
        match (s, &ty.kind()) {
            (Scalar::Int(_), ty::Int(it)) => match it {
                IntTy::I8 => Expr::int_constant(s.to_i8().unwrap(), Type::signed_int(8)),
                IntTy::I16 => Expr::int_constant(s.to_i16().unwrap(), Type::signed_int(16)),
                IntTy::I32 => Expr::int_constant(s.to_i32().unwrap(), Type::signed_int(32)),
                IntTy::I64 => Expr::int_constant(s.to_i64().unwrap(), Type::signed_int(64)),
                IntTy::I128 => Expr::int_constant(s.to_i128().unwrap(), Type::signed_int(128)),
                IntTy::Isize => {
                    Expr::int_constant(s.to_machine_isize(self).unwrap(), Type::ssize_t())
                }
            },
            (Scalar::Int(_), ty::Uint(it)) => match it {
                UintTy::U8 => Expr::int_constant(s.to_u8().unwrap(), Type::unsigned_int(8)),
                UintTy::U16 => Expr::int_constant(s.to_u16().unwrap(), Type::unsigned_int(16)),
                UintTy::U32 => Expr::int_constant(s.to_u32().unwrap(), Type::unsigned_int(32)),
                UintTy::U64 => Expr::int_constant(s.to_u64().unwrap(), Type::unsigned_int(64)),
                UintTy::U128 => Expr::int_constant(s.to_u128().unwrap(), Type::unsigned_int(128)),
                UintTy::Usize => {
                    Expr::int_constant(s.to_machine_usize(self).unwrap(), Type::size_t())
                }
            },
            (Scalar::Int(_), ty::Bool) => Expr::c_bool_constant(s.to_bool().unwrap()),
            (Scalar::Int(_), ty::Char) => {
                Expr::int_constant(s.to_i32().unwrap(), Type::signed_int(32))
            }
            (Scalar::Int(_), ty::Float(k)) =>
            // rustc uses a sophisticated format for floating points that is hard to get f32/f64 from.
            // Instead, we use integers with the right width to represent the bit pattern.
            {
                match k {
                    FloatTy::F32 => Expr::float_constant_from_bitpattern(s.to_u32().unwrap()),
                    FloatTy::F64 => Expr::double_constant_from_bitpattern(s.to_u64().unwrap()),
                }
            }
            (Scalar::Int(..), ty::FnDef(..)) => {
                // This was removed here: https://github.com/rust-lang/rust/pull/98957.
                unreachable!("ZST is no longer represented as a scalar")
            }
            (Scalar::Int(_), ty::RawPtr(tm)) => {
                Expr::pointer_constant(s.to_u64().unwrap(), self.codegen_ty(tm.ty).to_pointer())
            }
            // TODO: Removing this doesn't cause any regressions to fail.
            // We need a regression for this case.
            (Scalar::Int(int), ty::Ref(_, ty, _)) => {
                if int.is_null() {
                    self.codegen_ty(*ty).to_pointer().null()
                } else {
                    unreachable!()
                }
            }
            (Scalar::Int(_), ty::Adt(adt, subst)) => {
                if adt.is_struct() || adt.is_union() {
                    // in this case, we must have a one variant ADT. there are two cases
                    let variant = &adt.variants().raw[0];
                    // if there is no field, then it's just a ZST
                    if variant.fields.is_empty() {
                        if adt.is_struct() {
                            let overall_t = self.codegen_ty(ty);
                            Expr::struct_expr_from_values(overall_t, vec![], &self.symbol_table)
                        } else {
                            unimplemented!()
                        }
                    } else {
                        // otherwise, there is just one field, which is stored as the scalar data
                        let field = &variant.fields[0];
                        let fty = field.ty(self.tcx, subst);

                        let overall_t = self.codegen_ty(ty);
                        if adt.is_struct() {
                            self.codegen_single_variant_single_field(s, span, overall_t, fty)
                        } else {
                            unimplemented!()
                        }
                    }
                } else {
                    // if it's an enum
                    let layout = self.layout_of(ty);
                    let overall_t = self.codegen_ty(ty);
                    match &layout.variants {
                        Variants::Single { index } => {
                            // here we must have one variant
                            let variant = &adt.variants()[*index];

                            match variant.fields.len() {
                                0 => Expr::struct_expr_from_values(
                                    overall_t,
                                    vec![],
                                    &self.symbol_table,
                                ),
                                1 => {
                                    let fty = variant.fields[0].ty(self.tcx, subst);
                                    self.codegen_single_variant_single_field(
                                        s, span, overall_t, fty,
                                    )
                                }
                                _ => unreachable!(),
                            }
                        }
                        Variants::Multiple { tag_encoding, tag_field, .. } => match tag_encoding {
                            TagEncoding::Niche { .. } => {
                                let niche_offset = layout.fields.offset(*tag_field);
                                assert_eq!(
                                    niche_offset,
                                    Size::ZERO,
                                    "nonzero offset for niche in scalar"
                                );
                                let discr_ty = self.codegen_enum_discr_typ(ty);
                                let niche_val = self.codegen_scalar(s, discr_ty, span);
                                let result_type = self.codegen_ty(ty);
                                let niche_type = niche_val.typ().clone();
                                assert_eq!(
                                    niche_type.sizeof_in_bits(&self.symbol_table),
                                    result_type.sizeof_in_bits(&self.symbol_table),
                                    "niche type and enum have different size in scalar"
                                );
                                niche_val.transmute_to(result_type, &self.symbol_table)
                            }

                            TagEncoding::Direct => {
                                // then the scalar field stores the discriminant
                                let discr_ty = self.codegen_enum_discr_typ(ty);
                                let init = self.codegen_scalar(s, discr_ty, span);
                                let cgt = self.codegen_ty(ty);
                                let fields =
                                    cgt.get_non_empty_components(&self.symbol_table).unwrap();
                                // TagEncoding::Direct makes a constant with a tag but no data.
                                // Check our understanding that that the Enum must have one field,
                                // which is the tag, and no data field.
                                assert_eq!(
                                    fields.len(),
                                    1,
                                    "TagEncoding::Direct encountered for enum with non-empty variants"
                                );
                                assert_eq!(
                                    fields[0].name().to_string(),
                                    "case",
                                    "Unexpected field in enum/generator. Please report your failing case at https://github.com/model-checking/kani/issues/1465"
                                );
                                Expr::struct_expr_with_nondet_fields(
                                    cgt,
                                    btree_string_map![("case", init)],
                                    &self.symbol_table,
                                )
                            }
                        },
                    }
                }
            }
            (s, ty::Tuple(substs)) => {
                // here we have tuples of at most one length
                if substs.len() == 1 {
                    let overall_t = self.codegen_ty(ty);
                    let t = substs[0];
                    self.codegen_single_variant_single_field(s, span, overall_t, t)
                } else {
                    unreachable!()
                }
            }
            (_, ty::Array(_, _)) => {
                // we must have zero size array here
                Expr::struct_expr_from_values(
                    self.codegen_ty(ty),
                    vec![Expr::array_expr(self.codegen_ty_raw_array(ty), vec![])],
                    &self.symbol_table,
                )
            }
            (Scalar::Ptr(ptr, _size), _) => {
                let res_t = self.codegen_ty(ty);
                let (alloc_id, offset) = ptr.into_parts();
                self.codegen_alloc_pointer(res_t, alloc_id, offset, span)
            }
            _ => unimplemented!(),
        }
    }

    pub fn codegen_fndef(
        &mut self,
        d: DefId,
        substs: ty::subst::SubstsRef<'tcx>,
        span: Option<&Span>,
    ) -> Expr {
        let instance =
            Instance::resolve(self.tcx, ty::ParamEnv::reveal_all(), d, substs).unwrap().unwrap();
        self.codegen_fn_item(instance, span)
    }

    fn codegen_alloc_pointer(
        &mut self,
        res_t: Type,
        alloc_id: AllocId,
        offset: Size,
        span: Option<&Span>,
    ) -> Expr {
        let base_addr = match self.tcx.global_alloc(alloc_id) {
            GlobalAlloc::Function(instance) => {
                // here we have a function pointer
                self.codegen_func_expr(instance, span).address_of()
            }
            GlobalAlloc::Static(def_id) => self.codegen_static_pointer(def_id, false),
            GlobalAlloc::Memory(alloc) => {
                // Full (mangled) crate name added so that allocations from different
                // crates do not conflict. The name alone is insufficient because Rust
                // allows different versions of the same crate to be used.
                let name = format!("{}::{:?}", self.full_crate_name(), alloc_id);
                self.codegen_allocation(alloc.inner(), |_| name.clone(), Some(name.clone()))
            }
            GlobalAlloc::VTable(ty, trait_ref) => {
                // This is similar to GlobalAlloc::Memory but the type is opaque to rust and it
                // requires a bit more logic to get information about the allocation.
                let alloc_id = self.tcx.vtable_allocation((ty, trait_ref));
                let alloc = self.tcx.global_alloc(alloc_id).unwrap_memory();
                let name = format!("{}::{:?}", self.full_crate_name(), alloc_id);
                self.codegen_allocation(alloc.inner(), |_| name.clone(), Some(name.clone()))
            }
        };
        assert!(res_t.is_pointer() || res_t.is_transparent_type(&self.symbol_table));
        let offset_addr = base_addr
            .cast_to(Type::unsigned_int(8).to_pointer())
            .plus(Expr::int_constant(offset.bytes(), Type::unsigned_int(64)));

        // In some cases, Rust uses a transparent type here. Convert the pointer to an rvalue
        // of the type expected. https://github.com/model-checking/kani/issues/822
        if let Some(wrapped_type) = res_t.unwrap_transparent_type(&self.symbol_table) {
            assert!(wrapped_type.is_pointer());
            offset_addr
                .cast_to(wrapped_type)
                .transmute_to_structurally_equivalent_type(res_t, &self.symbol_table)
        } else {
            assert!(res_t.is_pointer());
            offset_addr.cast_to(res_t)
        }
    }

    /// Generates a pointer to a static or thread-local variable.
    pub fn codegen_static_pointer(&mut self, def_id: DefId, is_thread_local: bool) -> Expr {
        // here we have a potentially unevaluated static
        let instance = Instance::mono(self.tcx, def_id);

        let sym = self.ensure(&self.symbol_name(instance), |ctx, name| {
            // check if this static is extern
            let rlinkage = ctx.tcx.codegen_fn_attrs(def_id).linkage;

            // we believe rlinkage being `Some` means the static not extern
            // based on compiler/rustc_codegen_cranelift/src/linkage.rs#L21
            // see https://github.com/model-checking/kani/issues/388
            //
            // Update: The assertion below may fail in similar environments.
            // We are disabling it until we find out the root cause, see
            // https://github.com/model-checking/kani/issues/400
            //
            // assert!(rlinkage.is_none());

            let span = ctx.tcx.def_span(def_id);
            Symbol::static_variable(
                name.to_string(),
                name.to_string(),
                ctx.codegen_ty(instance.ty(ctx.tcx, ty::ParamEnv::reveal_all())),
                ctx.codegen_span(&span),
            )
            .with_is_extern(rlinkage.is_none())
            .with_is_thread_local(is_thread_local)
        });
        sym.clone().to_expr().address_of()
    }

    pub fn codegen_allocation_auto_imm_name<F: FnOnce(&mut GotocCtx<'tcx>) -> String>(
        &mut self,
        alloc: &'tcx Allocation,
        mut_name: F,
    ) -> Expr {
        self.codegen_allocation(alloc, mut_name, None)
    }

    pub fn codegen_allocation<F: FnOnce(&mut GotocCtx<'tcx>) -> String>(
        &mut self,
        alloc: &'tcx Allocation,
        mut_name: F,
        imm_name: Option<String>,
    ) -> Expr {
        debug!("codegen_allocation imm_name {:?} alloc {:?}", imm_name, alloc);
        let mem_var = match alloc.mutability {
            Mutability::Mut => {
                let name = mut_name(self);
                self.codegen_alloc_in_memory(alloc, name.clone());
                // here we know name must be in the symbol table
                self.symbol_table.lookup(&name).unwrap().clone().to_expr()
            }
            Mutability::Not => self.codegen_immutable_allocation(alloc, imm_name),
        };
        mem_var.address_of()
    }

    fn codegen_allocation_data(&mut self, alloc: &'tcx Allocation) -> Vec<AllocData<'tcx>> {
        let mut alloc_vals = Vec::with_capacity(alloc.provenance().len() + 1);
        let pointer_size =
            Size::from_bytes(self.symbol_table.machine_model().pointer_width_in_bytes());

        let mut next_offset = Size::ZERO;
        for &(offset, alloc_id) in alloc.provenance().iter() {
            if offset > next_offset {
                let bytes = alloc.inspect_with_uninit_and_ptr_outside_interpreter(
                    next_offset.bytes_usize()..offset.bytes_usize(),
                );
                alloc_vals.push(AllocData::Bytes(bytes));
            }
            let ptr_offset = {
                let bytes = alloc.inspect_with_uninit_and_ptr_outside_interpreter(
                    offset.bytes_usize()..(offset + pointer_size).bytes_usize(),
                );
                read_target_uint(self.tcx.sess.target.options.endian, bytes)
            }
            .unwrap();
            alloc_vals.push(AllocData::Expr(self.codegen_alloc_pointer(
                Type::signed_int(8).to_pointer(),
                alloc_id,
                Size::from_bytes(ptr_offset),
                None,
            )));

            next_offset = offset + pointer_size;
        }
        if alloc.len() >= next_offset.bytes_usize() {
            let range = next_offset.bytes_usize()..alloc.len();
            let bytes = alloc.inspect_with_uninit_and_ptr_outside_interpreter(range);
            alloc_vals.push(AllocData::Bytes(bytes));
        }

        alloc_vals
    }

    /// since it's immutable, we only allocate one location for it
    fn codegen_immutable_allocation(
        &mut self,
        alloc: &'tcx Allocation,
        name: Option<String>,
    ) -> Expr {
        if !self.alloc_map.contains_key(&alloc) {
            let name = if let Some(name) = name { name } else { self.next_global_name() };
            self.codegen_alloc_in_memory(alloc, name);
        }

        self.symbol_table.lookup(&self.alloc_map.get(&alloc).unwrap()).unwrap().to_expr()
    }

    /// Codegen alloc as a static global variable with initial value
    fn codegen_alloc_in_memory(&mut self, alloc: &'tcx Allocation, name: String) {
        debug!("codegen_alloc_in_memory name: {}", name);
        let struct_name = &format!("{}::struct", name);

        // The declaration of a static variable may have one type and the constant initializer for
        // a static variable may have a different type. This is because Rust uses bit patterns for
        // initializers. For example, for a boolean static variable, the variable will have type
        // CBool and the initializer will be a single byte (a one-character array) representing the
        // bit pattern for the boolean value.
        let alloc_typ_ref = self.ensure_struct(&struct_name, &struct_name, |ctx, _| {
            ctx.codegen_allocation_data(alloc)
                .iter()
                .enumerate()
                .map(|(i, d)| match d {
                    AllocData::Bytes(bytes) => DatatypeComponent::field(
                        &i.to_string(),
                        Type::unsigned_int(8).array_of(bytes.len()),
                    ),
                    AllocData::Expr(e) => DatatypeComponent::field(&i.to_string(), e.typ().clone()),
                })
                .collect()
        });

        // The global static variable may not be in the symbol table if we are dealing
        // with a literal that can be statically allocated.
        // We need to make a constructor whether it was in the table or not, so we can't use the
        // closure argument to ensure_global_var to do that here.
        let var = self.ensure_global_var(
            &name,
            false, //TODO is this correct?
            alloc_typ_ref.clone(),
            Location::none(),
            |_, _| None,
        );
        let var_typ = var.typ().clone();

        // Assign the initial value `val` to `var` via an intermediate `temp_var` to allow for
        // transmuting the allocation type to the global static variable type.
        let alloc_data = self.codegen_allocation_data(alloc);
        let val = Expr::struct_expr_from_values(
            alloc_typ_ref.clone(),
            alloc_data
                .iter()
                .map(|d| match d {
                    AllocData::Bytes(bytes) => Expr::array_expr(
                        Type::unsigned_int(8).array_of(bytes.len()),
                        bytes
                            .iter()
                            .map(|b| Expr::int_constant(*b, Type::unsigned_int(8)))
                            .collect(),
                    ),
                    AllocData::Expr(e) => e.clone(),
                })
                .collect(),
            &self.symbol_table,
        );
        let fn_name = Self::initializer_fn_name(&name);
        let temp_var = self.gen_function_local_variable(0, &fn_name, alloc_typ_ref).to_expr();
        let body = Stmt::block(
            vec![
                Stmt::decl(temp_var.clone(), Some(val), Location::none()),
                var.assign(temp_var.transmute_to(var_typ, &self.symbol_table), Location::none()),
            ],
            Location::none(),
        );
        self.register_initializer(&name, body);

        self.alloc_map.insert(alloc, name);
    }

    fn codegen_single_variant_single_field(
        &mut self,
        s: Scalar,
        span: Option<&Span>,
        overall_t: Type,
        fty: Ty<'tcx>,
    ) -> Expr {
        if fty.is_unit() {
            Expr::struct_expr_from_values(overall_t, vec![], &self.symbol_table)
        } else {
            Expr::struct_expr_from_values(
                overall_t,
                vec![self.codegen_scalar(s, fty, span)],
                &self.symbol_table,
            )
        }
    }

    /// fetch the niche value (as both left and right value)
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

    /// Ensure that the given instance is in the symbol table, returning the symbol.
    ///
    /// FIXME: The function should not have to return the type of the function symbol as well
    /// because the symbol should have the type. The problem is that the type in the symbol table
    /// sometimes subtly differs from the type that codegen_function_sig returns.
    /// This is tracked in <https://github.com/model-checking/kani/issues/1350>.
    pub fn codegen_func_symbol(&mut self, instance: Instance<'tcx>) -> (&Symbol, Type) {
        let func = self.symbol_name(instance);
        let funct = self.codegen_function_sig(self.fn_sig_of_instance(instance));
        // make sure the functions imported from other modules are in the symbol table
        let sym = self.ensure(&func, |ctx, _| {
            Symbol::function(
                &func,
                funct.clone(),
                None,
                ctx.readable_instance_name(instance),
                Location::none(),
            )
            .with_is_extern(true)
        });
        (sym, funct)
    }

    /// For a given function instance, generates an expression for the function symbol of type `Code`.
    ///
    /// Note: use `codegen_func_expr_zst` in the general case because GotoC does not allow functions to be used in all contexts
    /// (e.g. struct fields).
    ///
    /// For details, see <https://github.com/model-checking/kani/pull/1338>
    pub fn codegen_func_expr(&mut self, instance: Instance<'tcx>, span: Option<&Span>) -> Expr {
        let (func_symbol, func_typ) = self.codegen_func_symbol(instance);
        Expr::symbol_expression(func_symbol.name, func_typ)
            .with_location(self.codegen_span_option(span.cloned()))
    }

    /// For a given function instance, generates a zero-sized dummy symbol of type `Struct`.
    ///
    /// This is often necessary because GotoC does not allow functions to be used in all contexts (e.g. struct fields).
    /// For details, see <https://github.com/model-checking/kani/pull/1338>
    ///
    /// Note: use `codegen_func_expr` instead if you want to call the function immediately.
    fn codegen_fn_item(&mut self, instance: Instance<'tcx>, span: Option<&Span>) -> Expr {
        let (func_symbol, _) = self.codegen_func_symbol(instance);
        let func = func_symbol.name;
        let fn_struct_ty = self.codegen_fndef_type(instance);
        // This zero-sized object that a function name refers to in Rust is globally unique, so we create such a global object.
        let fn_singleton_name = format!("{func}::FnDefSingleton");
        let fn_singleton = self.ensure_global_var(
            &fn_singleton_name,
            false,
            fn_struct_ty,
            Location::none(),
            |_, _| None, // zero-sized, so no initialization necessary
        );
        fn_singleton.with_location(self.codegen_span_option(span.cloned()))
    }
}
