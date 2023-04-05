// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
use super::typ::TypeExt;
use super::typ::FN_RETURN_VOID_VAR_NAME;
use super::PropertyClass;
use crate::codegen_cprover_gotoc::{GotocCtx, VtableCtx};
use crate::unwrap_or_return_codegen_unimplemented_stmt;
use cbmc::goto_program::{Expr, Location, Stmt, Type};
use rustc_hir::def_id::DefId;
use rustc_middle::mir;
use rustc_middle::mir::{
    AssertKind, BasicBlock, NonDivergingIntrinsic, Operand, Place, Statement, StatementKind,
    SwitchTargets, Terminator, TerminatorKind,
};
use rustc_middle::ty;
use rustc_middle::ty::layout::LayoutOf;
use rustc_middle::ty::{Instance, InstanceDef, Ty};
use rustc_span::Span;
use rustc_target::abi::{FieldsShape, Primitive, TagEncoding, Variants};
use tracing::{debug, debug_span, trace};

impl<'tcx> GotocCtx<'tcx> {
    /// Generate Goto-C for MIR [Statement]s.
    /// This does not cover all possible "statements" because MIR distinguishes between ordinary
    /// statements and [Terminator]s, which can exclusively appear at the end of a basic block.
    ///
    /// See [GotocCtx::codegen_terminator] for those.
    pub fn codegen_statement(&mut self, stmt: &Statement<'tcx>) -> Stmt {
        let _trace_span = debug_span!("CodegenStatement", statement = ?stmt).entered();
        debug!(?stmt, kind=?stmt.kind, "handling_statement");
        let location = self.codegen_span(&stmt.source_info.span);
        match &stmt.kind {
            StatementKind::Assign(box (l, r)) => {
                let lty = self.place_ty(l);
                let rty = self.rvalue_ty(r);
                // we ignore assignment for all zero size types
                if self.is_zst(lty) {
                    Stmt::skip(location)
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
            StatementKind::Deinit(place) => self.codegen_deinit(place, location),
            StatementKind::SetDiscriminant { place, variant_index } => {
                // this requires place points to an enum type.
                let pt = self.place_ty(place);
                let layout = self.layout_of(pt);
                match &layout.variants {
                    Variants::Single { .. } => Stmt::skip(location),
                    Variants::Multiple { tag, tag_encoding, .. } => match tag_encoding {
                        TagEncoding::Direct => {
                            let discr =
                                pt.discriminant_for_variant(self.tcx, *variant_index).unwrap();
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
                            let place_goto_expr = unwrap_or_return_codegen_unimplemented_stmt!(
                                self,
                                self.codegen_place(place)
                            )
                            .goto_expr;
                            self.codegen_discriminant_field(place_goto_expr, pt)
                                .assign(discr, location)
                        }
                        TagEncoding::Niche { untagged_variant, niche_variants, niche_start } => {
                            if untagged_variant != variant_index {
                                let offset = match &layout.fields {
                                    FieldsShape::Arbitrary { offsets, .. } => offsets[0],
                                    _ => unreachable!("niche encoding must have arbitrary fields"),
                                };
                                let discr_ty = self.codegen_enum_discr_typ(pt);
                                let discr_ty = self.codegen_ty(discr_ty);
                                let niche_value =
                                    variant_index.as_u32() - niche_variants.start().as_u32();
                                let niche_value = (niche_value as u128).wrapping_add(*niche_start);
                                let value = if niche_value == 0
                                    && matches!(tag.primitive(), Primitive::Pointer(_))
                                {
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
            StatementKind::Intrinsic(box NonDivergingIntrinsic::CopyNonOverlapping(
                mir::CopyNonOverlapping { ref src, ref dst, ref count },
            )) => {
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
            StatementKind::Intrinsic(box NonDivergingIntrinsic::Assume(ref op)) => {
                let cond = self.codegen_operand(op).cast_to(Type::bool());
                self.codegen_assert_assume(
                    cond,
                    PropertyClass::Assume,
                    "assumption failed",
                    location,
                )
            }
            StatementKind::FakeRead(_)
            | StatementKind::Retag(_, _)
            | StatementKind::AscribeUserType(_, _)
            | StatementKind::Nop
            | StatementKind::Coverage { .. }
            | StatementKind::ConstEvalCounter => Stmt::skip(location),
        }
        .with_location(location)
    }

    /// Generate Goto-c for MIR [Terminator] statements.
    /// Many kinds of seemingly ordinary statements in Rust are "terminators" (i.e. the sort of statement that _ends_ a basic block)
    /// because of the need for unwinding/drop. For instance, function calls.
    ///
    /// See also [`GotocCtx::codegen_statement`] for ordinary [Statement]s.
    pub fn codegen_terminator(&mut self, term: &Terminator<'tcx>) -> Stmt {
        let loc = self.codegen_span(&term.source_info.span);
        let _trace_span = debug_span!("CodegenTerminator", statement = ?term.kind).entered();
        debug!("handling terminator {:?}", term);
        //TODO: Instead of doing location::none(), and updating, just putit in when we make the stmt.
        match &term.kind {
            TerminatorKind::Goto { target } => {
                Stmt::goto(self.current_fn().find_label(target), loc)
            }
            TerminatorKind::SwitchInt { discr, targets } => {
                self.codegen_switch_int(discr, targets, loc)
            }
            // The following two use `codegen_mimic_unimplemented`
            // because we don't want to raise the warning during compilation.
            // These operations will normally be codegen'd but normally be unreachable
            // since we make use of `-C unwind=abort`.
            TerminatorKind::Resume => self.codegen_mimic_unimplemented(
                "TerminatorKind::Resume",
                loc,
                "https://github.com/model-checking/kani/issues/692",
            ),
            TerminatorKind::Abort => self.codegen_mimic_unimplemented(
                "TerminatorKind::Abort",
                loc,
                "https://github.com/model-checking/kani/issues/692",
            ),
            TerminatorKind::Return => {
                let rty = self.current_fn().sig().skip_binder().output();
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
            TerminatorKind::Unreachable => self.codegen_assert_assume_false(
                PropertyClass::Unreachable,
                "unreachable code",
                loc,
            ),
            TerminatorKind::Drop { place, target, unwind: _ } => {
                self.codegen_drop(place, target, loc)
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

                let (msg_str, reach_stmt) =
                    self.codegen_reachability_check(msg.to_owned(), Some(term.source_info.span));

                Stmt::block(
                    vec![
                        reach_stmt,
                        self.codegen_assert_assume(
                            cond.cast_to(Type::bool()),
                            PropertyClass::Assertion,
                            &msg_str,
                            loc,
                        ),
                        Stmt::goto(self.current_fn().find_label(target), loc),
                    ],
                    loc,
                )
            }
            TerminatorKind::DropAndReplace { .. }
            | TerminatorKind::FalseEdge { .. }
            | TerminatorKind::FalseUnwind { .. } => {
                unreachable!("drop elaboration removes these TerminatorKind")
            }
            TerminatorKind::Yield { .. } | TerminatorKind::GeneratorDrop => {
                unreachable!("we should not hit these cases") // why?
            }
            TerminatorKind::InlineAsm { .. } => self.codegen_unimplemented_stmt(
                "TerminatorKind::InlineAsm",
                loc,
                "https://github.com/model-checking/kani/issues/2",
            ),
        }
    }

    /// From rustc doc: "This writes `uninit` bytes to the entire place."
    /// Our model of GotoC has a similar statement, which is later lowered
    /// to assigning a Nondet in CBMC, with a comment specifying that it
    /// corresponds to a Deinit.
    #[cfg(not(feature = "unsound_experiments"))]
    fn codegen_deinit(&mut self, place: &Place<'tcx>, loc: Location) -> Stmt {
        let dst_mir_ty = self.place_ty(place);
        let dst_type = self.codegen_ty(dst_mir_ty);
        let layout = self.layout_of(dst_mir_ty);
        if layout.is_zst() || dst_type.sizeof_in_bits(&self.symbol_table) == 0 {
            // We ignore assignment for all zero size types
            Stmt::skip(loc)
        } else {
            unwrap_or_return_codegen_unimplemented_stmt!(self, self.codegen_place(place))
                .goto_expr
                .deinit(loc)
        }
    }

    /// A special case handler to codegen `return ();`
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

    /// Generates Goto-C for MIR [TerminatorKind::Drop] calls. We only handle code _after_ Rust's "drop elaboration"
    /// transformation, so these have a simpler semantics.
    ///
    /// The generated code should invoke the appropriate `drop` function on `place`, then goto `target`.
    ///
    /// TODO: this function doesn't handle unwinding which begins if the destructor panics
    /// <https://github.com/model-checking/kani/issues/221>
    fn codegen_drop(&mut self, place: &Place<'tcx>, target: &BasicBlock, loc: Location) -> Stmt {
        let place_ty = self.place_ty(place);
        let drop_instance = Instance::resolve_drop_in_place(self.tcx, place_ty);
        debug!(?place_ty, ?drop_instance, "codegen_drop");
        // Once upon a time we did a `hook_applies` check here, but we no longer seem to hook drops
        let drop_implementation = match drop_instance.def {
            InstanceDef::DropGlue(_, None) => {
                // We can skip empty DropGlue functions
                Stmt::skip(loc)
            }
            InstanceDef::DropGlue(_def_id, Some(_)) => {
                let place_ref = self.codegen_place_ref(place);
                match place_ty.kind() {
                    ty::Dynamic(..) => {
                        // Virtual drop via a vtable lookup.
                        // Pull the drop function off of the fat pointer's vtable pointer
                        let vtable_ref = place_ref.to_owned().member("vtable", &self.symbol_table);
                        let data_ref = place_ref.to_owned().member("data", &self.symbol_table);
                        let vtable = vtable_ref.dereference();
                        let fn_ptr = vtable.member("drop", &self.symbol_table);
                        trace!(?fn_ptr, ?data_ref, "codegen_drop");

                        let call = fn_ptr.dereference().call(vec![data_ref]).as_stmt(loc);
                        if self.vtable_ctx.emit_vtable_restrictions {
                            self.virtual_call_with_restricted_fn_ptr(
                                place_ref.typ().clone(),
                                VtableCtx::drop_index(),
                                call,
                            )
                        } else {
                            call
                        }
                    }
                    _ => {
                        // Non-virtual, direct drop_in_place call
                        assert!(!matches!(drop_instance.def, InstanceDef::Virtual(_, _)));

                        let func = self.codegen_func_expr(drop_instance, None);
                        // The only argument should be a self reference
                        let args = vec![place_ref];

                        // We have a known issue where nested Arc and Mutex objects result in
                        // drop_in_place call implementations that fail to typecheck. Skipping
                        // drop entirely causes unsound verification results in common cases
                        // like vector extend, so for now, add a sound special case workaround
                        // for calls that fail the typecheck.
                        // https://github.com/model-checking/kani/issues/426
                        // Unblocks: https://github.com/model-checking/kani/issues/435
                        func.call(args).as_stmt(loc)
                    }
                }
            }
            _ => unreachable!(
                "TerminatorKind::Drop but not InstanceDef::DropGlue should be impossible"
            ),
        };
        let goto_target = Stmt::goto(self.current_fn().find_label(target), loc);
        let block = vec![drop_implementation, goto_target];
        Stmt::block(block, loc)
    }

    /// Generates Goto-C for MIR [TerminatorKind::SwitchInt].
    /// Operand evaluates to an integer;
    /// jump depending on its value to one of the targets, and otherwise fallback to `targets.otherwise()`.
    /// The otherwise value is stores as the last value of targets.
    fn codegen_switch_int(
        &mut self,
        discr: &Operand<'tcx>,
        targets: &SwitchTargets,
        loc: Location,
    ) -> Stmt {
        let v = self.codegen_operand(discr);
        let switch_ty = v.typ().clone();
        if targets.all_targets().len() == 1 {
            // Translate to a guarded goto
            let first_target = targets.iter().next().unwrap();
            Stmt::block(
                vec![
                    v.eq(Expr::int_constant(first_target.0, switch_ty)).if_then_else(
                        Stmt::goto(self.current_fn().find_label(&first_target.1), loc),
                        None,
                        loc,
                    ),
                    Stmt::goto(self.current_fn().find_label(&targets.otherwise()), loc),
                ],
                loc,
            )
        } else {
            // Switches with empty targets should've been eliminated already.
            assert!(targets.all_targets().len() > 1);
            let cases = targets
                .iter()
                .map(|(c, bb)| {
                    Expr::int_constant(c, switch_ty.clone())
                        .switch_case(Stmt::goto(self.current_fn().find_label(&bb), loc))
                })
                .collect();
            let default = Stmt::goto(self.current_fn().find_label(&targets.otherwise()), loc);
            v.switch(cases, Some(default), loc)
        }
    }

    /// As part of **calling** a function (or closure), we may need to un-tuple arguments.
    ///
    /// This function will replace the last `fargs` argument by its un-tupled version.
    ///
    /// Some context: A closure / shim takes two arguments:
    ///     0. a struct (or a pointer to) representing the environment
    ///     1. a tuple containing the parameters (if not empty)
    ///
    /// However, Rust generates a function where the tuple of parameters are flattened
    /// as subsequent parameters.
    ///
    /// See [GotocCtx::ty_needs_untupled_args] for more details.
    fn codegen_untupled_args(
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
        if !fargs.is_empty() {
            let tuple_ty = self.operand_ty(last_mir_arg.unwrap());
            if self.is_zst(tuple_ty) {
                // Don't pass anything if all tuple elements are ZST.
                // ZST arguments are ignored.
                return;
            }
            let tupe = fargs.remove(fargs.len() - 1);
            if let ty::Tuple(tupled_args) = tuple_ty.kind() {
                for (idx, arg_ty) in tupled_args.iter().enumerate() {
                    if !self.is_zst(arg_ty) {
                        // Access the tupled parameters through the `member` operation
                        let idx_expr = tupe.clone().member(&idx.to_string(), &self.symbol_table);
                        fargs.push(idx_expr);
                    }
                }
            }
        }
    }

    /// Because function calls terminate basic blocks, to "end" a function call, we
    /// must jump to the next basic block.
    fn codegen_end_call(&self, target: Option<&BasicBlock>, loc: Location) -> Stmt {
        if let Some(next_bb) = target {
            Stmt::goto(self.current_fn().find_label(next_bb), loc)
        } else {
            self.codegen_sanity(Expr::bool_false(), "Unexpected return from Never function", loc)
        }
    }

    /// Generate Goto-C for each argument to a function call.
    ///
    /// N.B. public only because instrinsics use this directly, too.
    /// When `skip_zst` is set to `true`, the return value will not include any argument that is ZST.
    /// This is used because we ignore ZST arguments, except for intrinsics.
    pub(crate) fn codegen_funcall_args(
        &mut self,
        args: &[Operand<'tcx>],
        skip_zst: bool,
    ) -> Vec<Expr> {
        let fargs = args
            .iter()
            .filter_map(|o| {
                let op_ty = self.operand_ty(o);
                if op_ty.is_bool() {
                    Some(self.codegen_operand(o).cast_to(Type::c_bool()))
                } else if !self.is_zst(op_ty) || !skip_zst {
                    Some(self.codegen_operand(o))
                } else {
                    // We ignore ZST types.
                    debug!(arg=?o, "codegen_funcall_args ignore");
                    None
                }
            })
            .collect();
        debug!(?fargs, "codegen_funcall_args");
        fargs
    }

    /// Generates Goto-C for a MIR [TerminatorKind::Call] statement.
    ///
    /// This calls either:
    ///
    /// 1. A statically-known function definition.
    /// 2. A statically-known trait function, which gets a pointer out of a vtable.
    /// 2. A direct function pointer.
    ///
    /// Kani also performs a few alterations:
    ///
    /// 1. Do nothing for "empty drop glue"
    /// 2. If a Kani hook applies, do that instead.
    fn codegen_funcall(
        &mut self,
        func: &Operand<'tcx>,
        args: &[Operand<'tcx>],
        destination: &Place<'tcx>,
        target: &Option<BasicBlock>,
        span: Span,
    ) -> Stmt {
        debug!(?func, ?args, ?destination, ?span, "codegen_funcall");
        if self.is_intrinsic(func) {
            return self.codegen_funcall_of_intrinsic(func, args, destination, target, span);
        }

        let loc = self.codegen_span(&span);
        let funct = self.operand_ty(func);
        let mut fargs = self.codegen_funcall_args(args, true);
        match &funct.kind() {
            ty::FnDef(defid, subst) => {
                let instance =
                    Instance::resolve(self.tcx, ty::ParamEnv::reveal_all(), *defid, subst)
                        .unwrap()
                        .unwrap();

                // TODO(celina): Move this check to be inside codegen_funcall_args.
                if self.ty_needs_untupled_args(funct) {
                    self.codegen_untupled_args(instance, &mut fargs, args.last());
                }

                if let Some(hk) = self.hooks.hook_applies(self.tcx, instance) {
                    return hk.handle(self, instance, fargs, *destination, *target, Some(span));
                }

                let mut stmts: Vec<Stmt> = match instance.def {
                    // Here an empty drop glue is invoked; we just ignore it.
                    InstanceDef::DropGlue(_, None) => {
                        return Stmt::goto(self.current_fn().find_label(&target.unwrap()), loc);
                    }
                    // Handle a virtual function call via a vtable lookup
                    InstanceDef::Virtual(def_id, idx) => {
                        let self_ty = self.operand_ty(&args[0]);
                        self.codegen_virtual_funcall(
                            self_ty,
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
                    | InstanceDef::VTableShim(..)
                    | InstanceDef::ReifyShim(..)
                    | InstanceDef::ClosureOnceShim { .. }
                    | InstanceDef::CloneShim(..) => {
                        // We need to handle FnDef items in a special way because `codegen_operand` compiles them to dummy structs.
                        // (cf. the function documentation)
                        let func_exp = self.codegen_func_expr(instance, None);
                        vec![
                            self.codegen_expr_to_place(destination, func_exp.call(fargs))
                                .with_location(loc),
                        ]
                    }
                };
                stmts.push(self.codegen_end_call(target.as_ref(), loc));
                Stmt::block(stmts, loc)
            }
            // Function call through a pointer
            ty::FnPtr(_) => {
                let func_expr = self.codegen_operand(func).dereference();
                // Actually generate the function call and return.
                Stmt::block(
                    vec![
                        self.codegen_expr_to_place(destination, func_expr.call(fargs))
                            .with_location(loc),
                        Stmt::goto(self.current_fn().find_label(&target.unwrap()), loc),
                    ],
                    loc,
                )
            }
            x => unreachable!("Function call where the function was of unexpected type: {:?}", x),
        }
    }

    /// Extract a reference to self for virtual method calls.
    ///
    /// See [GotocCtx::codegen_dynamic_function_sig] for more details.
    fn extract_ptr(&self, arg_expr: Expr, arg_ty: Ty<'tcx>) -> Expr {
        // Generate an expression that indexes the pointer.
        let expr = self
            .receiver_data_path(arg_ty)
            .fold(arg_expr, |curr_expr, (name, _)| curr_expr.member(name, &self.symbol_table));

        trace!(?arg_ty, gotoc_ty=?expr.typ(), gotoc_expr=?expr.value(), "extract_ptr");
        expr
    }

    /// Codegen the dynamic call to a trait method via the fat pointer vtable.
    ///
    /// If the original call was of the form
    /// f(arg0, arg1);
    ///
    /// The new call should be of the form
    /// arg0.vtable->f(arg0.data,arg1);
    ///
    /// For that, we do the following:
    /// 1. Extract the fat pointer out of the first argument.
    /// 2. Obtain the function pointer out of the fat pointer vtable.
    /// 3. Change the first argument to only reference the data pointer (instead of the fat one).
    ///     - When the receiver type is a `struct` we need to build a structure that mirrors
    ///       the original one but uses a thin pointer instead.
    /// 4. Generate the function call.
    fn codegen_virtual_funcall(
        &mut self,
        self_ty: Ty<'tcx>,
        def_id: DefId,
        idx: usize,
        place: &Place<'tcx>,
        fargs: &mut [Expr],
        loc: Location,
    ) -> Vec<Stmt> {
        let vtable_field_name = self.vtable_field_name(def_id, idx);
        trace!(?self_ty, ?place, ?vtable_field_name, "codegen_virtual_funcall");
        debug!(?fargs, "codegen_virtual_funcall");

        let trait_fat_ptr = self.extract_ptr(fargs[0].clone(), self_ty);
        assert!(
            trait_fat_ptr.typ().is_rust_trait_fat_ptr(&self.symbol_table),
            "Expected fat pointer, but got {:?}",
            trait_fat_ptr.typ()
        );

        let vtable_ref = trait_fat_ptr.to_owned().member("vtable", &self.symbol_table);
        let vtable = vtable_ref.dereference();
        let fn_ptr = vtable.member(vtable_field_name, &self.symbol_table);
        trace!(fn_typ=?fn_ptr.typ(), "codegen_virtual_funcall");

        let data_ptr = trait_fat_ptr.to_owned().member("data", &self.symbol_table);
        let mut ret_stmts = vec![];
        fargs[0] = if self_ty.is_adt() {
            // Generate a temp variable and assign its inner pointer to the fat_ptr.data.
            match fn_ptr.typ() {
                Type::Pointer { typ: box Type::Code { parameters, .. } } => {
                    let param_typ = parameters.first().unwrap().typ();
                    let (tmp, decl) = self.decl_temp_variable(param_typ.clone(), None, loc);
                    debug!(?tmp,
                        orig=?data_ptr.typ(),
                        "codegen_virtual_funcall");
                    ret_stmts.push(decl);
                    ret_stmts.push(Stmt::assign(
                        self.extract_ptr(tmp.clone(), self_ty),
                        data_ptr,
                        loc,
                    ));
                    tmp
                }
                _ => unreachable!("Unexpected virtual function type: {:?}", fn_ptr.typ()),
            }
        } else {
            // Update the argument from arg0 to arg0.data if arg0 is a fat pointer.
            data_ptr
        };

        // For soundness, add an assertion that the vtable function call is not null.
        // Otherwise, CBMC might treat this as an assume(0) and later user-added assertions
        // could become unreachable.
        let call_is_nonnull = fn_ptr.clone().is_nonnull();
        let assert_msg = format!("Non-null virtual function call for {vtable_field_name:?}");
        let assert_nonnull = self.codegen_sanity(call_is_nonnull, &assert_msg, loc);

        // Virtual function call and corresponding nonnull assertion.
        let call = fn_ptr.dereference().call(fargs.to_vec());
        let call_stmt = self.codegen_expr_to_place(place, call).with_location(loc);
        let call_stmt = if self.vtable_ctx.emit_vtable_restrictions {
            self.virtual_call_with_restricted_fn_ptr(trait_fat_ptr.typ().clone(), idx, call_stmt)
        } else {
            call_stmt
        };
        ret_stmts.push(assert_nonnull);
        ret_stmts.push(call_stmt);
        ret_stmts
    }

    /// Generates Goto-C to assign a value to a [Place].
    /// A MIR [Place] is an L-value (i.e. the LHS of an assignment).
    ///
    /// In Kani, we slightly optimize the special case for Unit and don't assign anything.
    pub(crate) fn codegen_expr_to_place(&mut self, p: &Place<'tcx>, e: Expr) -> Stmt {
        if self.place_ty(p).is_unit() {
            e.as_stmt(Location::none())
        } else {
            unwrap_or_return_codegen_unimplemented_stmt!(self, self.codegen_place(p))
                .goto_expr
                .assign(e, Location::none())
        }
    }
}
