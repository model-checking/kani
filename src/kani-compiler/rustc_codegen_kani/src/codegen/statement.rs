// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
use super::typ::TypeExt;
use super::typ::FN_RETURN_VOID_VAR_NAME;
use crate::{GotocCtx, VtableCtx};
use cbmc::goto_program::{BuiltinFn, Expr, ExprValue, Location, Stmt, Type};
use rustc_hir::def_id::DefId;
use rustc_middle::mir;
use rustc_middle::mir::{
    BasicBlock, Operand, Place, Statement, StatementKind, SwitchTargets, Terminator, TerminatorKind,
};
use rustc_middle::ty;
use rustc_middle::ty::layout::LayoutOf;
use rustc_middle::ty::{Instance, InstanceDef, Ty};
use rustc_span::Span;
use rustc_target::abi::{FieldsShape, Primitive, TagEncoding, Variants};
use tracing::debug;

impl<'tcx> GotocCtx<'tcx> {
    fn codegen_ret_unit(&mut self) -> Stmt {
        let is_file_local = false;
        let ty = self.codegen_ty_unit();
        let var = self.ensure_global_var(
            FN_RETURN_VOID_VAR_NAME,
            is_file_local,
            ty,
            Location::none(),
            |_, _| None,
        );
        Stmt::ret(Some(var), Location::none())
    }

    pub fn codegen_terminator(&mut self, term: &Terminator<'tcx>) -> Stmt {
        let loc = self.codegen_span(&term.source_info.span);
        debug!("handling terminator {:?}", term);
        //TODO: Instead of doing location::none(), and updating, just putit in when we make the stmt.
        match &term.kind {
            TerminatorKind::Goto { target } => {
                Stmt::goto(self.current_fn().find_label(target), loc)
            }
            TerminatorKind::SwitchInt { discr, switch_ty, targets } => {
                self.codegen_switch_int(discr, switch_ty, targets)
            }
            TerminatorKind::Resume => Stmt::assert_false("resume instruction", loc),
            TerminatorKind::Abort => Stmt::assert_false("abort instruction", loc),
            TerminatorKind::Return => {
                let rty = self.current_fn().sig().unwrap().skip_binder().output();
                if rty.is_unit() {
                    self.codegen_ret_unit()
                } else {
                    let p = Place::from(mir::RETURN_PLACE);
                    let v = self.codegen_place(&p).goto_expr;
                    if self.place_ty(&p).is_bool() {
                        v.cast_to(Type::c_bool()).ret(loc)
                    } else {
                        v.ret(loc)
                    }
                }
            }
            TerminatorKind::Unreachable => Stmt::block(
                vec![
                    Stmt::assert_false("unreachable code", loc.clone()),
                    Stmt::assume(Expr::bool_false(), loc.clone()),
                ],
                loc,
            ),
            TerminatorKind::Drop { place, target, unwind: _ } => self.codegen_drop(place, target),
            TerminatorKind::DropAndReplace { .. } => {
                unreachable!("this instruction is unreachable")
            }
            TerminatorKind::Call { func, args, destination, .. } => {
                self.codegen_funcall(func, args, destination, term.source_info.span)
            }
            TerminatorKind::Assert { cond, expected, msg, target, .. } => {
                let cond = {
                    let r = self.codegen_operand(cond);
                    if *expected { r } else { Expr::not(r) }
                };

                Stmt::block(
                    vec![
                        cond.cast_to(Type::bool()).if_then_else(
                            Stmt::goto(self.current_fn().find_label(target), loc.clone()),
                            None,
                            loc.clone(),
                        ),
                        Stmt::assert_false(&format!("{:?}", msg), loc.clone()),
                        Stmt::goto(self.current_fn().find_label(target), loc.clone()),
                    ],
                    loc,
                )
            }
            TerminatorKind::Yield { .. }
            | TerminatorKind::GeneratorDrop
            | TerminatorKind::FalseEdge { .. }
            | TerminatorKind::FalseUnwind { .. } => unreachable!("we should not hit these cases"),
            TerminatorKind::InlineAsm { .. } => self
                .codegen_unimplemented(
                    "InlineAsm",
                    Type::empty(),
                    loc.clone(),
                    "https://github.com/model-checking/rmc/issues/2",
                )
                .as_stmt(loc),
        }
    }

    // TODO: this function doesn't handle unwinding which begins if the destructor panics
    // https://github.com/model-checking/rmc/issues/221
    fn codegen_drop(&mut self, location: &Place<'tcx>, target: &BasicBlock) -> Stmt {
        let loc_ty = self.place_ty(location);
        let drop_instance = Instance::resolve_drop_in_place(self.tcx, loc_ty);
        if let Some(hk) = self.hooks.hook_applies(self.tcx, drop_instance) {
            let le = self.codegen_place(location).goto_expr;
            hk.handle(self, drop_instance, vec![le], None, Some(*target), None)
        } else {
            let drop_implementation = match drop_instance.def {
                InstanceDef::DropGlue(_, None) => {
                    // We can skip empty DropGlue functions
                    Stmt::skip(Location::none())
                }
                _ => {
                    match loc_ty.kind() {
                        ty::Dynamic(..) => {
                            // Virtual drop via a vtable lookup
                            let trait_fat_ptr =
                                self.codegen_place(location).fat_ptr_goto_expr.unwrap();

                            // Pull the function off of the fat pointer's vtable pointer
                            let vtable_ref =
                                trait_fat_ptr.to_owned().member("vtable", &self.symbol_table);
                            let vtable = vtable_ref.dereference();
                            let fn_ptr = vtable.member("drop", &self.symbol_table);

                            // Pull the self argument off of the fat pointer's data pointer
                            let self_ref =
                                trait_fat_ptr.to_owned().member("data", &self.symbol_table);
                            let self_ref =
                                self_ref.cast_to(trait_fat_ptr.typ().clone().to_pointer());

                            let call = if self.vtable_ctx.emit_vtable_restrictions {
                                self.virtual_call_with_restricted_fn_ptr(
                                    trait_fat_ptr.typ().clone(),
                                    VtableCtx::drop_index(),
                                    fn_ptr,
                                    vec![self_ref],
                                )
                            } else {
                                fn_ptr.dereference().call(vec![self_ref])
                            };
                            call.as_stmt(Location::none())
                        }
                        _ => {
                            // Non-virtual, direct drop call
                            assert!(!matches!(drop_instance.def, InstanceDef::Virtual(_, _)));

                            let func = self.codegen_func_expr(drop_instance, None);
                            let place = self.codegen_place(location);
                            let arg = if let Some(fat_ptr) = place.fat_ptr_goto_expr {
                                // Drop takes the fat pointer if it exists
                                fat_ptr
                            } else {
                                place.goto_expr.address_of()
                            };
                            // The only argument should be a self reference
                            let args = vec![arg];

                            // We have a known issue where nested Arc and Mutex objects result in
                            // drop_in_place call implementations that fail to typecheck. Skipping
                            // drop entirely causes unsound verification results in common cases
                            // like vector extend, so for now, add a sound special case workaround
                            // for calls that fail the typecheck.
                            // https://github.com/model-checking/rmc/issues/426
                            // Unblocks: https://github.com/model-checking/rmc/issues/435
                            if Expr::typecheck_call(&func, &args) {
                                func.call(args)
                            } else {
                                self.codegen_unimplemented(
                                    format!("drop_in_place call for {:?}", func).as_str(),
                                    func.typ().return_type().unwrap().clone(),
                                    Location::none(),
                                    "https://github.com/model-checking/rmc/issues/426",
                                )
                            }
                            .as_stmt(Location::none())
                        }
                    }
                }
            };
            let goto_target = Stmt::goto(self.current_fn().find_label(target), Location::none());
            let block = vec![drop_implementation, goto_target];
            Stmt::block(block, Location::none())
        }
    }

    /// https://doc.rust-lang.org/nightly/nightly-rustc/rustc_middle/mir/terminator/enum.TerminatorKind.html#variant.SwitchInt
    /// Operand evaluates to an integer;
    /// jump depending on its value to one of the targets, and otherwise fallback to otherwise.
    /// The otherwise value is stores as the last value of targets.
    fn codegen_switch_int(
        &mut self,
        discr: &Operand<'tcx>,
        switch_ty: Ty<'tcx>,
        targets: &SwitchTargets,
    ) -> Stmt {
        let v = self.codegen_operand(discr);
        let switch_ty = self.monomorphize(switch_ty);
        if targets.all_targets().len() == 1 {
            // Translate to a guarded goto
            let first_target = targets.iter().next().unwrap();
            Stmt::block(
                vec![
                    v.eq(Expr::int_constant(first_target.0, self.codegen_ty(switch_ty)))
                        .if_then_else(
                            Stmt::goto(
                                self.current_fn().find_label(&first_target.1),
                                Location::none(),
                            ),
                            None,
                            Location::none(),
                        ),
                    Stmt::goto(
                        self.current_fn().find_label(&targets.otherwise()),
                        Location::none(),
                    ),
                ],
                Location::none(),
            )
        } else {
            // Switches with empty targets should've been eliminated already.
            assert!(targets.all_targets().len() > 1);
            let cases = targets
                .iter()
                .map(|(c, bb)| {
                    Expr::int_constant(c, self.codegen_ty(switch_ty)).switch_case(Stmt::goto(
                        self.current_fn().find_label(&bb),
                        Location::none(),
                    ))
                })
                .collect();
            let default =
                Stmt::goto(self.current_fn().find_label(&targets.otherwise()), Location::none());
            v.switch(cases, Some(default), Location::none())
        }
    }

    fn codegen_untuple_closure_args(
        &mut self,
        instance: Instance<'tcx>,
        fargs: &mut Vec<Expr>,
        last_mir_arg: Option<&Operand<'tcx>>,
    ) {
        debug!(
            "codegen_untuple_closure_args instance: {:?}, fargs {:?}",
            self.readable_instance_name(instance),
            fargs
        );
        // A closure takes two arguments:
        //     0. a struct representing the environment
        //     1. a tuple containing the parameters
        //
        // However, for some reason, Rust decides to generate a function which still
        // takes the first argument as the environment struct, but the tuple of parameters
        // are flattened as subsequent parameters.
        // Therefore, we have to project out the corresponding fields when we detect
        // an invocation of a closure.
        //
        // Note: In some cases, the environment struct has type FnDef, so we skip it in
        // ignore_var_ty. So the tuple is always the last arg, but it might be in the
        // first or the second position.
        // Note 2: For empty closures, the only argument needed is the environment struct.
        if fargs.len() > 0 {
            let tupe = fargs.remove(fargs.len() - 1);
            let tupled_args: Vec<Type> = match self.operand_ty(last_mir_arg.unwrap()).kind() {
                ty::Tuple(tupled_args) => {
                    tupled_args.iter().map(|s| self.codegen_ty(s.expect_ty())).collect()
                }
                _ => unreachable!("Argument to function with Abi::RustCall is not a tuple"),
            };

            // Unwrap as needed
            for (i, _) in tupled_args.iter().enumerate() {
                // Access the tupled parameters through the `member` operation
                let index_param = tupe.clone().member(&i.to_string(), &self.symbol_table);
                fargs.push(index_param);
            }
        }
    }

    fn codegen_funcall(
        &mut self,
        func: &Operand<'tcx>,
        args: &[Operand<'tcx>],
        destination: &Option<(Place<'tcx>, BasicBlock)>,
        span: Span,
    ) -> Stmt {
        let loc = self.codegen_span(&span);
        let funct = self.operand_ty(func);
        let mut fargs: Vec<_> = args
            .iter()
            .filter_map(|o| {
                let ot = self.operand_ty(o);
                if self.ignore_var_ty(ot) {
                    None
                } else if ot.is_bool() {
                    Some(self.codegen_operand(o).cast_to(Type::c_bool()))
                } else {
                    Some(self.codegen_operand(o))
                }
            })
            .collect();
        match &funct.kind() {
            ty::FnDef(defid, subst) => {
                let instance =
                    Instance::resolve(self.tcx, ty::ParamEnv::reveal_all(), *defid, subst)
                        .unwrap()
                        .unwrap();

                if self.ty_needs_closure_untupled(funct) {
                    self.codegen_untuple_closure_args(instance, &mut fargs, args.last());
                }

                if let Some(hk) = self.hooks.hook_applies(self.tcx, instance) {
                    return hk.handle(
                        self,
                        instance,
                        fargs,
                        destination.map(|t| t.0),
                        destination.map(|t| t.1),
                        Some(span),
                    );
                }

                if destination.is_none() {
                    // No target block means this function doesn't return.
                    // This should have been handled by the Nevers hook.
                    return Stmt::assert_false(
                        &format!("reach some nonterminating function: {:?}", func),
                        loc.clone(),
                    );
                }

                let (p, target) = destination.unwrap();

                let mut stmts: Vec<Stmt> = match instance.def {
                    // Here an empty drop glue is invoked; we just ignore it.
                    InstanceDef::DropGlue(_, None) => {
                        return Stmt::goto(self.current_fn().find_label(&target), Location::none());
                    }
                    // Handle a virtual function call via a vtable lookup
                    InstanceDef::Virtual(def_id, idx) => {
                        // We must have at least one argument, and the first one
                        // should be a fat pointer for the trait
                        let trait_fat_ptr = fargs[0].to_owned();

                        //Check the Gotoc-level fat pointer type
                        assert!(
                            trait_fat_ptr.typ().is_rust_trait_fat_ptr(&self.symbol_table),
                            "Expected fat pointer, got:\n{:?}",
                            trait_fat_ptr,
                        );

                        self.codegen_virtual_funcall(
                            trait_fat_ptr,
                            def_id,
                            idx,
                            &p,
                            &mut fargs,
                            loc.clone(),
                        )
                    }
                    // Normal, non-virtual function calls
                    InstanceDef::Item(..)
                    | InstanceDef::DropGlue(_, Some(_))
                    | InstanceDef::Intrinsic(..)
                    | InstanceDef::FnPtrShim(.., _)
                    | InstanceDef::VtableShim(..)
                    | InstanceDef::ReifyShim(..)
                    | InstanceDef::ClosureOnceShim { .. }
                    | InstanceDef::CloneShim(..) => {
                        let func_exp = self.codegen_operand(func);
                        vec![
                            self.codegen_expr_to_place(&p, func_exp.call(fargs))
                                .with_location(loc.clone()),
                        ]
                    }
                };
                stmts.push(Stmt::goto(self.current_fn().find_label(&target), loc.clone()));
                return Stmt::block(stmts, loc);
            }
            // Function call through a pointer
            ty::FnPtr(_) => {
                let (p, target) = destination.unwrap();
                let func_expr = self.codegen_operand(func).dereference();
                // Actually generate the function call and return.
                return Stmt::block(
                    vec![
                        self.codegen_expr_to_place(&p, func_expr.call(fargs))
                            .with_location(loc.clone()),
                        Stmt::goto(self.current_fn().find_label(&target), loc.clone()),
                    ],
                    loc,
                );
            }
            x => unreachable!("Function call where the function was of unexpected type: {:?}", x),
        };
    }

    fn codegen_virtual_funcall(
        &mut self,
        trait_fat_ptr: Expr,
        def_id: DefId,
        idx: usize,
        place: &Place<'tcx>,
        fargs: &mut Vec<Expr>,
        loc: Location,
    ) -> Vec<Stmt> {
        let vtable_field_name = self.vtable_field_name(def_id, idx);

        // Now that we have all the stuff we need, we can actually build the dynamic call
        // If the original call was of the form
        // f(arg0, arg1);
        // The new call should be of the form
        // arg0.vtable->f(arg0.data,arg1);
        let vtable_ref = trait_fat_ptr.to_owned().member("vtable", &self.symbol_table);
        let vtable = vtable_ref.dereference();
        let fn_ptr = vtable.member(vtable_field_name, &self.symbol_table);

        // Update the argument from arg0 to arg0.data
        fargs[0] = trait_fat_ptr.to_owned().member("data", &self.symbol_table);

        // For soundness, add an assertion that the vtable function call is not null.
        // Otherwise, CBMC might treat this as an assert(0) and later user-added assertions
        // could be vacuously true.
        let call_is_nonnull = fn_ptr.clone().is_nonnull();
        let assert_msg = format!("Non-null virtual function call for {:?}", vtable_field_name);
        let assert_nonnull = Stmt::assert(call_is_nonnull, &assert_msg, loc.clone());

        // Virtual function call and corresponding nonnull assertion.
        let call = if self.vtable_ctx.emit_vtable_restrictions {
            self.virtual_call_with_restricted_fn_ptr(
                trait_fat_ptr.typ().clone(),
                idx,
                fn_ptr,
                fargs.to_vec(),
            )
        } else {
            fn_ptr.dereference().call(fargs.to_vec())
        };
        vec![assert_nonnull, self.codegen_expr_to_place(place, call).with_location(loc.clone())]
    }

    /// A place is similar to the C idea of a LHS. For example, the returned value of a function call is stored to a place.
    /// If the place is unit (i.e. the statement value is not stored anywhere), then we can just turn it directly to a statement.
    /// Otherwise, we assign the value of the expression to the place.
    pub fn codegen_expr_to_place(&mut self, p: &Place<'tcx>, e: Expr) -> Stmt {
        if self.place_ty(p).is_unit() {
            e.as_stmt(Location::none())
        } else {
            self.codegen_place(&p).goto_expr.assign(e, Location::none())
        }
    }

    pub fn codegen_panic(&self, span: Option<Span>, fargs: Vec<Expr>) -> Stmt {
        // CBMC requires that the argument to the assertion must be a string constant.
        // If there is one in the MIR, use it; otherwise, explain that we can't.
        assert!(!fargs.is_empty(), "Panic requires a string message");
        let msg = extract_const_message(&fargs[0]).unwrap_or(String::from(
            "This is a placeholder message; Kani doesn't support message formatted at runtime",
        ));

        self.codegen_fatal_error(&msg, span)
    }

    // Generate code for fatal error which should trigger an assertion failure and abort the
    // execution.
    pub fn codegen_fatal_error(&self, msg: &str, span: Option<Span>) -> Stmt {
        let loc = self.codegen_caller_span(&span);
        Stmt::block(
            vec![
                Stmt::assert_false(msg, loc.clone()),
                BuiltinFn::Abort.call(vec![], loc.clone()).as_stmt(loc.clone()),
            ],
            loc,
        )
    }

    pub fn codegen_statement(&mut self, stmt: &Statement<'tcx>) -> Stmt {
        debug!("handling statement {:?}", stmt);
        match &stmt.kind {
            StatementKind::Assign(box (l, r)) => {
                let lty = self.place_ty(l);
                let rty = self.rvalue_ty(r);
                let llayout = self.layout_of(lty);
                // we ignore assignment for all zero size types
                if llayout.is_zst() {
                    Stmt::skip(Location::none())
                } else if lty.is_fn_ptr() && rty.is_fn() && !rty.is_fn_ptr() {
                    // implicit address of a function pointer, e.g.
                    // let fp: fn() -> i32 = foo;
                    // where the reference is implicit.
                    self.codegen_place(l)
                        .goto_expr
                        .assign(self.codegen_rvalue(r).address_of(), Location::none())
                } else if rty.is_bool() {
                    self.codegen_place(l)
                        .goto_expr
                        .assign(self.codegen_rvalue(r).cast_to(Type::c_bool()), Location::none())
                } else {
                    self.codegen_place(l).goto_expr.assign(self.codegen_rvalue(r), Location::none())
                }
            }
            StatementKind::SetDiscriminant { place, variant_index } => {
                // this requires place points to an enum type.
                let pt = self.place_ty(place);
                let (def, _) = match pt.kind() {
                    ty::Adt(def, substs) => (def, substs),
                    ty::Generator(..) => {
                        return self
                            .codegen_unimplemented(
                                "ty::Generator",
                                Type::code(vec![], Type::empty()),
                                Location::none(),
                                "https://github.com/model-checking/rmc/issues/416",
                            )
                            .as_stmt(Location::none());
                    }
                    _ => unreachable!(),
                };
                let layout = self.layout_of(pt);
                match &layout.variants {
                    Variants::Single { .. } => Stmt::skip(Location::none()),
                    Variants::Multiple { tag, tag_encoding, .. } => match tag_encoding {
                        TagEncoding::Direct => {
                            let discr = def.discriminant_for_variant(self.tcx, *variant_index);
                            let discr_t = self.codegen_enum_discr_typ(pt);
                            let discr = Expr::int_constant(discr.val, self.codegen_ty(discr_t));
                            self.codegen_place(place)
                                .goto_expr
                                .member("case", &self.symbol_table)
                                .assign(discr, Location::none())
                        }
                        TagEncoding::Niche { dataful_variant, niche_variants, niche_start } => {
                            if dataful_variant != variant_index {
                                let offset = match &layout.fields {
                                    FieldsShape::Arbitrary { offsets, .. } => {
                                        offsets[0].bytes_usize()
                                    }
                                    _ => unreachable!("niche encoding must have arbitrary fields"),
                                };
                                let discr_ty = self.codegen_enum_discr_typ(pt);
                                let discr_ty = self.codegen_ty(discr_ty);
                                let niche_value =
                                    variant_index.as_u32() - niche_variants.start().as_u32();
                                let niche_value = (niche_value as u128).wrapping_add(*niche_start);
                                let value = if niche_value == 0 && tag.value == Primitive::Pointer {
                                    discr_ty.null()
                                } else {
                                    Expr::int_constant(niche_value, discr_ty.clone())
                                };
                                let place = self.codegen_place(place).goto_expr;
                                self.codegen_get_niche(place, offset, discr_ty)
                                    .assign(value, Location::none())
                            } else {
                                Stmt::skip(Location::none())
                            }
                        }
                    },
                }
            }
            StatementKind::StorageLive(_) => Stmt::skip(Location::none()), // TODO: fix me
            StatementKind::StorageDead(_) => Stmt::skip(Location::none()), // TODO: fix me
            StatementKind::CopyNonOverlapping(box mir::CopyNonOverlapping {
                ref src,
                ref dst,
                ref count,
            }) => {
                let src = self.codegen_operand(src).cast_to(Type::void_pointer());
                let dst = self.codegen_operand(dst);
                let count = self.codegen_operand(count);
                let sz = dst.typ().base_type().unwrap().sizeof(&self.symbol_table);
                let sz = Expr::int_constant(sz, Type::size_t());
                let n = sz.mul(count);
                let dst = dst.cast_to(Type::void_pointer());
                let e = BuiltinFn::Memcpy.call(vec![dst, src, n.clone()], Location::none());

                // The C implementation of memcpy does not allow an invalid pointer for
                // the src/dst, but the LLVM implementation specifies that a copy with
                // length zero is a no-op. This comes up specifically when handling
                // the empty string; CBMC will fail on passing a reference to empty
                // string unless we codegen this zero check.
                // https://llvm.org/docs/LangRef.html#llvm-memcpy-intrinsic
                Stmt::if_then_else(
                    n.is_zero().not(),
                    e.as_stmt(Location::none()),
                    None,
                    Location::none(),
                )
            }
            StatementKind::FakeRead(_)
            | StatementKind::Retag(_, _)
            | StatementKind::AscribeUserType(_, _)
            | StatementKind::Nop
            | StatementKind::Coverage { .. } => Stmt::skip(Location::none()),
        }
        .with_location(self.codegen_span(&stmt.source_info.span))
    }
}

/// Tries to extract a string message from an `Expr`.
/// If the expression represents a pointer to a string constant, this will return the string
/// constant. Otherwise, return `None`.
fn extract_const_message(arg: &Expr) -> Option<String> {
    match arg.value() {
        ExprValue::Struct { values } => match &values[0].value() {
            ExprValue::AddressOf(address) => match address.value() {
                ExprValue::Index { array, .. } => match array.value() {
                    ExprValue::StringConstant { s } => Some(s.to_string()),
                    _ => None,
                },
                _ => None,
            },
            _ => None,
        },
        _ => None,
    }
}
