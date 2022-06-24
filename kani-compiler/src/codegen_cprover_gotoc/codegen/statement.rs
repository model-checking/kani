// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
use super::typ::TypeExt;
use super::typ::FN_RETURN_VOID_VAR_NAME;
use super::PropertyClass;
use crate::codegen_cprover_gotoc::codegen::typ::pointee_type;
use crate::codegen_cprover_gotoc::utils;
use crate::codegen_cprover_gotoc::{GotocCtx, VtableCtx};
use crate::unwrap_or_return_codegen_unimplemented_stmt;
use cbmc::goto_program::{BuiltinFn, Expr, Location, Stmt, Type};
use cbmc::utils::BUG_REPORT_URL;
use kani_queries::UserInput;
use rustc_hir::def_id::DefId;
use rustc_middle::mir;
use rustc_middle::mir::{
    AssertKind, BasicBlock, Operand, Place, Statement, StatementKind, SwitchTargets, Terminator,
    TerminatorKind,
};
use rustc_middle::ty;
use rustc_middle::ty::layout::LayoutOf;
use rustc_middle::ty::{Instance, InstanceDef, Ty};
use rustc_span::Span;
use rustc_target::abi::{FieldsShape, Primitive, TagEncoding, Variants};
use tracing::{debug, info_span, warn};

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
        let _trace_span = info_span!("CodegenTerminator", statement = ?term.kind).entered();
        debug!("handling terminator {:?}", term);
        //TODO: Instead of doing location::none(), and updating, just putit in when we make the stmt.
        match &term.kind {
            TerminatorKind::Goto { target } => {
                Stmt::goto(self.current_fn().find_label(target), loc)
            }
            TerminatorKind::SwitchInt { discr, switch_ty, targets } => {
                self.codegen_switch_int(discr, *switch_ty, targets)
            }
            TerminatorKind::Resume => self.codegen_assert_false(
                PropertyClass::UnsupportedConstruct,
                "resume instruction",
                loc,
            ),
            TerminatorKind::Abort => self.codegen_assert_false(
                PropertyClass::UnsupportedConstruct,
                "abort instruction",
                loc,
            ),
            TerminatorKind::Return => {
                let rty = self.current_fn().sig().unwrap().skip_binder().output();
                if rty.is_unit() {
                    self.codegen_ret_unit()
                } else {
                    let p = Place::from(mir::RETURN_PLACE);
                    let v =
                        unwrap_or_return_codegen_unimplemented_stmt!(self, self.codegen_place(&p))
                            .goto_expr;
                    if self.place_ty(&p).is_bool() {
                        v.cast_to(Type::c_bool()).ret(loc)
                    } else {
                        v.ret(loc)
                    }
                }
            }
            TerminatorKind::Unreachable => Stmt::block(
                vec![
                    self.codegen_assert_false(PropertyClass::Unreachable, "unreachable code", loc),
                    Stmt::assume(Expr::bool_false(), loc),
                ],
                loc,
            ),
            TerminatorKind::Drop { place, target, unwind: _ } => self.codegen_drop(place, target),
            TerminatorKind::DropAndReplace { .. } => {
                unreachable!("this instruction is unreachable")
            }
            TerminatorKind::Call { func, args, destination, target, .. } => {
                self.codegen_funcall(func, args, destination, target, term.source_info.span)
            }
            TerminatorKind::Assert { cond, expected, msg, target, .. } => {
                let cond = {
                    let r = self.codegen_operand(cond);
                    if *expected { r } else { Expr::not(r) }
                };

                let msg = if let AssertKind::BoundsCheck { .. } = msg {
                    // For bounds check the following panic message is generated at runtime:
                    // "index out of bounds: the length is {len} but the index is {index}",
                    // but CBMC only accepts static messages so we don't add values to the message.
                    "index out of bounds: the length is less than or equal to the given index"
                } else {
                    // For all other assert kind we can get the static message.
                    msg.description()
                };

                // TODO: switch to tagging assertions via the property class once CBMC allows that:
                // https://github.com/diffblue/cbmc/issues/6692
                let (msg_str, reach_stmt) = if self.queries.get_check_assertion_reachability() {
                    let check_id = self.next_check_id();
                    let msg_str = GotocCtx::add_prefix_to_msg(msg, &check_id);
                    let reach_msg = GotocCtx::reachability_check_message(&check_id);
                    (msg_str, self.codegen_cover_loc(&reach_msg, Some(term.source_info.span)))
                } else {
                    (msg.to_string(), Stmt::skip(loc))
                };

                Stmt::block(
                    vec![
                        reach_stmt,
                        cond.cast_to(Type::bool()).if_then_else(
                            Stmt::goto(self.current_fn().find_label(target), loc),
                            None,
                            loc,
                        ),
                        self.codegen_assert_false(PropertyClass::Assertion, &msg_str, loc),
                        Stmt::goto(self.current_fn().find_label(target), loc),
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
                    loc,
                    "https://github.com/model-checking/kani/issues/2",
                )
                .as_stmt(loc),
        }
    }

    // TODO: this function doesn't handle unwinding which begins if the destructor panics
    // https://github.com/model-checking/kani/issues/221
    fn codegen_drop(&mut self, location: &Place<'tcx>, target: &BasicBlock) -> Stmt {
        let loc_ty = self.place_ty(location);
        debug!(?loc_ty, "codegen_drop");
        let drop_instance = Instance::resolve_drop_in_place(self.tcx, loc_ty);
        // Once upon a time we did a `hook_applies` check here, but we no longer seem to hook drops
        let drop_implementation = match drop_instance.def {
            InstanceDef::DropGlue(_, None) => {
                // We can skip empty DropGlue functions
                Stmt::skip(Location::none())
            }
            _ => {
                match loc_ty.kind() {
                    ty::Dynamic(..) => {
                        // Virtual drop via a vtable lookup
                        let trait_fat_ptr = unwrap_or_return_codegen_unimplemented_stmt!(
                            self,
                            self.codegen_place(location)
                        )
                        .fat_ptr_goto_expr
                        .unwrap();
                        debug!(?trait_fat_ptr, "codegen_drop: ");

                        // Pull the function off of the fat pointer's vtable pointer
                        let vtable_ref =
                            trait_fat_ptr.to_owned().member("vtable", &self.symbol_table);

                        let vtable = vtable_ref.dereference();
                        let fn_ptr = vtable.member("drop", &self.symbol_table);

                        // Pull the self argument off of the fat pointer's data pointer
                        if let Some(typ) = pointee_type(self.local_ty(location.local)) {
                            if !(typ.is_trait() || typ.is_box()) {
                                warn!(self_type=?typ, "Unsupported drop of unsized");
                                return self
                                    .codegen_unimplemented(
                                        format!("Unsupported drop unsized struct: {:?}", typ)
                                            .as_str(),
                                        Type::Empty,
                                        Location::None,
                                        "https://github.com/model-checking/kani/issues/1072",
                                    )
                                    .as_stmt(Location::None);
                            }
                        }
                        let self_data = trait_fat_ptr.to_owned().member("data", &self.symbol_table);
                        let self_ref = self_data.cast_to(trait_fat_ptr.typ().clone().to_pointer());

                        let call =
                            fn_ptr.dereference().call(vec![self_ref]).as_stmt(Location::none());
                        if self.vtable_ctx.emit_vtable_restrictions {
                            self.virtual_call_with_restricted_fn_ptr(
                                trait_fat_ptr.typ().clone(),
                                VtableCtx::drop_index(),
                                call,
                            )
                        } else {
                            call
                        }
                    }
                    _ => {
                        // Non-virtual, direct drop call
                        assert!(!matches!(drop_instance.def, InstanceDef::Virtual(_, _)));

                        let func = self.codegen_func_expr(drop_instance, None);
                        let place = unwrap_or_return_codegen_unimplemented_stmt!(
                            self,
                            self.codegen_place(location)
                        );
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
                        // https://github.com/model-checking/kani/issues/426
                        // Unblocks: https://github.com/model-checking/kani/issues/435
                        if Expr::typecheck_call(&func, &args) {
                            func.call(args)
                        } else {
                            self.codegen_unimplemented(
                                format!("drop_in_place call for {:?}", func).as_str(),
                                func.typ().return_type().unwrap().clone(),
                                Location::none(),
                                "https://github.com/model-checking/kani/issues/426",
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
        if !fargs.is_empty() {
            let tupe = fargs.remove(fargs.len() - 1);
            let tupled_args: Vec<Type> = match self.operand_ty(last_mir_arg.unwrap()).kind() {
                ty::Tuple(tupled_args) => tupled_args.iter().map(|s| self.codegen_ty(s)).collect(),
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

    fn codegen_end_call(&self, target: Option<&BasicBlock>, loc: Location) -> Stmt {
        if let Some(next_bb) = target {
            Stmt::goto(self.current_fn().find_label(next_bb), loc)
        } else {
            Stmt::assert_sanity_check(
                Expr::bool_false(),
                "Unexpected return from Never function",
                BUG_REPORT_URL,
                loc,
            )
        }
    }

    pub fn codegen_funcall_args(&mut self, args: &[Operand<'tcx>]) -> Vec<Expr> {
        args.iter()
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
            .collect()
    }

    fn codegen_funcall(
        &mut self,
        func: &Operand<'tcx>,
        args: &[Operand<'tcx>],
        destination: &Place<'tcx>,
        target: &Option<BasicBlock>,
        span: Span,
    ) -> Stmt {
        if self.is_intrinsic(func) {
            return self.codegen_funcall_of_intrinsic(func, args, destination, target, span);
        }

        let loc = self.codegen_span(&span);
        let funct = self.operand_ty(func);
        let mut fargs = self.codegen_funcall_args(args);
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
                    return hk.handle(self, instance, fargs, *destination, *target, Some(span));
                }

                let mut stmts: Vec<Stmt> = match instance.def {
                    // Here an empty drop glue is invoked; we just ignore it.
                    InstanceDef::DropGlue(_, None) => {
                        return Stmt::goto(
                            self.current_fn().find_label(&target.unwrap()),
                            Location::none(),
                        );
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
                            destination,
                            &mut fargs,
                            loc,
                        )
                    }
                    // Normal, non-virtual function calls
                    InstanceDef::Item(..)
                    | InstanceDef::DropGlue(_, Some(_))
                    | InstanceDef::Intrinsic(..)
                    | InstanceDef::FnPtrShim(..)
                    | InstanceDef::VtableShim(..)
                    | InstanceDef::ReifyShim(..)
                    | InstanceDef::ClosureOnceShim { .. }
                    | InstanceDef::CloneShim(..) => {
                        let func_exp = self.codegen_operand(func);
                        vec![
                            self.codegen_expr_to_place(destination, func_exp.call(fargs))
                                .with_location(loc),
                        ]
                    }
                };
                stmts.push(self.codegen_end_call(target.as_ref(), loc));
                return Stmt::block(stmts, loc);
            }
            // Function call through a pointer
            ty::FnPtr(_) => {
                let func_expr = self.codegen_operand(func).dereference();
                // Actually generate the function call and return.
                return Stmt::block(
                    vec![
                        self.codegen_expr_to_place(destination, func_expr.call(fargs))
                            .with_location(loc),
                        Stmt::goto(self.current_fn().find_label(&target.unwrap()), loc),
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
        fargs: &mut [Expr],
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
        let assert_nonnull =
            self.codegen_assert(call_is_nonnull, PropertyClass::SanityCheck, &assert_msg, loc);

        // Virtual function call and corresponding nonnull assertion.
        let call = fn_ptr.dereference().call(fargs.to_vec());
        let call_stmt = self.codegen_expr_to_place(place, call).with_location(loc);
        let call_stmt = if self.vtable_ctx.emit_vtable_restrictions {
            self.virtual_call_with_restricted_fn_ptr(trait_fat_ptr.typ().clone(), idx, call_stmt)
        } else {
            call_stmt
        };
        vec![assert_nonnull, call_stmt]
    }

    /// A place is similar to the C idea of a LHS. For example, the returned value of a function call is stored to a place.
    /// If the place is unit (i.e. the statement value is not stored anywhere), then we can just turn it directly to a statement.
    /// Otherwise, we assign the value of the expression to the place.
    pub fn codegen_expr_to_place(&mut self, p: &Place<'tcx>, e: Expr) -> Stmt {
        if self.place_ty(p).is_unit() {
            e.as_stmt(Location::none())
        } else {
            unwrap_or_return_codegen_unimplemented_stmt!(self, self.codegen_place(p))
                .goto_expr
                .assign(e, Location::none())
        }
    }

    pub fn codegen_panic(&self, span: Option<Span>, fargs: Vec<Expr>) -> Stmt {
        // CBMC requires that the argument to the assertion must be a string constant.
        // If there is one in the MIR, use it; otherwise, explain that we can't.
        assert!(!fargs.is_empty(), "Panic requires a string message");
        let msg = utils::extract_const_message(&fargs[0]).unwrap_or(String::from(
            "This is a placeholder message; Kani doesn't support message formatted at runtime",
        ));

        self.codegen_fatal_error(PropertyClass::Assertion, &msg, span)
    }

    // Generate code for fatal error which should trigger an assertion failure and abort the
    // execution.
    pub fn codegen_fatal_error(
        &self,
        property_class: PropertyClass,
        msg: &str,
        span: Option<Span>,
    ) -> Stmt {
        let loc = self.codegen_caller_span(&span);
        Stmt::block(
            vec![
                self.codegen_assert_false(property_class, msg, loc),
                BuiltinFn::Abort.call(vec![], loc).as_stmt(loc),
            ],
            loc,
        )
    }

    /// Generate code to cover the given condition at the current location
    pub fn codegen_cover(&self, cond: Expr, msg: &str, span: Option<Span>) -> Stmt {
        let loc = self.codegen_caller_span(&span);
        // Should use Stmt::cover, but currently this doesn't work with CBMC
        // unless it is run with '--cover cover' (see
        // https://github.com/diffblue/cbmc/issues/6613). So for now use
        // assert(!cond).
        self.codegen_assert(cond.not(), PropertyClass::Cover, msg, loc)
    }

    /// Generate code to cover the current location
    pub fn codegen_cover_loc(&self, msg: &str, span: Option<Span>) -> Stmt {
        self.codegen_cover(Expr::bool_true(), msg, span)
    }

    pub fn codegen_statement(&mut self, stmt: &Statement<'tcx>) -> Stmt {
        let _trace_span = info_span!("CodegenStatement", statement = ?stmt).entered();
        debug!("handling statement {:?}", stmt);
        let location = self.codegen_span(&stmt.source_info.span);
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
                    unwrap_or_return_codegen_unimplemented_stmt!(self, self.codegen_place(l))
                        .goto_expr
                        .assign(self.codegen_rvalue(r, location).address_of(), location)
                } else if rty.is_bool() {
                    unwrap_or_return_codegen_unimplemented_stmt!(self, self.codegen_place(l))
                        .goto_expr
                        .assign(self.codegen_rvalue(r, location).cast_to(Type::c_bool()), location)
                } else {
                    unwrap_or_return_codegen_unimplemented_stmt!(self, self.codegen_place(l))
                        .goto_expr
                        .assign(self.codegen_rvalue(r, location), location)
                }
            }
            StatementKind::Deinit(place) => {
                // From rustc doc: "This writes `uninit` bytes to the entire place."
                // Thus, we assign nondet() value to the entire place.
                let dst_mir_ty = self.place_ty(place);
                let dst_type = self.codegen_ty(dst_mir_ty);
                let layout = self.layout_of(dst_mir_ty);
                if layout.is_zst() || dst_type.sizeof_in_bits(&self.symbol_table) == 0 {
                    // We ignore assignment for all zero size types
                    // Ignore generators too for now:
                    // https://github.com/model-checking/kani/issues/416
                    Stmt::skip(location)
                } else {
                    unwrap_or_return_codegen_unimplemented_stmt!(self, self.codegen_place(place))
                        .goto_expr
                        .assign(dst_type.nondet(), location)
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
                                location,
                                "https://github.com/model-checking/kani/issues/416",
                            )
                            .as_stmt(location);
                    }
                    _ => unreachable!(),
                };
                let layout = self.layout_of(pt);
                match &layout.variants {
                    Variants::Single { .. } => Stmt::skip(location),
                    Variants::Multiple { tag, tag_encoding, .. } => match tag_encoding {
                        TagEncoding::Direct => {
                            let discr = def.discriminant_for_variant(self.tcx, *variant_index);
                            let discr_t = self.codegen_enum_discr_typ(pt);
                            // The constant created below may not fit into the type.
                            // https://github.com/model-checking/kani/issues/996
                            //
                            // It doesn't matter if the type comes from `self.codegen_enum_discr_typ(pt)`
                            // or `discr.ty`. It looks like something is wrong with `discriminat_for_variant`
                            // because when it tries to codegen `std::cmp::Ordering` (which should produce
                            // discriminant values -1, 0 and 1) it produces values 255, 0 and 1 with i8 types:
                            //
                            // debug!("DISCRIMINANT - val:{:?} ty:{:?}", discr.val, discr.ty);
                            // DISCRIMINANT - val:255 ty:i8
                            // DISCRIMINANT - val:0 ty:i8
                            // DISCRIMINANT - val:1 ty:i8
                            let discr = Expr::int_constant(discr.val, self.codegen_ty(discr_t));
                            unwrap_or_return_codegen_unimplemented_stmt!(
                                self,
                                self.codegen_place(place)
                            )
                            .goto_expr
                            .member("case", &self.symbol_table)
                            .assign(discr, location)
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
                                let value =
                                    if niche_value == 0 && tag.primitive() == Primitive::Pointer {
                                        discr_ty.null()
                                    } else {
                                        Expr::int_constant(niche_value, discr_ty.clone())
                                    };
                                let place = unwrap_or_return_codegen_unimplemented_stmt!(
                                    self,
                                    self.codegen_place(place)
                                )
                                .goto_expr;
                                self.codegen_get_niche(place, offset, discr_ty)
                                    .assign(value, location)
                            } else {
                                Stmt::skip(location)
                            }
                        }
                    },
                }
            }
            StatementKind::StorageLive(_) => Stmt::skip(location), // TODO: fix me
            StatementKind::StorageDead(_) => Stmt::skip(location), // TODO: fix me
            StatementKind::CopyNonOverlapping(box mir::CopyNonOverlapping {
                ref src,
                ref dst,
                ref count,
            }) => {
                // Pack the operands and their types, then call `codegen_copy`
                let fargs = vec![
                    self.codegen_operand(src),
                    self.codegen_operand(dst),
                    self.codegen_operand(count),
                ];
                let farg_types =
                    &[self.operand_ty(src), self.operand_ty(dst), self.operand_ty(count)];
                self.codegen_copy("copy_nonoverlapping", true, fargs, farg_types, None, location)
            }
            StatementKind::FakeRead(_)
            | StatementKind::Retag(_, _)
            | StatementKind::AscribeUserType(_, _)
            | StatementKind::Nop
            | StatementKind::Coverage { .. } => Stmt::skip(location),
        }
        .with_location(self.codegen_span(&stmt.source_info.span))
    }
}
