// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
use crate::gotoc::cbmc::goto_program::{Expr, Location, Stmt, Symbol, Type};
use crate::gotoc::mir_to_goto::utils::slice_fat_ptr;
use crate::gotoc::mir_to_goto::GotocCtx;
use rustc_ast::ast::Mutability;
use rustc_middle::mir::interpret::{
    read_target_uint, AllocId, Allocation, ConstValue, GlobalAlloc, Scalar,
};
use rustc_middle::mir::{Constant, ConstantKind, Operand};
use rustc_middle::ty::layout::LayoutOf;
use rustc_middle::ty::{
    self, Const, ConstKind, FloatTy, Instance, IntTy, ScalarInt, Ty, Uint, UintTy,
};
use rustc_span::def_id::DefId;
use rustc_span::Span;
use rustc_target::abi::{FieldsShape, Size, TagEncoding, Variants};
use tracing::debug;

enum AllocData<'a> {
    Bytes(&'a [u8]),
    Expr(Expr),
}

impl<'tcx> GotocCtx<'tcx> {
    pub fn codegen_operand(&mut self, o: &Operand<'tcx>) -> Expr {
        match o {
            Operand::Copy(d) | Operand::Move(d) =>
            // TODO: move shouldn't be the same as copy
            {
                let projection = self.codegen_place(d);
                // If the operand itself is a Dynamic (like when passing a boxed closure),
                // we need to pull off the fat pointer. In that case, the rustc kind() on
                // both the operand and the inner type are Dynamic.
                // Consider moving this check elsewhere in:
                // https://github.com/model-checking/rmc/issues/277
                match self.operand_ty(o).kind() {
                    ty::Dynamic(..) => projection.fat_ptr_goto_expr.unwrap(),
                    _ => projection.goto_expr,
                }
            }
            Operand::Constant(c) => self.codegen_constant(&c),
        }
    }

    fn codegen_constant(&mut self, c: &Constant<'tcx>) -> Expr {
        let const_ = match self.monomorphize(c.literal) {
            ConstantKind::Ty(ct) => ct,
            ConstantKind::Val(val, ty) => return self.codegen_const_value(val, ty, Some(&c.span)),
        };

        self.codegen_const(const_, Some(&c.span))
    }

    pub fn codegen_const(&mut self, lit: &'tcx Const<'tcx>, span: Option<&Span>) -> Expr {
        debug!("found literal: {:?}", lit);
        let lit = self.monomorphize(lit);

        match lit.val {
            // evaluate constant if it has no been evaluated yet
            ConstKind::Unevaluated(unevaluated) => {
                debug!("The literal was a Unevaluated");
                let const_val = self
                    .tcx
                    .const_eval_resolve(ty::ParamEnv::reveal_all(), unevaluated, None)
                    .unwrap();
                self.codegen_const_value(const_val, lit.ty, span)
            }

            ConstKind::Value(v) => {
                debug!("The literal was a ConstValue {:?}", v);
                self.codegen_const_value(v, lit.ty, span)
            }
            _ => {
                unreachable!("monomorphized item shouldn't have this constant value: {:?}", lit.val)
            }
        }
    }

    pub fn codegen_const_value(
        &mut self,
        v: ConstValue<'tcx>,
        lit_ty: Ty<'tcx>,
        span: Option<&Span>,
    ) -> Expr {
        match v {
            ConstValue::Scalar(s) => self.codegen_scalar(s, lit_ty, span),
            ConstValue::Slice { data, start, end } => match lit_ty.kind() {
                ty::Ref(_, ty::TyS { kind: ty::Str, .. }, _) => {
                    let slice = data.inspect_with_uninit_and_ptr_outside_interpreter(start..end);
                    let s = ::std::str::from_utf8(slice).expect("non utf8 str from miri");
                    Expr::struct_expr_from_values(
                        self.codegen_ty(lit_ty),
                        vec![Expr::string_constant(s), Expr::int_constant(s.len(), Type::size_t())],
                        &self.symbol_table,
                    )
                }
                ty::Ref(
                    _,
                    ty::TyS { kind: ty::Slice(ty::TyS { kind: Uint(UintTy::U8), .. }), .. },
                    _,
                ) => {
                    // The case where we have a slice of u8 is easy enough: make an array of u8
                    // TODO: Handle cases with larger int types by making an array of bytes,
                    // then using byte-extract on it.
                    let slice = data.inspect_with_uninit_and_ptr_outside_interpreter(start..end);
                    let vec_of_bytes: Vec<Expr> = slice
                        .iter()
                        .map(|b| Expr::int_constant(*b, Type::unsigned_int(8)))
                        .collect();
                    let len = vec_of_bytes.len();
                    let array_expr =
                        Expr::array_expr(Type::unsigned_int(8).array_of(len), vec_of_bytes);
                    let data_expr = array_expr.array_to_ptr();
                    let len_expr = Expr::int_constant(len, Type::size_t());
                    slice_fat_ptr(self.codegen_ty(lit_ty), data_expr, len_expr, &self.symbol_table)
                }
                _ => unimplemented!("\nv {:?}\nlit_ty {:?}\nspan {:?}", v, lit_ty, span),
            },
            ConstValue::ByRef { alloc, offset } => {
                debug!("ConstValue by ref {:?} {:?}", alloc, offset);
                let mem_var =
                    self.codegen_allocation_auto_imm_name(alloc, |tcx| tcx.next_global_name());
                mem_var
                    .cast_to(Type::unsigned_int(8).to_pointer())
                    .plus(Expr::int_constant(offset.bytes(), Type::unsigned_int(64)))
                    .cast_to(self.codegen_ty(lit_ty).to_pointer())
                    .dereference()
            }
        }
    }

    fn codegen_scalar(&mut self, s: Scalar, ty: Ty<'tcx>, span: Option<&Span>) -> Expr {
        debug! {"codegen_scalar\n{:?}\n{:?}\n{:?}\n{:?}",s, ty, span, &ty.kind};
        match (s, &ty.kind()) {
            (Scalar::Int(ScalarInt { data, .. }), ty::Int(it)) => match it {
                IntTy::I8 => Expr::int_constant(data, Type::signed_int(8)),
                IntTy::I16 => Expr::int_constant(data, Type::signed_int(16)),
                IntTy::I32 => Expr::int_constant(data, Type::signed_int(32)),
                IntTy::I64 => Expr::int_constant(data, Type::signed_int(64)),
                IntTy::I128 => Expr::int_constant(data, Type::signed_int(128)),
                IntTy::Isize => Expr::int_constant(data, Type::ssize_t()),
            },
            (Scalar::Int(ScalarInt { data, .. }), ty::Uint(it)) => match it {
                UintTy::U8 => Expr::int_constant(data, Type::unsigned_int(8)),
                UintTy::U16 => Expr::int_constant(data, Type::unsigned_int(16)),
                UintTy::U32 => Expr::int_constant(data, Type::unsigned_int(32)),
                UintTy::U64 => Expr::int_constant(data, Type::unsigned_int(64)),
                UintTy::U128 => Expr::int_constant(data, Type::unsigned_int(128)),
                UintTy::Usize => Expr::int_constant(data, Type::size_t()),
            },
            (Scalar::Int(ScalarInt { .. }), ty::Bool) => {
                Expr::c_bool_constant(s.to_bool().unwrap())
            }
            (Scalar::Int(ScalarInt { .. }), ty::Char) => {
                Expr::int_constant(s.to_i32().unwrap(), Type::signed_int(32))
            }
            (Scalar::Int(ScalarInt { .. }), ty::Float(k)) =>
            // rustc uses a sophisticated format for floating points that is hard to get f32/f64 from.
            // Instead, we use integers with the right width to represent the bit pattern.
            {
                match k {
                    FloatTy::F32 => Expr::float_constant_from_bitpattern(s.to_u32().unwrap()),
                    FloatTy::F64 => Expr::double_constant_from_bitpattern(s.to_u64().unwrap()),
                }
            }
            (Scalar::Int(ScalarInt { size: 0, .. }), ty::FnDef(d, substs)) => {
                self.codegen_fndef(*d, substs, span)
            }
            (Scalar::Int(ScalarInt { .. }), ty::RawPtr(tm)) => {
                Expr::pointer_constant(s.to_u64().unwrap(), self.codegen_ty(tm.ty).to_pointer())
            }
            // TODO: Removing this doesn't cause any regressions to fail.
            // We need a regression for this case.
            (Scalar::Int(ScalarInt { data: 0, .. }), ty::Ref(_, ty, _)) => {
                self.codegen_ty(ty).to_pointer().null()
            }
            (Scalar::Int(ScalarInt { .. }), ty::Adt(adt, subst)) => {
                if adt.is_struct() || adt.is_union() {
                    // in this case, we must have a one variant ADT. there are two cases
                    let variant = &adt.variants.raw[0];
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
                            let variant = &adt.variants[*index];

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
                        Variants::Multiple { tag_encoding, .. } => match tag_encoding {
                            TagEncoding::Niche { .. } => {
                                let offset = match &layout.fields {
                                    FieldsShape::Arbitrary { offsets, .. } => {
                                        offsets[0].bytes_usize()
                                    }
                                    _ => unreachable!("niche encoding must have arbitrary fields"),
                                };
                                let discr_ty = self.codegen_enum_discr_typ(ty);
                                let niche_val = self.codegen_scalar(s, discr_ty, span);
                                self.codegen_niche_literal(ty, offset, niche_val)
                            }

                            TagEncoding::Direct => {
                                // then the scalar field stores the discriminant
                                let discr_ty = self.codegen_enum_discr_typ(ty);

                                let init = self.codegen_scalar(s, discr_ty, span);
                                self.codegen_direct_literal(ty, init)
                            }
                        },
                    }
                }
            }
            (s, ty::Tuple(substs)) => {
                // here we have tuples of at most one length
                if substs.len() == 1 {
                    let overall_t = self.codegen_ty(ty);
                    let t = substs[0].expect_ty();
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

    fn codegen_direct_literal(&mut self, ty: Ty<'tcx>, init: Expr) -> Expr {
        let func_name = format!("gen-{}:direct", self.ty_mangled_name(ty));
        let cgt = self.codegen_ty(ty);
        self.ensure(&func_name, |tcx, _| {
            let target_ty = init.typ().clone(); // N
            let param = tcx.gen_function_local_variable(1, &func_name, target_ty);
            let var = tcx.gen_function_local_variable(2, &func_name, cgt.clone()).to_expr();
            let body = vec![
                Stmt::decl(var.clone(), None, Location::none()),
                var.clone()
                    .member("case", &tcx.symbol_table)
                    .assign(param.to_expr(), Location::none()),
                var.clone().ret(Location::none()),
            ];
            Symbol::function(
                &func_name,
                Type::code(vec![param.to_function_parameter()], cgt),
                Some(Stmt::block(body, Location::none())),
                None,
                Location::none(),
            )
        });

        self.find_function(&func_name).unwrap().call(vec![init])
    }

    pub fn codegen_fndef(
        &mut self,
        d: DefId,
        substs: ty::subst::SubstsRef<'tcx>,
        span: Option<&Span>,
    ) -> Expr {
        let instance =
            Instance::resolve(self.tcx, ty::ParamEnv::reveal_all(), d, substs).unwrap().unwrap();
        self.codegen_func_expr(instance, span)
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
            GlobalAlloc::Static(def_id) => {
                // here we have a potentially unevaluated static
                let instance = Instance::mono(self.tcx, def_id);

                let sym = self.ensure(&self.symbol_name(instance), |ctx, name| {
                    // check if this static is extern
                    let rlinkage = ctx.tcx.codegen_fn_attrs(def_id).linkage;

                    // we believe rlinkage being `Some` means the static not extern
                    // based on compiler/rustc_codegen_cranelift/src/linkage.rs#L21
                    // see https://github.com/model-checking/rmc/issues/388
                    //
                    // Update: The assertion below may fail in similar environments.
                    // We are disabling it until we find out the root cause, see
                    // https://github.com/model-checking/rmc/issues/400
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
                });
                sym.clone().to_expr().address_of()
            }
            GlobalAlloc::Memory(alloc) => {
                // Full (mangled) crate name added so that allocations from different
                // crates do not conflict. The name alone is insufficient becase Rust
                // allows different versions of the same crate to be used.
                let name = format!("{}::{:?}", self.full_crate_name(), alloc_id);
                self.codegen_allocation(alloc, |_| name.clone(), Some(name.clone()))
            }
        };
        base_addr
            .cast_to(Type::unsigned_int(8).to_pointer())
            .plus(Expr::int_constant(offset.bytes(), Type::unsigned_int(64)))
            .cast_to(res_t)
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
        let mut alloc_vals = Vec::with_capacity(alloc.relocations().len() + 1);
        let pointer_size = self.symbol_table.machine_model().pointer_width() as usize / 8;

        let mut next_offset = 0;
        for &(offset, alloc_id) in alloc.relocations().iter() {
            let offset = offset.bytes_usize();
            if offset > next_offset {
                let bytes =
                    alloc.inspect_with_uninit_and_ptr_outside_interpreter(next_offset..offset);
                alloc_vals.push(AllocData::Bytes(bytes));
            }
            let ptr_offset = {
                let bytes = alloc.inspect_with_uninit_and_ptr_outside_interpreter(
                    offset..(offset + pointer_size),
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
        if alloc.len() >= next_offset {
            let range = next_offset..alloc.len();
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

        // The declaration of a static variable may have one type and the constant initializer for
        // a static variable may have a different type. This is because Rust uses bit patterns for
        // initializers. For example, for a boolean static variable, the variable will have type
        // CBool and the initializer will be a single byte (a one-character array) representing the
        // bit pattern for the boolean value.
        let alloc_typ_ref = self.ensure_struct(&format!("{}::struct", name), |ctx, _| {
            ctx.codegen_allocation_data(alloc)
                .iter()
                .enumerate()
                .map(|(i, d)| match d {
                    AllocData::Bytes(bytes) => Type::datatype_component(
                        &i.to_string(),
                        Type::unsigned_int(8).array_of(bytes.len()),
                    ),
                    AllocData::Expr(e) => Type::datatype_component(&i.to_string(), e.typ().clone()),
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

    fn codegen_niche_init_name(&self, ty: Ty<'tcx>) -> String {
        let name = self.ty_mangled_name(ty);
        format!("gen-{}:niche", name)
    }

    /// fetch the niche value (as both left and right value)
    pub fn codegen_get_niche(&self, v: Expr, offset: usize, niche_ty: Type) -> Expr {
        v // t: T
            .address_of() // &t: T*
            .cast_to(Type::unsigned_int(8).to_pointer()) // (u8 *)&t: u8 *
            .plus(Expr::int_constant(offset, Type::size_t())) // ((u8 *)&t) + offset: u8 *
            .cast_to(niche_ty.to_pointer()) // (N *)(((u8 *)&t) + offset): N *
            .dereference() // *(N *)(((u8 *)&t) + offset): N
    }

    fn codegen_niche_literal(&mut self, ty: Ty<'tcx>, offset: usize, init: Expr) -> Expr {
        let cgt = self.codegen_ty(ty);
        let fname = self.codegen_niche_init_name(ty);
        self.ensure(&fname, |tcx, _| {
            let target_ty = init.typ().clone(); // N
            let param = tcx.gen_function_local_variable(1, &fname, target_ty.clone());
            let var = tcx.gen_function_local_variable(2, &fname, cgt.clone()).to_expr();
            let body = vec![
                Stmt::decl(var.clone(), None, Location::none()),
                tcx.codegen_get_niche(var.clone(), offset, target_ty)
                    .assign(param.to_expr(), Location::none()),
                var.ret(Location::none()),
            ];
            Symbol::function(
                &fname,
                Type::code(vec![param.to_function_parameter()], cgt),
                Some(Stmt::block(body, Location::none())),
                None,
                Location::none(),
            )
        });
        self.find_function(&fname).unwrap().call(vec![init])
    }

    pub fn codegen_func_expr(&mut self, instance: Instance<'tcx>, span: Option<&Span>) -> Expr {
        let func = self.symbol_name(instance);
        let funct = self.codegen_function_sig(self.fn_sig_of_instance(instance).unwrap());
        // make sure the functions imported from other modules are in the symbol table
        self.ensure(&func, |ctx, _| {
            Symbol::function(
                &func,
                funct.clone(),
                None,
                Some(ctx.readable_instance_name(instance)),
                Location::none(),
            )
            .with_is_extern(true)
        });
        Expr::symbol_expression(func, funct).with_location(self.codegen_span_option(span.cloned()))
    }
}
