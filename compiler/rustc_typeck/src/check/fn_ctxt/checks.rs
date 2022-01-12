use crate::astconv::AstConv;
use crate::check::coercion::CoerceMany;
use crate::check::gather_locals::Declaration;
use crate::check::method::MethodCallee;
use crate::check::Expectation::*;
use crate::check::TupleArgumentsFlag::*;
use crate::check::{
    potentially_plural_count, struct_span_err, BreakableCtxt, Diverges, Expectation, FnCtxt,
    LocalTy, Needs, TupleArgumentsFlag,
};

use rustc_ast as ast;
use rustc_data_structures::sync::Lrc;
use rustc_errors::{Applicability, DiagnosticBuilder, DiagnosticId};
use rustc_hir as hir;
use rustc_hir::def::{CtorOf, DefKind, Res};
use rustc_hir::def_id::DefId;
use rustc_hir::{ExprKind, Node, QPath};
use rustc_middle::ty::adjustment::AllowTwoPhase;
use rustc_middle::ty::fold::TypeFoldable;
use rustc_middle::ty::{self, Ty};
use rustc_session::Session;
use rustc_span::symbol::Ident;
use rustc_span::{self, MultiSpan, Span};
use rustc_trait_selection::traits::{self, ObligationCauseCode, StatementAsExpression};

use crate::structured_errors::StructuredDiagnostic;
use std::iter;
use std::slice;

impl<'a, 'tcx> FnCtxt<'a, 'tcx> {
    pub(in super::super) fn check_casts(&self) {
        let mut deferred_cast_checks = self.deferred_cast_checks.borrow_mut();
        debug!("FnCtxt::check_casts: {} deferred checks", deferred_cast_checks.len());
        for cast in deferred_cast_checks.drain(..) {
            cast.check(self);
        }
    }

    pub(in super::super) fn check_method_argument_types(
        &self,
        sp: Span,
        expr: &'tcx hir::Expr<'tcx>,
        method: Result<MethodCallee<'tcx>, ()>,
        args_no_rcvr: &'tcx [hir::Expr<'tcx>],
        tuple_arguments: TupleArgumentsFlag,
        expected: Expectation<'tcx>,
    ) -> Ty<'tcx> {
        let has_error = match method {
            Ok(method) => method.substs.references_error() || method.sig.references_error(),
            Err(_) => true,
        };
        if has_error {
            let err_inputs = self.err_args(args_no_rcvr.len());

            let err_inputs = match tuple_arguments {
                DontTupleArguments => err_inputs,
                TupleArguments => vec![self.tcx.intern_tup(&err_inputs)],
            };

            self.check_argument_types(
                sp,
                expr,
                &err_inputs,
                vec![],
                args_no_rcvr,
                false,
                tuple_arguments,
                None,
            );
            return self.tcx.ty_error();
        }

        let method = method.unwrap();
        // HACK(eddyb) ignore self in the definition (see above).
        let expected_input_tys = self.expected_inputs_for_expected_output(
            sp,
            expected,
            method.sig.output(),
            &method.sig.inputs()[1..],
        );
        self.check_argument_types(
            sp,
            expr,
            &method.sig.inputs()[1..],
            expected_input_tys,
            args_no_rcvr,
            method.sig.c_variadic,
            tuple_arguments,
            Some(method.def_id),
        );
        method.sig.output()
    }

    /// Generic function that factors out common logic from function calls,
    /// method calls and overloaded operators.
    pub(in super::super) fn check_argument_types(
        &self,
        // Span enclosing the call site
        call_span: Span,
        // Expression of the call site
        call_expr: &'tcx hir::Expr<'tcx>,
        // Types (as defined in the *signature* of the target function)
        formal_input_tys: &[Ty<'tcx>],
        // More specific expected types, after unifying with caller output types
        expected_input_tys: Vec<Ty<'tcx>>,
        // The expressions for each provided argument
        provided_args: &'tcx [hir::Expr<'tcx>],
        // Whether the function is variadic, for example when imported from C
        c_variadic: bool,
        // Whether the arguments have been bundled in a tuple (ex: closures)
        tuple_arguments: TupleArgumentsFlag,
        // The DefId for the function being called, for better error messages
        fn_def_id: Option<DefId>,
    ) {
        let tcx = self.tcx;
        // Grab the argument types, supplying fresh type variables
        // if the wrong number of arguments were supplied
        let supplied_arg_count =
            if tuple_arguments == DontTupleArguments { provided_args.len() } else { 1 };

        // All the input types from the fn signature must outlive the call
        // so as to validate implied bounds.
        for (&fn_input_ty, arg_expr) in iter::zip(formal_input_tys, provided_args) {
            self.register_wf_obligation(fn_input_ty.into(), arg_expr.span, traits::MiscObligation);
        }

        let expected_arg_count = formal_input_tys.len();

        let param_count_error = |expected_count: usize,
                                 arg_count: usize,
                                 error_code: &str,
                                 c_variadic: bool,
                                 sugg_unit: bool| {
            let (span, start_span, args, ctor_of) = match &call_expr.kind {
                hir::ExprKind::Call(
                    hir::Expr {
                        span,
                        kind:
                            hir::ExprKind::Path(hir::QPath::Resolved(
                                _,
                                hir::Path { res: Res::Def(DefKind::Ctor(of, _), _), .. },
                            )),
                        ..
                    },
                    args,
                ) => (*span, *span, &args[..], Some(of)),
                hir::ExprKind::Call(hir::Expr { span, .. }, args) => {
                    (*span, *span, &args[..], None)
                }
                hir::ExprKind::MethodCall(path_segment, span, args, _) => (
                    *span,
                    // `sp` doesn't point at the whole `foo.bar()`, only at `bar`.
                    path_segment
                        .args
                        .and_then(|args| args.args.iter().last())
                        // Account for `foo.bar::<T>()`.
                        .map(|arg| {
                            // Skip the closing `>`.
                            tcx.sess
                                .source_map()
                                .next_point(tcx.sess.source_map().next_point(arg.span()))
                        })
                        .unwrap_or(*span),
                    &args[1..], // Skip the receiver.
                    None,       // methods are never ctors
                ),
                k => span_bug!(call_span, "checking argument types on a non-call: `{:?}`", k),
            };
            let arg_spans = if provided_args.is_empty() {
                // foo()
                // ^^^-- supplied 0 arguments
                // |
                // expected 2 arguments
                vec![tcx.sess.source_map().next_point(start_span).with_hi(call_span.hi())]
            } else {
                // foo(1, 2, 3)
                // ^^^ -  -  - supplied 3 arguments
                // |
                // expected 2 arguments
                args.iter().map(|arg| arg.span).collect::<Vec<Span>>()
            };

            let mut err = tcx.sess.struct_span_err_with_code(
                span,
                &format!(
                    "this {} takes {}{} but {} {} supplied",
                    match ctor_of {
                        Some(CtorOf::Struct) => "struct",
                        Some(CtorOf::Variant) => "enum variant",
                        None => "function",
                    },
                    if c_variadic { "at least " } else { "" },
                    potentially_plural_count(expected_count, "argument"),
                    potentially_plural_count(arg_count, "argument"),
                    if arg_count == 1 { "was" } else { "were" }
                ),
                DiagnosticId::Error(error_code.to_owned()),
            );
            let label = format!("supplied {}", potentially_plural_count(arg_count, "argument"));
            for (i, span) in arg_spans.into_iter().enumerate() {
                err.span_label(
                    span,
                    if arg_count == 0 || i + 1 == arg_count { &label } else { "" },
                );
            }

            if let Some(def_id) = fn_def_id {
                if let Some(def_span) = tcx.def_ident_span(def_id) {
                    let mut spans: MultiSpan = def_span.into();

                    let params = tcx
                        .hir()
                        .get_if_local(def_id)
                        .and_then(|node| node.body_id())
                        .into_iter()
                        .map(|id| tcx.hir().body(id).params)
                        .flatten();

                    for param in params {
                        spans.push_span_label(param.span, String::new());
                    }

                    let def_kind = tcx.def_kind(def_id);
                    err.span_note(spans, &format!("{} defined here", def_kind.descr(def_id)));
                }
            }

            if sugg_unit {
                let sugg_span = tcx.sess.source_map().end_point(call_expr.span);
                // remove closing `)` from the span
                let sugg_span = sugg_span.shrink_to_lo();
                err.span_suggestion(
                    sugg_span,
                    "expected the unit value `()`; create it with empty parentheses",
                    String::from("()"),
                    Applicability::MachineApplicable,
                );
            } else {
                err.span_label(
                    span,
                    format!(
                        "expected {}{}",
                        if c_variadic { "at least " } else { "" },
                        potentially_plural_count(expected_count, "argument")
                    ),
                );
            }
            err.emit();
        };

        let (formal_input_tys, expected_input_tys) = if tuple_arguments == TupleArguments {
            let tuple_type = self.structurally_resolved_type(call_span, formal_input_tys[0]);
            match tuple_type.kind() {
                ty::Tuple(arg_types) if arg_types.len() != provided_args.len() => {
                    param_count_error(arg_types.len(), provided_args.len(), "E0057", false, false);
                    (self.err_args(provided_args.len()), vec![])
                }
                ty::Tuple(arg_types) => {
                    let expected_input_tys = match expected_input_tys.get(0) {
                        Some(&ty) => match ty.kind() {
                            ty::Tuple(ref tys) => tys.iter().map(|k| k.expect_ty()).collect(),
                            _ => vec![],
                        },
                        None => vec![],
                    };
                    (arg_types.iter().map(|k| k.expect_ty()).collect(), expected_input_tys)
                }
                _ => {
                    struct_span_err!(
                        tcx.sess,
                        call_span,
                        E0059,
                        "cannot use call notation; the first type parameter \
                         for the function trait is neither a tuple nor unit"
                    )
                    .emit();
                    (self.err_args(provided_args.len()), vec![])
                }
            }
        } else if expected_arg_count == supplied_arg_count {
            (formal_input_tys.to_vec(), expected_input_tys)
        } else if c_variadic {
            if supplied_arg_count >= expected_arg_count {
                (formal_input_tys.to_vec(), expected_input_tys)
            } else {
                param_count_error(expected_arg_count, supplied_arg_count, "E0060", true, false);
                (self.err_args(supplied_arg_count), vec![])
            }
        } else {
            // is the missing argument of type `()`?
            let sugg_unit = if expected_input_tys.len() == 1 && supplied_arg_count == 0 {
                self.resolve_vars_if_possible(expected_input_tys[0]).is_unit()
            } else if formal_input_tys.len() == 1 && supplied_arg_count == 0 {
                self.resolve_vars_if_possible(formal_input_tys[0]).is_unit()
            } else {
                false
            };
            param_count_error(expected_arg_count, supplied_arg_count, "E0061", false, sugg_unit);

            (self.err_args(supplied_arg_count), vec![])
        };

        debug!(
            "check_argument_types: formal_input_tys={:?}",
            formal_input_tys.iter().map(|t| self.ty_to_string(*t)).collect::<Vec<String>>()
        );

        // If there is no expectation, expect formal_input_tys.
        let expected_input_tys = if !expected_input_tys.is_empty() {
            expected_input_tys
        } else {
            formal_input_tys.clone()
        };

        assert_eq!(expected_input_tys.len(), formal_input_tys.len());

        // Keep track of the fully coerced argument types
        let mut final_arg_types: Vec<(usize, Ty<'_>, Ty<'_>)> = vec![];

        // We introduce a helper function to demand that a given argument satisfy a given input
        // This is more complicated than just checking type equality, as arguments could be coerced
        // This version writes those types back so further type checking uses the narrowed types
        let demand_compatible = |idx, final_arg_types: &mut Vec<(usize, Ty<'tcx>, Ty<'tcx>)>| {
            let formal_input_ty: Ty<'tcx> = formal_input_tys[idx];
            let expected_input_ty: Ty<'tcx> = expected_input_tys[idx];
            let provided_arg = &provided_args[idx];

            debug!("checking argument {}: {:?} = {:?}", idx, provided_arg, formal_input_ty);

            // The special-cased logic below has three functions:
            // 1. Provide as good of an expected type as possible.
            let expectation = Expectation::rvalue_hint(self, expected_input_ty);

            let checked_ty = self.check_expr_with_expectation(provided_arg, expectation);

            // 2. Coerce to the most detailed type that could be coerced
            //    to, which is `expected_ty` if `rvalue_hint` returns an
            //    `ExpectHasType(expected_ty)`, or the `formal_ty` otherwise.
            let coerced_ty = expectation.only_has_type(self).unwrap_or(formal_input_ty);

            // Keep track of these for below
            final_arg_types.push((idx, checked_ty, coerced_ty));

            // Cause selection errors caused by resolving a single argument to point at the
            // argument and not the call. This is otherwise redundant with the `demand_coerce`
            // call immediately after, but it lets us customize the span pointed to in the
            // fulfillment error to be more accurate.
            let _ =
                self.resolve_vars_with_obligations_and_mutate_fulfillment(coerced_ty, |errors| {
                    self.point_at_type_arg_instead_of_call_if_possible(errors, call_expr);
                    self.point_at_arg_instead_of_call_if_possible(
                        errors,
                        &final_arg_types,
                        call_expr,
                        call_span,
                        provided_args,
                    );
                });

            // We're processing function arguments so we definitely want to use
            // two-phase borrows.
            self.demand_coerce(&provided_arg, checked_ty, coerced_ty, None, AllowTwoPhase::Yes);

            // 3. Relate the expected type and the formal one,
            //    if the expected type was used for the coercion.
            self.demand_suptype(provided_arg.span, formal_input_ty, coerced_ty);
        };

        // Check the arguments.
        // We do this in a pretty awful way: first we type-check any arguments
        // that are not closures, then we type-check the closures. This is so
        // that we have more information about the types of arguments when we
        // type-check the functions. This isn't really the right way to do this.
        for check_closures in [false, true] {
            // More awful hacks: before we check argument types, try to do
            // an "opportunistic" trait resolution of any trait bounds on
            // the call. This helps coercions.
            if check_closures {
                self.select_obligations_where_possible(false, |errors| {
                    self.point_at_type_arg_instead_of_call_if_possible(errors, call_expr);
                    self.point_at_arg_instead_of_call_if_possible(
                        errors,
                        &final_arg_types,
                        call_expr,
                        call_span,
                        &provided_args,
                    );
                })
            }

            let minimum_input_count = formal_input_tys.len();
            for (idx, arg) in provided_args.iter().enumerate() {
                // Warn only for the first loop (the "no closures" one).
                // Closure arguments themselves can't be diverging, but
                // a previous argument can, e.g., `foo(panic!(), || {})`.
                if !check_closures {
                    self.warn_if_unreachable(arg.hir_id, arg.span, "expression");
                }

                // For C-variadic functions, we don't have a declared type for all of
                // the arguments hence we only do our usual type checking with
                // the arguments who's types we do know. However, we *can* check
                // for unreachable expressions (see above).
                // FIXME: unreachable warning current isn't emitted
                if idx >= minimum_input_count {
                    continue;
                }

                let is_closure = matches!(arg.kind, ExprKind::Closure(..));
                if is_closure != check_closures {
                    continue;
                }

                demand_compatible(idx, &mut final_arg_types);
            }
        }

        // We also need to make sure we at least write the ty of the other
        // arguments which we skipped above.
        if c_variadic {
            fn variadic_error<'tcx>(sess: &Session, span: Span, ty: Ty<'tcx>, cast_ty: &str) {
                use crate::structured_errors::MissingCastForVariadicArg;

                MissingCastForVariadicArg { sess, span, ty, cast_ty }.diagnostic().emit()
            }

            for arg in provided_args.iter().skip(expected_arg_count) {
                let arg_ty = self.check_expr(&arg);

                // There are a few types which get autopromoted when passed via varargs
                // in C but we just error out instead and require explicit casts.
                let arg_ty = self.structurally_resolved_type(arg.span, arg_ty);
                match arg_ty.kind() {
                    ty::Float(ty::FloatTy::F32) => {
                        variadic_error(tcx.sess, arg.span, arg_ty, "c_double");
                    }
                    ty::Int(ty::IntTy::I8 | ty::IntTy::I16) | ty::Bool => {
                        variadic_error(tcx.sess, arg.span, arg_ty, "c_int");
                    }
                    ty::Uint(ty::UintTy::U8 | ty::UintTy::U16) => {
                        variadic_error(tcx.sess, arg.span, arg_ty, "c_uint");
                    }
                    ty::FnDef(..) => {
                        let ptr_ty = self.tcx.mk_fn_ptr(arg_ty.fn_sig(self.tcx));
                        let ptr_ty = self.resolve_vars_if_possible(ptr_ty);
                        variadic_error(tcx.sess, arg.span, arg_ty, &ptr_ty.to_string());
                    }
                    _ => {}
                }
            }
        }
    }

    // AST fragment checking
    pub(in super::super) fn check_lit(
        &self,
        lit: &hir::Lit,
        expected: Expectation<'tcx>,
    ) -> Ty<'tcx> {
        let tcx = self.tcx;

        match lit.node {
            ast::LitKind::Str(..) => tcx.mk_static_str(),
            ast::LitKind::ByteStr(ref v) => {
                tcx.mk_imm_ref(tcx.lifetimes.re_static, tcx.mk_array(tcx.types.u8, v.len() as u64))
            }
            ast::LitKind::Byte(_) => tcx.types.u8,
            ast::LitKind::Char(_) => tcx.types.char,
            ast::LitKind::Int(_, ast::LitIntType::Signed(t)) => tcx.mk_mach_int(ty::int_ty(t)),
            ast::LitKind::Int(_, ast::LitIntType::Unsigned(t)) => tcx.mk_mach_uint(ty::uint_ty(t)),
            ast::LitKind::Int(_, ast::LitIntType::Unsuffixed) => {
                let opt_ty = expected.to_option(self).and_then(|ty| match ty.kind() {
                    ty::Int(_) | ty::Uint(_) => Some(ty),
                    ty::Char => Some(tcx.types.u8),
                    ty::RawPtr(..) => Some(tcx.types.usize),
                    ty::FnDef(..) | ty::FnPtr(_) => Some(tcx.types.usize),
                    _ => None,
                });
                opt_ty.unwrap_or_else(|| self.next_int_var())
            }
            ast::LitKind::Float(_, ast::LitFloatType::Suffixed(t)) => {
                tcx.mk_mach_float(ty::float_ty(t))
            }
            ast::LitKind::Float(_, ast::LitFloatType::Unsuffixed) => {
                let opt_ty = expected.to_option(self).and_then(|ty| match ty.kind() {
                    ty::Float(_) => Some(ty),
                    _ => None,
                });
                opt_ty.unwrap_or_else(|| self.next_float_var())
            }
            ast::LitKind::Bool(_) => tcx.types.bool,
            ast::LitKind::Err(_) => tcx.ty_error(),
        }
    }

    pub fn check_struct_path(
        &self,
        qpath: &QPath<'_>,
        hir_id: hir::HirId,
    ) -> Option<(&'tcx ty::VariantDef, Ty<'tcx>)> {
        let path_span = qpath.span();
        let (def, ty) = self.finish_resolving_struct_path(qpath, path_span, hir_id);
        let variant = match def {
            Res::Err => {
                self.set_tainted_by_errors();
                return None;
            }
            Res::Def(DefKind::Variant, _) => match ty.kind() {
                ty::Adt(adt, substs) => Some((adt.variant_of_res(def), adt.did, substs)),
                _ => bug!("unexpected type: {:?}", ty),
            },
            Res::Def(DefKind::Struct | DefKind::Union | DefKind::TyAlias | DefKind::AssocTy, _)
            | Res::SelfTy(..) => match ty.kind() {
                ty::Adt(adt, substs) if !adt.is_enum() => {
                    Some((adt.non_enum_variant(), adt.did, substs))
                }
                _ => None,
            },
            _ => bug!("unexpected definition: {:?}", def),
        };

        if let Some((variant, did, substs)) = variant {
            debug!("check_struct_path: did={:?} substs={:?}", did, substs);
            self.write_user_type_annotation_from_substs(hir_id, did, substs, None);

            // Check bounds on type arguments used in the path.
            self.add_required_obligations(path_span, did, substs);

            Some((variant, ty))
        } else {
            match ty.kind() {
                ty::Error(_) => {
                    // E0071 might be caused by a spelling error, which will have
                    // already caused an error message and probably a suggestion
                    // elsewhere. Refrain from emitting more unhelpful errors here
                    // (issue #88844).
                }
                _ => {
                    struct_span_err!(
                        self.tcx.sess,
                        path_span,
                        E0071,
                        "expected struct, variant or union type, found {}",
                        ty.sort_string(self.tcx)
                    )
                    .span_label(path_span, "not a struct")
                    .emit();
                }
            }
            None
        }
    }

    pub fn check_decl_initializer(
        &self,
        hir_id: hir::HirId,
        pat: &'tcx hir::Pat<'tcx>,
        init: &'tcx hir::Expr<'tcx>,
    ) -> Ty<'tcx> {
        // FIXME(tschottdorf): `contains_explicit_ref_binding()` must be removed
        // for #42640 (default match binding modes).
        //
        // See #44848.
        let ref_bindings = pat.contains_explicit_ref_binding();

        let local_ty = self.local_ty(init.span, hir_id).revealed_ty;
        if let Some(m) = ref_bindings {
            // Somewhat subtle: if we have a `ref` binding in the pattern,
            // we want to avoid introducing coercions for the RHS. This is
            // both because it helps preserve sanity and, in the case of
            // ref mut, for soundness (issue #23116). In particular, in
            // the latter case, we need to be clear that the type of the
            // referent for the reference that results is *equal to* the
            // type of the place it is referencing, and not some
            // supertype thereof.
            let init_ty = self.check_expr_with_needs(init, Needs::maybe_mut_place(m));
            self.demand_eqtype(init.span, local_ty, init_ty);
            init_ty
        } else {
            self.check_expr_coercable_to_type(init, local_ty, None)
        }
    }

    pub(in super::super) fn check_decl(&self, decl: Declaration<'tcx>) {
        // Determine and write the type which we'll check the pattern against.
        let decl_ty = self.local_ty(decl.span, decl.hir_id).decl_ty;
        self.write_ty(decl.hir_id, decl_ty);

        // Type check the initializer.
        if let Some(ref init) = decl.init {
            let init_ty = self.check_decl_initializer(decl.hir_id, decl.pat, &init);
            self.overwrite_local_ty_if_err(decl.hir_id, decl.pat, decl_ty, init_ty);
        }

        // Does the expected pattern type originate from an expression and what is the span?
        let (origin_expr, ty_span) = match (decl.ty, decl.init) {
            (Some(ty), _) => (false, Some(ty.span)), // Bias towards the explicit user type.
            (_, Some(init)) => (true, Some(init.span)), // No explicit type; so use the scrutinee.
            _ => (false, None), // We have `let $pat;`, so the expected type is unconstrained.
        };

        // Type check the pattern. Override if necessary to avoid knock-on errors.
        self.check_pat_top(&decl.pat, decl_ty, ty_span, origin_expr);
        let pat_ty = self.node_ty(decl.pat.hir_id);
        self.overwrite_local_ty_if_err(decl.hir_id, decl.pat, decl_ty, pat_ty);
    }

    /// Type check a `let` statement.
    pub fn check_decl_local(&self, local: &'tcx hir::Local<'tcx>) {
        self.check_decl(local.into());
    }

    pub fn check_stmt(&self, stmt: &'tcx hir::Stmt<'tcx>, is_last: bool) {
        // Don't do all the complex logic below for `DeclItem`.
        match stmt.kind {
            hir::StmtKind::Item(..) => return,
            hir::StmtKind::Local(..) | hir::StmtKind::Expr(..) | hir::StmtKind::Semi(..) => {}
        }

        self.warn_if_unreachable(stmt.hir_id, stmt.span, "statement");

        // Hide the outer diverging and `has_errors` flags.
        let old_diverges = self.diverges.replace(Diverges::Maybe);
        let old_has_errors = self.has_errors.replace(false);

        match stmt.kind {
            hir::StmtKind::Local(ref l) => {
                self.check_decl_local(&l);
            }
            // Ignore for now.
            hir::StmtKind::Item(_) => {}
            hir::StmtKind::Expr(ref expr) => {
                // Check with expected type of `()`.
                self.check_expr_has_type_or_error(&expr, self.tcx.mk_unit(), |err| {
                    if expr.can_have_side_effects() {
                        self.suggest_semicolon_at_end(expr.span, err);
                    }
                });
            }
            hir::StmtKind::Semi(ref expr) => {
                // All of this is equivalent to calling `check_expr`, but it is inlined out here
                // in order to capture the fact that this `match` is the last statement in its
                // function. This is done for better suggestions to remove the `;`.
                let expectation = match expr.kind {
                    hir::ExprKind::Match(..) if is_last => IsLast(stmt.span),
                    _ => NoExpectation,
                };
                self.check_expr_with_expectation(expr, expectation);
            }
        }

        // Combine the diverging and `has_error` flags.
        self.diverges.set(self.diverges.get() | old_diverges);
        self.has_errors.set(self.has_errors.get() | old_has_errors);
    }

    pub fn check_block_no_value(&self, blk: &'tcx hir::Block<'tcx>) {
        let unit = self.tcx.mk_unit();
        let ty = self.check_block_with_expected(blk, ExpectHasType(unit));

        // if the block produces a `!` value, that can always be
        // (effectively) coerced to unit.
        if !ty.is_never() {
            self.demand_suptype(blk.span, unit, ty);
        }
    }

    pub(in super::super) fn check_block_with_expected(
        &self,
        blk: &'tcx hir::Block<'tcx>,
        expected: Expectation<'tcx>,
    ) -> Ty<'tcx> {
        let prev = self.ps.replace(self.ps.get().recurse(blk));

        // In some cases, blocks have just one exit, but other blocks
        // can be targeted by multiple breaks. This can happen both
        // with labeled blocks as well as when we desugar
        // a `try { ... }` expression.
        //
        // Example 1:
        //
        //    'a: { if true { break 'a Err(()); } Ok(()) }
        //
        // Here we would wind up with two coercions, one from
        // `Err(())` and the other from the tail expression
        // `Ok(())`. If the tail expression is omitted, that's a
        // "forced unit" -- unless the block diverges, in which
        // case we can ignore the tail expression (e.g., `'a: {
        // break 'a 22; }` would not force the type of the block
        // to be `()`).
        let tail_expr = blk.expr.as_ref();
        let coerce_to_ty = expected.coercion_target_type(self, blk.span);
        let coerce = if blk.targeted_by_break {
            CoerceMany::new(coerce_to_ty)
        } else {
            let tail_expr: &[&hir::Expr<'_>] = match tail_expr {
                Some(e) => slice::from_ref(e),
                None => &[],
            };
            CoerceMany::with_coercion_sites(coerce_to_ty, tail_expr)
        };

        let prev_diverges = self.diverges.get();
        let ctxt = BreakableCtxt { coerce: Some(coerce), may_break: false };

        let (ctxt, ()) = self.with_breakable_ctxt(blk.hir_id, ctxt, || {
            for (pos, s) in blk.stmts.iter().enumerate() {
                self.check_stmt(s, blk.stmts.len() - 1 == pos);
            }

            // check the tail expression **without** holding the
            // `enclosing_breakables` lock below.
            let tail_expr_ty = tail_expr.map(|t| self.check_expr_with_expectation(t, expected));

            let mut enclosing_breakables = self.enclosing_breakables.borrow_mut();
            let ctxt = enclosing_breakables.find_breakable(blk.hir_id);
            let coerce = ctxt.coerce.as_mut().unwrap();
            if let Some(tail_expr_ty) = tail_expr_ty {
                let tail_expr = tail_expr.unwrap();
                let span = self.get_expr_coercion_span(tail_expr);
                let cause = self.cause(span, ObligationCauseCode::BlockTailExpression(blk.hir_id));
                coerce.coerce(self, &cause, tail_expr, tail_expr_ty);
            } else {
                // Subtle: if there is no explicit tail expression,
                // that is typically equivalent to a tail expression
                // of `()` -- except if the block diverges. In that
                // case, there is no value supplied from the tail
                // expression (assuming there are no other breaks,
                // this implies that the type of the block will be
                // `!`).
                //
                // #41425 -- label the implicit `()` as being the
                // "found type" here, rather than the "expected type".
                if !self.diverges.get().is_always() {
                    // #50009 -- Do not point at the entire fn block span, point at the return type
                    // span, as it is the cause of the requirement, and
                    // `consider_hint_about_removing_semicolon` will point at the last expression
                    // if it were a relevant part of the error. This improves usability in editors
                    // that highlight errors inline.
                    let mut sp = blk.span;
                    let mut fn_span = None;
                    if let Some((decl, ident)) = self.get_parent_fn_decl(blk.hir_id) {
                        let ret_sp = decl.output.span();
                        if let Some(block_sp) = self.parent_item_span(blk.hir_id) {
                            // HACK: on some cases (`ui/liveness/liveness-issue-2163.rs`) the
                            // output would otherwise be incorrect and even misleading. Make sure
                            // the span we're aiming at correspond to a `fn` body.
                            if block_sp == blk.span {
                                sp = ret_sp;
                                fn_span = Some(ident.span);
                            }
                        }
                    }
                    coerce.coerce_forced_unit(
                        self,
                        &self.misc(sp),
                        &mut |err| {
                            if let Some(expected_ty) = expected.only_has_type(self) {
                                self.consider_hint_about_removing_semicolon(blk, expected_ty, err);
                                if expected_ty == self.tcx.types.bool {
                                    // If this is caused by a missing `let` in a `while let`,
                                    // silence this redundant error, as we already emit E0070.
                                    let parent = self.tcx.hir().get_parent_node(blk.hir_id);
                                    let parent = self.tcx.hir().get_parent_node(parent);
                                    let parent = self.tcx.hir().get_parent_node(parent);
                                    let parent = self.tcx.hir().get_parent_node(parent);
                                    let parent = self.tcx.hir().get_parent_node(parent);
                                    match self.tcx.hir().find(parent) {
                                        Some(hir::Node::Expr(hir::Expr {
                                            kind: hir::ExprKind::Loop(_, _, hir::LoopSource::While, _),
                                            ..
                                        })) => {
                                            err.delay_as_bug();
                                        }
                                        _ => {}
                                    }
                                }
                            }
                            if let Some(fn_span) = fn_span {
                                err.span_label(
                                    fn_span,
                                    "implicitly returns `()` as its body has no tail or `return` \
                                     expression",
                                );
                            }
                        },
                        false,
                    );
                }
            }
        });

        if ctxt.may_break {
            // If we can break from the block, then the block's exit is always reachable
            // (... as long as the entry is reachable) - regardless of the tail of the block.
            self.diverges.set(prev_diverges);
        }

        let mut ty = ctxt.coerce.unwrap().complete(self);

        if self.has_errors.get() || ty.references_error() {
            ty = self.tcx.ty_error()
        }

        self.write_ty(blk.hir_id, ty);

        self.ps.set(prev);
        ty
    }

    /// A common error is to add an extra semicolon:
    ///
    /// ```
    /// fn foo() -> usize {
    ///     22;
    /// }
    /// ```
    ///
    /// This routine checks if the final statement in a block is an
    /// expression with an explicit semicolon whose type is compatible
    /// with `expected_ty`. If so, it suggests removing the semicolon.
    fn consider_hint_about_removing_semicolon(
        &self,
        blk: &'tcx hir::Block<'tcx>,
        expected_ty: Ty<'tcx>,
        err: &mut DiagnosticBuilder<'_>,
    ) {
        if let Some((span_semi, boxed)) = self.could_remove_semicolon(blk, expected_ty) {
            if let StatementAsExpression::NeedsBoxing = boxed {
                err.span_suggestion_verbose(
                    span_semi,
                    "consider removing this semicolon and boxing the expression",
                    String::new(),
                    Applicability::HasPlaceholders,
                );
            } else {
                err.span_suggestion_short(
                    span_semi,
                    "consider removing this semicolon",
                    String::new(),
                    Applicability::MachineApplicable,
                );
            }
        }
    }

    fn parent_item_span(&self, id: hir::HirId) -> Option<Span> {
        let node = self.tcx.hir().get(self.tcx.hir().get_parent_item(id));
        match node {
            Node::Item(&hir::Item { kind: hir::ItemKind::Fn(_, _, body_id), .. })
            | Node::ImplItem(&hir::ImplItem { kind: hir::ImplItemKind::Fn(_, body_id), .. }) => {
                let body = self.tcx.hir().body(body_id);
                if let ExprKind::Block(block, _) = &body.value.kind {
                    return Some(block.span);
                }
            }
            _ => {}
        }
        None
    }

    /// Given a function block's `HirId`, returns its `FnDecl` if it exists, or `None` otherwise.
    fn get_parent_fn_decl(&self, blk_id: hir::HirId) -> Option<(&'tcx hir::FnDecl<'tcx>, Ident)> {
        let parent = self.tcx.hir().get(self.tcx.hir().get_parent_item(blk_id));
        self.get_node_fn_decl(parent).map(|(fn_decl, ident, _)| (fn_decl, ident))
    }

    /// If `expr` is a `match` expression that has only one non-`!` arm, use that arm's tail
    /// expression's `Span`, otherwise return `expr.span`. This is done to give better errors
    /// when given code like the following:
    /// ```text
    /// if false { return 0i32; } else { 1u32 }
    /// //                               ^^^^ point at this instead of the whole `if` expression
    /// ```
    fn get_expr_coercion_span(&self, expr: &hir::Expr<'_>) -> rustc_span::Span {
        let check_in_progress = |elem: &hir::Expr<'_>| {
            self.in_progress_typeck_results
                .and_then(|typeck_results| typeck_results.borrow().node_type_opt(elem.hir_id))
                .and_then(|ty| {
                    if ty.is_never() {
                        None
                    } else {
                        Some(match elem.kind {
                            // Point at the tail expression when possible.
                            hir::ExprKind::Block(block, _) => {
                                block.expr.map_or(block.span, |e| e.span)
                            }
                            _ => elem.span,
                        })
                    }
                })
        };

        if let hir::ExprKind::If(_, _, Some(el)) = expr.kind {
            if let Some(rslt) = check_in_progress(el) {
                return rslt;
            }
        }

        if let hir::ExprKind::Match(_, arms, _) = expr.kind {
            let mut iter = arms.iter().filter_map(|arm| check_in_progress(arm.body));
            if let Some(span) = iter.next() {
                if iter.next().is_none() {
                    return span;
                }
            }
        }

        expr.span
    }

    fn overwrite_local_ty_if_err(
        &self,
        hir_id: hir::HirId,
        pat: &'tcx hir::Pat<'tcx>,
        decl_ty: Ty<'tcx>,
        ty: Ty<'tcx>,
    ) {
        if ty.references_error() {
            // Override the types everywhere with `err()` to avoid knock on errors.
            self.write_ty(hir_id, ty);
            self.write_ty(pat.hir_id, ty);
            let local_ty = LocalTy { decl_ty, revealed_ty: ty };
            self.locals.borrow_mut().insert(hir_id, local_ty);
            self.locals.borrow_mut().insert(pat.hir_id, local_ty);
        }
    }

    // Finish resolving a path in a struct expression or pattern `S::A { .. }` if necessary.
    // The newly resolved definition is written into `type_dependent_defs`.
    fn finish_resolving_struct_path(
        &self,
        qpath: &QPath<'_>,
        path_span: Span,
        hir_id: hir::HirId,
    ) -> (Res, Ty<'tcx>) {
        match *qpath {
            QPath::Resolved(ref maybe_qself, ref path) => {
                let self_ty = maybe_qself.as_ref().map(|qself| self.to_ty(qself));
                let ty = <dyn AstConv<'_>>::res_to_ty(self, self_ty, path, true);
                (path.res, ty)
            }
            QPath::TypeRelative(ref qself, ref segment) => {
                let ty = self.to_ty(qself);

                let res = if let hir::TyKind::Path(QPath::Resolved(_, ref path)) = qself.kind {
                    path.res
                } else {
                    Res::Err
                };
                let result = <dyn AstConv<'_>>::associated_path_to_ty(
                    self, hir_id, path_span, ty, res, segment, true,
                );
                let ty = result.map(|(ty, _, _)| ty).unwrap_or_else(|_| self.tcx().ty_error());
                let result = result.map(|(_, kind, def_id)| (kind, def_id));

                // Write back the new resolution.
                self.write_resolution(hir_id, result);

                (result.map_or(Res::Err, |(kind, def_id)| Res::Def(kind, def_id)), ty)
            }
            QPath::LangItem(lang_item, span, id) => {
                self.resolve_lang_item_path(lang_item, span, hir_id, id)
            }
        }
    }

    /// Given a vec of evaluated `FulfillmentError`s and an `fn` call argument expressions, we walk
    /// the checked and coerced types for each argument to see if any of the `FulfillmentError`s
    /// reference a type argument. The reason to walk also the checked type is that the coerced type
    /// can be not easily comparable with predicate type (because of coercion). If the types match
    /// for either checked or coerced type, and there's only *one* argument that does, we point at
    /// the corresponding argument's expression span instead of the `fn` call path span.
    fn point_at_arg_instead_of_call_if_possible(
        &self,
        errors: &mut Vec<traits::FulfillmentError<'tcx>>,
        final_arg_types: &[(usize, Ty<'tcx>, Ty<'tcx>)],
        expr: &'tcx hir::Expr<'tcx>,
        call_sp: Span,
        args: &'tcx [hir::Expr<'tcx>],
    ) {
        // We *do not* do this for desugared call spans to keep good diagnostics when involving
        // the `?` operator.
        if call_sp.desugaring_kind().is_some() {
            return;
        }

        for error in errors {
            // Only if the cause is somewhere inside the expression we want try to point at arg.
            // Otherwise, it means that the cause is somewhere else and we should not change
            // anything because we can break the correct span.
            if !call_sp.contains(error.obligation.cause.span) {
                continue;
            }

            // Peel derived obligation, because it's the type that originally
            // started this inference chain that matters, not the one we wound
            // up with at the end.
            fn unpeel_to_top(
                mut code: Lrc<ObligationCauseCode<'_>>,
            ) -> Lrc<ObligationCauseCode<'_>> {
                let mut result_code = code.clone();
                loop {
                    let parent = match &*code {
                        ObligationCauseCode::BuiltinDerivedObligation(c)
                        | ObligationCauseCode::ImplDerivedObligation(c)
                        | ObligationCauseCode::DerivedObligation(c) => c.parent_code.clone(),
                        _ => break,
                    };
                    result_code = std::mem::replace(&mut code, parent);
                }
                result_code
            }
            let self_: ty::subst::GenericArg<'_> = match &*unpeel_to_top(error.obligation.cause.clone_code()) {
                ObligationCauseCode::BuiltinDerivedObligation(code) |
                ObligationCauseCode::ImplDerivedObligation(code) |
                ObligationCauseCode::DerivedObligation(code) => {
                    code.parent_trait_ref.self_ty().skip_binder().into()
                }
                _ if let ty::PredicateKind::Trait(predicate) =
                    error.obligation.predicate.kind().skip_binder() => {
                        predicate.self_ty().into()
                    }
                _ =>  continue,
            };
            let self_ = self.resolve_vars_if_possible(self_);

            // Collect the argument position for all arguments that could have caused this
            // `FulfillmentError`.
            let mut referenced_in = final_arg_types
                .iter()
                .map(|&(i, checked_ty, _)| (i, checked_ty))
                .chain(final_arg_types.iter().map(|&(i, _, coerced_ty)| (i, coerced_ty)))
                .flat_map(|(i, ty)| {
                    let ty = self.resolve_vars_if_possible(ty);
                    // We walk the argument type because the argument's type could have
                    // been `Option<T>`, but the `FulfillmentError` references `T`.
                    if ty.walk(self.tcx).any(|arg| arg == self_) { Some(i) } else { None }
                })
                .collect::<Vec<usize>>();

            // Both checked and coerced types could have matched, thus we need to remove
            // duplicates.

            // We sort primitive type usize here and can use unstable sort
            referenced_in.sort_unstable();
            referenced_in.dedup();

            if let (Some(ref_in), None) = (referenced_in.pop(), referenced_in.pop()) {
                // Do not point at the inside of a macro.
                // That would often result in poor error messages.
                if args[ref_in].span.from_expansion() {
                    return;
                }
                // We make sure that only *one* argument matches the obligation failure
                // and we assign the obligation's span to its expression's.
                error.obligation.cause.span = args[ref_in].span;
                let parent_code = error.obligation.cause.clone_code();
                *error.obligation.cause.make_mut_code() =
                    ObligationCauseCode::FunctionArgumentObligation {
                        arg_hir_id: args[ref_in].hir_id,
                        call_hir_id: expr.hir_id,
                        parent_code,
                    };
            } else if error.obligation.cause.span == call_sp {
                // Make function calls point at the callee, not the whole thing.
                if let hir::ExprKind::Call(callee, _) = expr.kind {
                    error.obligation.cause.span = callee.span;
                }
            }
        }
    }

    /// Given a vec of evaluated `FulfillmentError`s and an `fn` call expression, we walk the
    /// `PathSegment`s and resolve their type parameters to see if any of the `FulfillmentError`s
    /// were caused by them. If they were, we point at the corresponding type argument's span
    /// instead of the `fn` call path span.
    fn point_at_type_arg_instead_of_call_if_possible(
        &self,
        errors: &mut Vec<traits::FulfillmentError<'tcx>>,
        call_expr: &'tcx hir::Expr<'tcx>,
    ) {
        if let hir::ExprKind::Call(path, _) = &call_expr.kind {
            if let hir::ExprKind::Path(hir::QPath::Resolved(_, path)) = &path.kind {
                for error in errors {
                    if let ty::PredicateKind::Trait(predicate) =
                        error.obligation.predicate.kind().skip_binder()
                    {
                        // If any of the type arguments in this path segment caused the
                        // `FulfillmentError`, point at its span (#61860).
                        for arg in path
                            .segments
                            .iter()
                            .filter_map(|seg| seg.args.as_ref())
                            .flat_map(|a| a.args.iter())
                        {
                            if let hir::GenericArg::Type(hir_ty) = &arg {
                                if let hir::TyKind::Path(hir::QPath::TypeRelative(..)) =
                                    &hir_ty.kind
                                {
                                    // Avoid ICE with associated types. As this is best
                                    // effort only, it's ok to ignore the case. It
                                    // would trigger in `is_send::<T::AssocType>();`
                                    // from `typeck-default-trait-impl-assoc-type.rs`.
                                } else {
                                    let ty = <dyn AstConv<'_>>::ast_ty_to_ty(self, hir_ty);
                                    let ty = self.resolve_vars_if_possible(ty);
                                    if ty == predicate.self_ty() {
                                        error.obligation.cause.span = hir_ty.span;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
