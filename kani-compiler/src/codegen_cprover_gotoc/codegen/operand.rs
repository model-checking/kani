// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
use crate::codegen_cprover_gotoc::utils::slice_fat_ptr;
use crate::codegen_cprover_gotoc::GotocCtx;
use crate::unwrap_or_return_codegen_unimplemented;
use cbmc::btree_string_map;
use cbmc::goto_program::{DatatypeComponent, Expr, ExprValue, Location, Stmt, Symbol, Type};
use rustc_ast::ast::Mutability;
use rustc_middle::mir::interpret::{
    read_target_uint, AllocId, Allocation, ConstValue, GlobalAlloc, Scalar,
};
use rustc_middle::mir::{Constant, ConstantKind, Operand, UnevaluatedConst};
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
    /// Generate a goto expression from a MIR operand.
    ///
    /// A MIR operand is either a constant (literal or `const` declaration) or a place
    /// (being moved or copied for this operation).
    /// An "operand" in MIR is the argument to an "Rvalue" (and is also used by some statements.)
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

    /// Generate a goto expression from a MIR constant operand.
    ///
    /// There are three possibile constants:
    /// 1. `Ty` means e.g. that it's a const generic parameter. (See `codegen_const`)
    /// 2. `Val` means it's a constant value of various kinds. (See `codegen_const_value`)
    /// 3. `Unevaluated` means we need to run the interpreter, to get a `ConstValue`. (See `codegen_const_unevaluated`)
    fn codegen_constant(&mut self, c: &Constant<'tcx>) -> Expr {
        trace!(constant=?c, "codegen_constant");
        let span = Some(&c.span);
        match self.monomorphize(c.literal) {
            ConstantKind::Ty(ct) => self.codegen_const(ct, span),
            ConstantKind::Val(val, ty) => self.codegen_const_value(val, ty, span),
            ConstantKind::Unevaluated(unevaluated, ty) => {
                self.codegen_const_unevaluated(unevaluated, ty, span)
            }
        }
    }

    /// Runs the interpreter to get a `ConstValue`, then call `codegen_const_value`
    fn codegen_const_unevaluated(
        &mut self,
        unevaluated: UnevaluatedConst<'tcx>,
        ty: Ty<'tcx>,
        span: Option<&Span>,
    ) -> Expr {
        debug!(?unevaluated, "codegen_const_unevaluated");
        let const_val =
            self.tcx.const_eval_resolve(ty::ParamEnv::reveal_all(), unevaluated, None).unwrap();
        self.codegen_const_value(const_val, ty, span)
    }

    /// Generate a goto expression from a MIR `Const`.
    ///
    /// `Const` are special constant values that (only?) come from the type system,
    /// and consequently only need monomorphization to produce a value.
    ///
    /// Not to be confused with the more general MIR `Constant` which may need interpretation.
    pub fn codegen_const(&mut self, lit: Const<'tcx>, span: Option<&Span>) -> Expr {
        debug!("found literal: {:?}", lit);
        let lit = self.monomorphize(lit);

        match lit.kind() {
            // A `ConstantKind::Ty(ConstKind::Unevaluated)` should no longer show up
            // and should be a `ConstantKind::Unevaluated` instead (and thus handled
            // at the level of `codegen_constant` instead of `codegen_const`.)
            ConstKind::Unevaluated(_) => unreachable!(),

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

    /// Generate a goto expression from a MIR `ConstValue`.
    ///
    /// A `ConstValue` is the result of evaluation of a constant (of various original forms).
    /// All forms of constant code generation ultimately land here, where we have an actual value
    /// that we now just need to translate based on its kind.
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
                let mem_var = self.codegen_const_allocation(alloc.inner(), None);
                mem_var
                    .cast_to(Type::unsigned_int(8).to_pointer())
                    .plus(Expr::int_constant(offset.bytes(), Type::unsigned_int(64)))
                    .cast_to(self.codegen_ty(lit_ty).to_pointer())
                    .dereference()
            }
            ConstValue::ZeroSized => match lit_ty.kind() {
                // Rust "function items" (not closures, not function pointers, see `codegen_fndef`)
                ty::FnDef(d, substs) => self.codegen_fndef(*d, substs, span),
                _ => Expr::init_unit(self.codegen_ty(lit_ty), &self.symbol_table),
            },
        }
    }

    /// Generate a goto expression from a MIR `ConstValue::Slice`.
    ///
    /// A constant slice is an internal reference to another constant allocation.
    fn codegen_slice_value(
        &mut self,
        v: ConstValue<'tcx>,
        lit_ty: Ty<'tcx>,
        span: Option<&Span>,
        data: &'tcx Allocation,
        start: usize,
        end: usize,
    ) -> Expr {
        if let ty::Ref(_, ref_ty, _) = lit_ty.kind() {
            match ref_ty.kind() {
                ty::Str => {
                    // a string literal
                    // These seem to always start at 0
                    assert_eq!(start, 0);
                    // Create a static variable that holds its value
                    let mem_var = self.codegen_const_allocation(data, None);

                    // Extract identifier for static variable.
                    // codegen_allocation_auto_imm_name returns the *address* of
                    // the variable, so need to pattern match to extract it.
                    let ident = match mem_var.value() {
                        ExprValue::AddressOf(address) => match address.value() {
                            ExprValue::Symbol { identifier } => identifier,
                            _ => unreachable!("Expecting a symbol for a string literal allocation"),
                        },
                        _ => unreachable!("Expecting an address for string literal allocation"),
                    };

                    // Extract the actual string literal
                    let slice = data.inspect_with_uninit_and_ptr_outside_interpreter(start..end);
                    let s = ::std::str::from_utf8(slice).expect("non utf8 str from miri");

                    // Store the identifier to the string literal in the goto context
                    self.str_literals.insert(*ident, s.into());

                    // Codegen as a fat pointer
                    let data_expr = mem_var.cast_to(Type::unsigned_int(8).to_pointer());
                    let len_expr = Expr::int_constant(end - start, Type::size_t());
                    return slice_fat_ptr(
                        self.codegen_ty(lit_ty),
                        data_expr,
                        len_expr,
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
                        let operation_name = format!("Constant slice for type {slice_ty}");
                        return self.codegen_unimplemented_expr(
                            &operation_name,
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

    /// Generate a goto expression from a MIR `ConstValue::Scalar`.
    ///
    /// A `Scalar` is a constant too small/simple to require an `Allocation` such as:
    /// 1. integers
    /// 2. ZST, or transparent structs of one (scalar) value
    /// 3. enums that don't carry data
    /// 4. unit, tuples (may be multi-ary!), or size-0 arrays
    /// 5. pointers to an allocation
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
                    Expr::int_constant(s.to_target_isize(self).unwrap(), Type::ssize_t())
                }
            },
            (Scalar::Int(_), ty::Uint(it)) => match it {
                UintTy::U8 => Expr::int_constant(s.to_u8().unwrap(), Type::unsigned_int(8)),
                UintTy::U16 => Expr::int_constant(s.to_u16().unwrap(), Type::unsigned_int(16)),
                UintTy::U32 => Expr::int_constant(s.to_u32().unwrap(), Type::unsigned_int(32)),
                UintTy::U64 => Expr::int_constant(s.to_u64().unwrap(), Type::unsigned_int(64)),
                UintTy::U128 => Expr::int_constant(s.to_u128().unwrap(), Type::unsigned_int(128)),
                UintTy::Usize => {
                    Expr::int_constant(s.to_target_usize(self).unwrap(), Type::size_t())
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
                Expr::int_constant(s.to_u64().unwrap(), Type::unsigned_int(64))
                    .cast_to(self.codegen_ty(tm.ty).to_pointer())
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
                        let field = &variant.fields[0usize.into()];
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
                                    let fty = variant.fields[0usize.into()].ty(self.tcx, subst);
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
            (Scalar::Int(int), ty::Tuple(_)) => {
                // A ScalarInt has a u128-typed data field, so the result can never be larger than
                // that and the conversion to a uint (of an actual size that may be smaller than
                // 128 bits) will succeed.
                let int_u128 = int.try_to_uint(int.size()).ok().unwrap();
                let overall_t = self.codegen_ty(ty);
                let expr_int = Expr::int_constant(
                    int_u128,
                    Type::unsigned_int(overall_t.sizeof_in_bits(&self.symbol_table)),
                );
                expr_int.transmute_to(overall_t, &self.symbol_table)
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

    /// A private helper for `codegen_scalar`. Many "scalars" are more complex types, but get treated as scalars
    /// because they only have one (small) field. We still translated them as struct types, however.
    fn codegen_single_variant_single_field(
        &mut self,
        s: Scalar,
        span: Option<&Span>,
        overall_t: Type,
        fty: Ty<'tcx>,
    ) -> Expr {
        if fty.is_unit() {
            // TODO: It's not clear if this case is reachable. It's not covered by our test suite at least.
            Expr::struct_expr_from_values(overall_t, vec![], &self.symbol_table)
        } else {
            Expr::struct_expr_from_values(
                overall_t,
                vec![self.codegen_scalar(s, fty, span)],
                &self.symbol_table,
            )
        }
    }

    /// A private helper function that ensures `alloc_id` is "allocated" (exists in the global symbol table and is
    /// initialized), and just returns a pointer to somewhere (using `offset`) inside it.
    fn codegen_alloc_pointer(
        &mut self,
        res_t: Type,
        alloc_id: AllocId,
        offset: Size,
        span: Option<&Span>,
    ) -> Expr {
        let base_addr = match self.tcx.global_alloc(alloc_id) {
            GlobalAlloc::Function(instance) => {
                // We want to return the function pointer (not to be confused with function item)
                self.codegen_func_expr(instance, span).address_of()
            }
            GlobalAlloc::Static(def_id) => self.codegen_static_pointer(def_id, false),
            GlobalAlloc::Memory(alloc) => {
                // Full (mangled) crate name added so that allocations from different
                // crates do not conflict. The name alone is insufficient because Rust
                // allows different versions of the same crate to be used.
                let name = format!("{}::{alloc_id:?}", self.full_crate_name());
                self.codegen_const_allocation(alloc.inner(), Some(name))
            }
            GlobalAlloc::VTable(ty, trait_ref) => {
                // This is similar to GlobalAlloc::Memory but the type is opaque to rust and it
                // requires a bit more logic to get information about the allocation.
                let alloc_id = self.tcx.vtable_allocation((ty, trait_ref));
                let alloc = self.tcx.global_alloc(alloc_id).unwrap_memory();
                let name = format!("{}::{alloc_id:?}", self.full_crate_name());
                self.codegen_const_allocation(alloc.inner(), Some(name))
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

    /// Generate a goto expression for a pointer to a static or thread-local variable.
    ///
    /// These are not initialized here, see `codegen_static`.
    pub fn codegen_static_pointer(&mut self, def_id: DefId, is_thread_local: bool) -> Expr {
        let instance = Instance::mono(self.tcx, def_id);

        let sym = self.ensure(&self.symbol_name(instance), |ctx, name| {
            // Rust has a notion of "extern static" variables. These are in an "extern" block,
            // and so aren't initialized in the current codegen unit. For example (from std):
            //      extern "C" {
            //          #[linkage = "extern_weak"]
            //          static __dso_handle: *mut u8;
            //          #[linkage = "extern_weak"]
            //          static __cxa_thread_atexit_impl: *const libc::c_void;
            //      }
            // CBMC shares C's notion of "extern" global variables. However, CBMC mostly does
            // not use this information except when doing C typechecking.
            // The one exception is handling static variables with no initializer (see
            // CBMC's `static_lifetime_init`):
            //   1. If they are `is_extern` they are nondet-initialized.
            //   2. If they are `!is_extern`, they are zero-initialized.
            // So we recognize a Rust "extern" declaration and pass that information along.
            let is_extern = ctx.tcx.is_foreign_item(def_id);

            let span = ctx.tcx.def_span(def_id);
            Symbol::static_variable(
                name.to_string(),
                name.to_string(),
                ctx.codegen_ty(instance.ty(ctx.tcx, ty::ParamEnv::reveal_all())),
                ctx.codegen_span(&span),
            )
            .with_is_extern(is_extern)
            .with_is_thread_local(is_thread_local)
        });
        sym.clone().to_expr().address_of()
    }

    /// Generate an expression that represents the address for a constant allocation.
    ///
    /// This function will only allocate a new memory location if necessary. The standard does
    /// not offer any guarantees over the location of a constant.
    ///
    /// These constants can be named constants which are declared by the user, or constant values
    /// used scattered throughout the source
    fn codegen_const_allocation(&mut self, alloc: &'tcx Allocation, name: Option<String>) -> Expr {
        debug!(?name, "codegen_const_allocation");
        assert_eq!(
            alloc.mutability,
            Mutability::Not,
            "Expected constant allocation for `{name:?}`, but got a mutable instead"
        );
        if !self.alloc_map.contains_key(&alloc) {
            let name = if let Some(name) = name { name } else { self.next_global_name() };
            self.codegen_alloc_in_memory(alloc, name);
        }

        let mem_place =
            self.symbol_table.lookup(&self.alloc_map.get(&alloc).unwrap()).unwrap().to_expr();
        mem_place.address_of()
    }

    /// Insert an allocation into the goto symbol table, and generate a goto function that will
    /// initialize it.
    ///
    /// This function is ultimately responsible for creating new statically initialized global variables
    /// in our goto binaries.
    pub fn codegen_alloc_in_memory(&mut self, alloc: &'tcx Allocation, name: String) {
        debug!("codegen_alloc_in_memory name: {}", name);
        let struct_name = &format!("{name}::struct");

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

    /// This is an internal helper function for `codegen_alloc_in_memory` and you should understand
    /// it by starting there.
    ///
    /// We codegen global statics as their own unique struct types, and this creates a field-by-field
    /// representation of what those fields should be initialized with.
    /// (A field is either bytes, or initialized with an expression.)
    fn codegen_allocation_data(&mut self, alloc: &'tcx Allocation) -> Vec<AllocData<'tcx>> {
        let mut alloc_vals = Vec::with_capacity(alloc.provenance().ptrs().len() + 1);
        let pointer_size =
            Size::from_bytes(self.symbol_table.machine_model().pointer_width_in_bytes());

        let mut next_offset = Size::ZERO;
        for &(offset, alloc_id) in alloc.provenance().ptrs().iter() {
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

    /// Generate a goto expression for a MIR "function item" reference.
    ///
    /// A "function item" is a ZST that corresponds to a specific single function.
    /// This is not the closure, nor a function pointer.
    ///
    /// Unlike closures or pointers, which can point to anything of the correct type,
    /// a function item is a type associated with a unique function.
    /// This type has impls for e.g. Fn, FnOnce, etc, which is how it safely converts to other
    /// function types.
    ///
    /// See <https://doc.rust-lang.org/reference/types/function-item.html>
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

    /// Ensure that the given instance is in the symbol table, returning the symbol.
    ///
    /// FIXME: The function should not have to return the type of the function symbol as well
    /// because the symbol should have the type. The problem is that the type in the symbol table
    /// sometimes subtly differs from the type that codegen_function_sig returns.
    /// This is tracked in <https://github.com/model-checking/kani/issues/1350>.
    fn codegen_func_symbol(&mut self, instance: Instance<'tcx>) -> (&Symbol, Type) {
        let funct = self.codegen_function_sig(self.fn_sig_of_instance(instance));
        let sym = if self.tcx.is_foreign_item(instance.def_id()) {
            // Get the symbol that represents a foreign instance.
            self.codegen_foreign_fn(instance)
        } else {
            // All non-foreign functions should've been declared beforehand.
            trace!(func=?instance, "codegen_func_symbol");
            let func = self.symbol_name(instance);
            self.symbol_table
                .lookup(&func)
                .unwrap_or_else(|| panic!("Function `{func}` should've been declared before usage"))
        };
        (sym, funct)
    }

    /// Generate a goto expression that references the function identified by `instance`.
    ///
    /// Note: In general with this `Expr` you should immediately either `.address_of()` or `.call(...)`.
    ///
    /// This should not be used where Rust expects a "function item" (See `codegen_fn_item`)
    pub fn codegen_func_expr(&mut self, instance: Instance<'tcx>, span: Option<&Span>) -> Expr {
        let (func_symbol, func_typ) = self.codegen_func_symbol(instance);
        Expr::symbol_expression(func_symbol.name, func_typ)
            .with_location(self.codegen_span_option(span.cloned()))
    }

    /// Generate a goto expression referencing the singleton value for a MIR "function item".
    ///
    /// For a given function instance, generate a ZST struct and return a singleton reference to that.
    /// This is the Rust "function item". See <https://doc.rust-lang.org/reference/types/function-item.html>
    /// This is not the function pointer, for that use `codegen_func_expr`.
    fn codegen_fn_item(&mut self, instance: Instance<'tcx>, span: Option<&Span>) -> Expr {
        let (func_symbol, _) = self.codegen_func_symbol(instance);
        let mangled_name = func_symbol.name;
        let fn_item_struct_ty = self.codegen_fndef_type(instance);
        // This zero-sized object that a function name refers to in Rust is globally unique, so we create such a global object.
        let fn_singleton_name = format!("{mangled_name}::FnDefSingleton");
        let fn_singleton = self.ensure_global_var(
            &fn_singleton_name,
            false,
            fn_item_struct_ty,
            Location::none(),
            |_, _| None, // zero-sized, so no initialization necessary
        );
        fn_singleton.with_location(self.codegen_span_option(span.cloned()))
    }
}
