// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! this module contains various codegen hooks for functions.
//! e.g.
//! functions start with [__nondet] is silently replaced by nondeterministic values, and
//! [begin_panic] is replaced by [assert(false)], etc.
//!
//! It would be too nasty if we spread around these sort of undocumented hooks in place, so
//! this module addresses this issue.

use crate::utils;
use crate::GotocCtx;
use cbmc::goto_program::{BuiltinFn, Expr, Location, Stmt, Symbol, Type};
use cbmc::NO_PRETTY_NAME;
use kani_queries::UserInput;
use rustc_middle::mir::{BasicBlock, Place};
use rustc_middle::ty::layout::LayoutOf;
use rustc_middle::ty::print::with_no_trimmed_paths;
use rustc_middle::ty::{self, Instance, InstanceDef, TyCtxt};
use rustc_span::Span;
use std::rc::Rc;
use tracing::{debug, warn};

pub trait GotocHook<'tcx> {
    /// if the hook applies, it means the codegen would do something special to it
    fn hook_applies(&self, tcx: TyCtxt<'tcx>, instance: Instance<'tcx>) -> bool;
    /// the handler for codegen
    fn handle(
        &self,
        tcx: &mut GotocCtx<'tcx>,
        instance: Instance<'tcx>,
        fargs: Vec<Expr>,
        assign_to: Option<Place<'tcx>>,
        target: Option<BasicBlock>,
        span: Option<Span>,
    ) -> Stmt;
}

fn output_of_instance_is_never<'tcx>(tcx: TyCtxt<'tcx>, instance: Instance<'tcx>) -> bool {
    let ty = instance.ty(tcx, ty::ParamEnv::reveal_all());
    match ty.kind() {
        ty::Closure(_, substs) => tcx
            .normalize_erasing_late_bound_regions(
                ty::ParamEnv::reveal_all(),
                substs.as_closure().sig(),
            )
            .output()
            .is_never(),
        ty::FnDef(..) | ty::FnPtr(..) => tcx
            .normalize_erasing_late_bound_regions(ty::ParamEnv::reveal_all(), ty.fn_sig(tcx))
            .output()
            .is_never(),
        ty::Generator(_, substs, _) => substs.as_generator().return_ty().is_never(),
        _ => {
            unreachable!(
                "Can't take get ouput type of instance:\n{:?}\nType kind:\n{:?}",
                ty,
                ty.kind()
            )
        }
    }
}

fn matches_function(tcx: TyCtxt<'tcx>, instance: Instance<'tcx>, attr_name: &str) -> bool {
    let attr_sym = rustc_span::symbol::Symbol::intern(attr_name);
    if let Some(attr_id) = tcx.all_diagnostic_items(()).name_to_id.get(&attr_sym) {
        if instance.def.def_id() == *attr_id {
            debug!("matched: {:?} {:?}", attr_id, attr_sym);
            return true;
        }
    }
    false
}

struct ExpectFail;
impl<'tcx> GotocHook<'tcx> for ExpectFail {
    fn hook_applies(&self, tcx: TyCtxt<'tcx>, instance: Instance<'tcx>) -> bool {
        // Deprecate old __VERIFIER notation that doesn't respect rust naming conventions.
        // Complete removal is tracked here: https://github.com/model-checking/kani/issues/599
        if utils::instance_name_starts_with(tcx, instance, "__VERIFIER_expect_fail") {
            warn!(
                "The function __VERIFIER_expect_fail is deprecated. Use kani::expect_fail instead"
            );
            return true;
        }
        matches_function(tcx, instance, "KaniExpectFail")
    }

    fn handle(
        &self,
        tcx: &mut GotocCtx<'tcx>,
        _instance: Instance<'tcx>,
        mut fargs: Vec<Expr>,
        _assign_to: Option<Place<'tcx>>,
        target: Option<BasicBlock>,
        span: Option<Span>,
    ) -> Stmt {
        assert_eq!(fargs.len(), 2);
        let target = target.unwrap();
        let cond = fargs.remove(0).cast_to(Type::bool());
        //TODO: actually use the error message passed by the user.
        let msg = "EXPECTED FAIL";
        let loc = tcx.codegen_span_option(span);
        Stmt::block(
            vec![
                Stmt::assert(cond, msg, loc.clone()),
                Stmt::goto(tcx.current_fn().find_label(&target), loc.clone()),
            ],
            loc,
        )
    }
}

struct Assume;
impl<'tcx> GotocHook<'tcx> for Assume {
    fn hook_applies(&self, tcx: TyCtxt<'tcx>, instance: Instance<'tcx>) -> bool {
        // Deprecate old __VERIFIER notation that doesn't respect rust naming conventions.
        // Complete removal is tracked here: https://github.com/model-checking/kani/issues/599
        if utils::instance_name_starts_with(tcx, instance, "__VERIFIER_assume") {
            warn!("The function __VERIFIER_assume is deprecated. Use kani::assume instead");
            return true;
        }
        matches_function(tcx, instance, "KaniAssume")
    }

    fn handle(
        &self,
        tcx: &mut GotocCtx<'tcx>,
        _instance: Instance<'tcx>,
        mut fargs: Vec<Expr>,
        _assign_to: Option<Place<'tcx>>,
        target: Option<BasicBlock>,
        span: Option<Span>,
    ) -> Stmt {
        assert_eq!(fargs.len(), 1);
        let cond = fargs.remove(0).cast_to(Type::bool());
        let target = target.unwrap();
        let loc = tcx.codegen_span_option(span);

        Stmt::block(
            vec![
                Stmt::assume(cond, loc.clone()),
                Stmt::goto(tcx.current_fn().find_label(&target), loc.clone()),
            ],
            loc,
        )
    }
}

struct Assert;
impl<'tcx> GotocHook<'tcx> for Assert {
    fn hook_applies(&self, tcx: TyCtxt<'tcx>, instance: Instance<'tcx>) -> bool {
        matches_function(tcx, instance, "KaniAssert")
    }

    fn handle(
        &self,
        tcx: &mut GotocCtx<'tcx>,
        _instance: Instance<'tcx>,
        mut fargs: Vec<Expr>,
        _assign_to: Option<Place<'tcx>>,
        target: Option<BasicBlock>,
        span: Option<Span>,
    ) -> Stmt {
        assert_eq!(fargs.len(), 2);
        let cond = fargs.remove(0).cast_to(Type::bool());
        let msg = fargs.remove(0);
        let mut msg = utils::extract_const_message(&msg).unwrap();
        let target = target.unwrap();
        let caller_loc = tcx.codegen_caller_span(&span);

        let mut stmts: Vec<Stmt> = Vec::new();

        if tcx.queries.get_check_assertion_reachability() {
            // Generate a unique ID for the assert
            let assert_id = tcx.next_check_id();
            // Use a description of the form:
            // [KANI_REACHABILITY_CHECK] <check ID>
            // for reachability checks
            msg = format!("[{}] {}", assert_id, msg);
            let reach_msg = format!("[KANI_REACHABILITY_CHECK] {}", assert_id);
            // inject a reachability (cover) check to the current location
            stmts.push(tcx.codegen_cover_loc(&reach_msg, span));
        }

        // Since `cond` might have side effects, assign it to a temporary
        // variable so that it's evaluated once, then assert and assume it
        let tmp = tcx.gen_temp_variable(cond.typ().clone(), caller_loc.clone()).to_expr();
        stmts.append(&mut vec![
            Stmt::decl(tmp.clone(), Some(cond), caller_loc.clone()),
            Stmt::assert(tmp.clone(), &msg, caller_loc.clone()),
            Stmt::assume(tmp, caller_loc.clone()),
            Stmt::goto(tcx.current_fn().find_label(&target), caller_loc.clone()),
        ]);
        Stmt::block(stmts, caller_loc)
    }
}

struct Nondet;

impl<'tcx> GotocHook<'tcx> for Nondet {
    fn hook_applies(&self, tcx: TyCtxt<'tcx>, instance: Instance<'tcx>) -> bool {
        // Deprecate old __nondet since it doesn't match rust naming conventions.
        // Complete removal is tracked here: https://github.com/model-checking/kani/issues/599
        if utils::instance_name_starts_with(tcx, instance, "__nondet") {
            warn!("The function __nondet is deprecated. Use kani::any instead");
            return true;
        }
        matches_function(tcx, instance, "KaniAnyRaw")
    }

    fn handle(
        &self,
        tcx: &mut GotocCtx<'tcx>,
        _instance: Instance<'tcx>,
        fargs: Vec<Expr>,
        assign_to: Option<Place<'tcx>>,
        target: Option<BasicBlock>,
        span: Option<Span>,
    ) -> Stmt {
        assert!(fargs.is_empty());
        let loc = tcx.codegen_span_option(span);
        let p = assign_to.unwrap();
        let target = target.unwrap();
        let pt = tcx.place_ty(&p);
        if pt.is_unit() {
            Stmt::goto(tcx.current_fn().find_label(&target), loc)
        } else {
            let pe = tcx.codegen_place(&p).goto_expr;
            Stmt::block(
                vec![
                    pe.clone().assign(tcx.codegen_ty(pt).nondet(), loc.clone()),
                    Stmt::goto(tcx.current_fn().find_label(&target), loc.clone()),
                ],
                loc,
            )
        }
    }
}

struct Panic;

impl<'tcx> GotocHook<'tcx> for Panic {
    fn hook_applies(&self, tcx: TyCtxt<'tcx>, instance: Instance<'tcx>) -> bool {
        let def_id = instance.def.def_id();
        Some(def_id) == tcx.lang_items().panic_fn()
            || Some(def_id) == tcx.lang_items().panic_display()
            || Some(def_id) == tcx.lang_items().panic_fmt()
            || Some(def_id) == tcx.lang_items().begin_panic_fn()
    }

    fn handle(
        &self,
        tcx: &mut GotocCtx<'tcx>,
        _instance: Instance<'tcx>,
        fargs: Vec<Expr>,
        _assign_to: Option<Place<'tcx>>,
        _target: Option<BasicBlock>,
        span: Option<Span>,
    ) -> Stmt {
        tcx.codegen_panic(span, fargs)
    }
}

struct Nevers;

impl<'tcx> GotocHook<'tcx> for Nevers {
    fn hook_applies(&self, tcx: TyCtxt<'tcx>, instance: Instance<'tcx>) -> bool {
        output_of_instance_is_never(tcx, instance)
    }

    fn handle(
        &self,
        tcx: &mut GotocCtx<'tcx>,
        instance: Instance<'tcx>,
        _fargs: Vec<Expr>,
        _assign_to: Option<Place<'tcx>>,
        _target: Option<BasicBlock>,
        span: Option<Span>,
    ) -> Stmt {
        let msg = format!(
            "a panicking function {} is invoked",
            with_no_trimmed_paths(|| tcx.tcx.def_path_str(instance.def_id()))
        );
        tcx.codegen_fatal_error(&msg, span)
    }
}

struct Intrinsic;

impl<'tcx> GotocHook<'tcx> for Intrinsic {
    fn hook_applies(&self, _tcx: TyCtxt<'tcx>, instance: Instance<'tcx>) -> bool {
        match instance.def {
            InstanceDef::Intrinsic(_) => true,
            _ => false,
        }
    }

    fn handle(
        &self,
        tcx: &mut GotocCtx<'tcx>,
        instance: Instance<'tcx>,
        fargs: Vec<Expr>,
        assign_to: Option<Place<'tcx>>,
        target: Option<BasicBlock>,
        span: Option<Span>,
    ) -> Stmt {
        match assign_to {
            None => tcx.codegen_never_return_intrinsic(instance, span),
            Some(assign_to) => {
                let target = target.unwrap();
                let loc = tcx.codegen_span_option(span);
                Stmt::block(
                    vec![
                        tcx.codegen_intrinsic(instance, fargs, &assign_to, span),
                        Stmt::goto(tcx.current_fn().find_label(&target), loc.clone()),
                    ],
                    loc,
                )
            }
        }
    }
}

struct MemReplace;

impl<'tcx> GotocHook<'tcx> for MemReplace {
    fn hook_applies(&self, tcx: TyCtxt<'tcx>, instance: Instance<'tcx>) -> bool {
        let name = with_no_trimmed_paths(|| tcx.def_path_str(instance.def_id()));
        name == "core::mem::replace" || name == "std::mem::replace"
    }

    fn handle(
        &self,
        tcx: &mut GotocCtx<'tcx>,
        _instance: Instance<'tcx>,
        mut fargs: Vec<Expr>,
        assign_to: Option<Place<'tcx>>,
        target: Option<BasicBlock>,
        span: Option<Span>,
    ) -> Stmt {
        let loc = tcx.codegen_span_option(span);
        let p = assign_to.unwrap();
        let target = target.unwrap();
        // Skip an assignment to a destination that has a zero-sized type
        // (For a ZST, Rust optimizes away the source and fargs.len() == 1)
        let place_type = tcx.place_ty(&p);
        let place_layout = tcx.layout_of(place_type);
        let place_is_zst = place_layout.is_zst();
        if place_is_zst {
            Stmt::block(vec![Stmt::goto(tcx.current_fn().find_label(&target), loc.clone())], loc)
        } else {
            let dest = fargs.remove(0);
            let src = fargs.remove(0);
            Stmt::block(
                vec![
                    tcx.codegen_place(&p)
                        .goto_expr
                        .assign(dest.clone().dereference().with_location(loc.clone()), loc.clone()),
                    dest.dereference().assign(src, loc.clone()),
                    Stmt::goto(tcx.current_fn().find_label(&target), loc.clone()),
                ],
                loc,
            )
        }
    }
}

struct MemSwap;

impl<'tcx> GotocHook<'tcx> for MemSwap {
    fn hook_applies(&self, tcx: TyCtxt<'tcx>, instance: Instance<'tcx>) -> bool {
        // We need to keep the old std / core functions here because we don't compile std yet.
        let name = with_no_trimmed_paths(|| tcx.def_path_str(instance.def_id()));
        name == "core::mem::swap"
            || name == "std::mem::swap"
            || name == "core::ptr::swap"
            || name == "std::ptr::swap"
    }

    fn handle(
        &self,
        tcx: &mut GotocCtx<'tcx>,
        instance: Instance<'tcx>,
        mut fargs: Vec<Expr>,
        _assign_to: Option<Place<'tcx>>,
        target: Option<BasicBlock>,
        span: Option<Span>,
    ) -> Stmt {
        let ty = tcx.monomorphize(instance.substs.type_at(0));
        let loc = tcx.codegen_span_option(span);
        let target = target.unwrap();
        let x = fargs.remove(0);
        let y = fargs.remove(0);

        let func_name = format!("gen-swap<{}>", tcx.ty_mangled_name(ty));
        tcx.ensure(&func_name, |tcx, _| {
            let ty = tcx.codegen_ty(ty);
            let x_param = tcx.gen_function_local_variable(1, &func_name, ty.clone().to_pointer());
            let y_param = tcx.gen_function_local_variable(2, &func_name, ty.clone().to_pointer());
            let var = tcx.gen_function_local_variable(3, &func_name, ty);
            let mut block = Vec::new();
            let xe = x_param.to_expr();
            block.push(Stmt::decl(var.to_expr(), Some(xe.clone().dereference()), Location::none()));
            let ye = y_param.to_expr();
            let var = var.to_expr();
            block.push(xe.dereference().assign(ye.clone().dereference(), loc.clone()));
            block.push(ye.dereference().assign(var, loc.clone()));

            Symbol::function(
                &func_name,
                Type::code(
                    vec![x_param.to_function_parameter(), y_param.to_function_parameter()],
                    Type::empty(),
                ),
                Some(Stmt::block(block, loc.clone())),
                NO_PRETTY_NAME,
                Location::none(),
            )
        });

        Stmt::block(
            vec![
                tcx.find_function(&func_name).unwrap().call(vec![x, y]).as_stmt(loc.clone()),
                Stmt::goto(tcx.current_fn().find_label(&target), loc.clone()),
            ],
            loc,
        )
    }
}

struct PtrRead;

impl<'tcx> GotocHook<'tcx> for PtrRead {
    fn hook_applies(&self, tcx: TyCtxt<'tcx>, instance: Instance<'tcx>) -> bool {
        let name = with_no_trimmed_paths(|| tcx.def_path_str(instance.def_id()));
        name == "core::ptr::read"
            || name == "core::ptr::read_unaligned"
            || name == "core::ptr::read_volatile"
            || name == "std::ptr::read"
            || name == "std::ptr::read_unaligned"
            || name == "std::ptr::read_volatile"
    }

    fn handle(
        &self,
        tcx: &mut GotocCtx<'tcx>,
        _instance: Instance<'tcx>,
        mut fargs: Vec<Expr>,
        assign_to: Option<Place<'tcx>>,
        target: Option<BasicBlock>,
        span: Option<Span>,
    ) -> Stmt {
        let loc = tcx.codegen_span_option(span);
        let p = assign_to.unwrap();
        let target = target.unwrap();
        let src = fargs.remove(0);
        Stmt::block(
            vec![
                tcx.codegen_place(&p)
                    .goto_expr
                    .assign(src.dereference().with_location(loc.clone()), loc.clone()),
                Stmt::goto(tcx.current_fn().find_label(&target), loc.clone()),
            ],
            loc,
        )
    }
}

struct PtrWrite;

impl<'tcx> GotocHook<'tcx> for PtrWrite {
    fn hook_applies(&self, tcx: TyCtxt<'tcx>, instance: Instance<'tcx>) -> bool {
        let name = with_no_trimmed_paths(|| tcx.def_path_str(instance.def_id()));
        name == "core::ptr::write"
            || name == "core::ptr::write_unaligned"
            || name == "core::ptr::write_volatile"
            || name == "std::ptr::write"
            || name == "std::ptr::write_unaligned"
            || name == "std::ptr::write_volatile"
    }

    fn handle(
        &self,
        tcx: &mut GotocCtx<'tcx>,
        _instance: Instance<'tcx>,
        mut fargs: Vec<Expr>,
        _assign_to: Option<Place<'tcx>>,
        target: Option<BasicBlock>,
        span: Option<Span>,
    ) -> Stmt {
        let loc = tcx.codegen_span_option(span);
        let target = target.unwrap();
        let dst = fargs.remove(0);
        let src = fargs.remove(0);
        Stmt::block(
            vec![
                dst.dereference().assign(src, loc.clone()).with_location(loc.clone()),
                Stmt::goto(tcx.current_fn().find_label(&target), loc.clone()),
            ],
            loc,
        )
    }
}

struct RustAlloc;

impl<'tcx> GotocHook<'tcx> for RustAlloc {
    fn hook_applies(&self, tcx: TyCtxt<'tcx>, instance: Instance<'tcx>) -> bool {
        let name = tcx.symbol_name(instance).name.to_string();
        let full_name = with_no_trimmed_paths(|| tcx.def_path_str(instance.def_id()));
        name == "__rust_alloc" || full_name == "alloc::alloc::exchange_malloc"
    }

    fn handle(
        &self,
        tcx: &mut GotocCtx<'tcx>,
        instance: Instance<'tcx>,
        mut fargs: Vec<Expr>,
        assign_to: Option<Place<'tcx>>,
        target: Option<BasicBlock>,
        span: Option<Span>,
    ) -> Stmt {
        debug!(?instance, "Replace allocation");
        let loc = tcx.codegen_span_option(span);
        match (assign_to, target) {
            (Some(p), Some(target)) => {
                let size = fargs.remove(0);
                Stmt::block(
                    vec![
                        tcx.codegen_place(&p).goto_expr.assign(
                            BuiltinFn::Malloc
                                .call(vec![size], loc.clone())
                                .cast_to(Type::unsigned_int(8).to_pointer()),
                            loc,
                        ),
                        Stmt::goto(tcx.current_fn().find_label(&target), Location::none()),
                    ],
                    Location::none(),
                )
            }
            _ => unreachable!(),
        }
    }
}

struct RustDealloc;

impl<'tcx> GotocHook<'tcx> for RustDealloc {
    fn hook_applies(&self, tcx: TyCtxt<'tcx>, instance: Instance<'tcx>) -> bool {
        let name = tcx.symbol_name(instance).name.to_string();
        name == "__rust_dealloc"
    }

    fn handle(
        &self,
        tcx: &mut GotocCtx<'tcx>,
        _instance: Instance<'tcx>,
        mut fargs: Vec<Expr>,
        _assign_to: Option<Place<'tcx>>,
        target: Option<BasicBlock>,
        span: Option<Span>,
    ) -> Stmt {
        let loc = tcx.codegen_span_option(span);
        match target {
            Some(target) => {
                let ptr = fargs.remove(0);
                Stmt::block(
                    vec![
                        BuiltinFn::Free
                            .call(vec![ptr.cast_to(Type::void_pointer())], loc.clone())
                            .as_stmt(loc.clone()),
                        Stmt::goto(tcx.current_fn().find_label(&target), Location::none()),
                    ],
                    loc,
                )
            }
            _ => unreachable!(),
        }
    }
}

struct RustRealloc;

impl<'tcx> GotocHook<'tcx> for RustRealloc {
    fn hook_applies(&self, tcx: TyCtxt<'tcx>, instance: Instance<'tcx>) -> bool {
        let name = tcx.symbol_name(instance).name.to_string();
        name == "__rust_realloc"
    }

    fn handle(
        &self,
        tcx: &mut GotocCtx<'tcx>,
        _instance: Instance<'tcx>,
        mut fargs: Vec<Expr>,
        assign_to: Option<Place<'tcx>>,
        target: Option<BasicBlock>,
        span: Option<Span>,
    ) -> Stmt {
        let loc = tcx.codegen_span_option(span);
        let p = assign_to.unwrap();
        let target = target.unwrap();
        let ptr = fargs.remove(0).cast_to(Type::void_pointer());
        fargs.remove(0); // old_size
        fargs.remove(0); // align
        let size = fargs.remove(0);
        Stmt::block(
            vec![
                tcx.codegen_place(&p).goto_expr.assign(
                    BuiltinFn::Realloc
                        .call(vec![ptr, size], loc.clone())
                        .cast_to(Type::unsigned_int(8).to_pointer()),
                    loc.clone(),
                ),
                Stmt::goto(tcx.current_fn().find_label(&target), loc.clone()),
            ],
            loc,
        )
    }
}

struct RustAllocZeroed;

impl<'tcx> GotocHook<'tcx> for RustAllocZeroed {
    fn hook_applies(&self, tcx: TyCtxt<'tcx>, instance: Instance<'tcx>) -> bool {
        let name = tcx.symbol_name(instance).name.to_string();
        name == "__rust_alloc_zeroed"
    }

    fn handle(
        &self,
        tcx: &mut GotocCtx<'tcx>,
        _instance: Instance<'tcx>,
        mut fargs: Vec<Expr>,
        assign_to: Option<Place<'tcx>>,
        target: Option<BasicBlock>,
        span: Option<Span>,
    ) -> Stmt {
        let loc = tcx.codegen_span_option(span);
        let p = assign_to.unwrap();
        let target = target.unwrap();
        let size = fargs.remove(0);
        Stmt::block(
            vec![
                tcx.codegen_place(&p).goto_expr.assign(
                    BuiltinFn::Calloc
                        .call(vec![Type::size_t().one(), size], loc.clone())
                        .cast_to(Type::unsigned_int(8).to_pointer()),
                    loc.clone(),
                ),
                Stmt::goto(tcx.current_fn().find_label(&target), loc.clone()),
            ],
            loc,
        )
    }
}

struct SliceFromRawPart;

impl<'tcx> GotocHook<'tcx> for SliceFromRawPart {
    fn hook_applies(&self, tcx: TyCtxt<'tcx>, instance: Instance<'tcx>) -> bool {
        let name = with_no_trimmed_paths(|| tcx.def_path_str(instance.def_id()));
        name == "core::ptr::slice_from_raw_parts"
            || name == "std::ptr::slice_from_raw_parts"
            || name == "core::ptr::slice_from_raw_parts_mut"
            || name == "std::ptr::slice_from_raw_parts_mut"
    }

    fn handle(
        &self,
        tcx: &mut GotocCtx<'tcx>,
        _instance: Instance<'tcx>,
        mut fargs: Vec<Expr>,
        assign_to: Option<Place<'tcx>>,
        target: Option<BasicBlock>,
        span: Option<Span>,
    ) -> Stmt {
        let loc = tcx.codegen_span_option(span);
        let p = assign_to.unwrap();
        let target = target.unwrap();
        let pt = tcx.codegen_ty(tcx.place_ty(&p));
        let data = fargs.remove(0);
        let len = fargs.remove(0);
        let code = tcx
            .codegen_place(&p)
            .goto_expr
            .assign(
                Expr::struct_expr_from_values(pt, vec![data, len], &tcx.symbol_table),
                loc.clone(),
            )
            .with_location(loc.clone());
        Stmt::block(vec![code, Stmt::goto(tcx.current_fn().find_label(&target), loc.clone())], loc)
    }
}

pub fn fn_hooks<'tcx>() -> GotocHooks<'tcx> {
    GotocHooks {
        hooks: vec![
            Rc::new(Panic), //Must go first, so it overrides Nevers
            Rc::new(Assume),
            Rc::new(Assert),
            Rc::new(ExpectFail),
            Rc::new(Intrinsic),
            Rc::new(MemReplace),
            Rc::new(MemSwap),
            Rc::new(Nevers),
            Rc::new(Nondet),
            Rc::new(PtrRead),
            Rc::new(PtrWrite),
            Rc::new(RustAlloc),
            Rc::new(RustAllocZeroed),
            Rc::new(RustDealloc),
            Rc::new(RustRealloc),
            Rc::new(SliceFromRawPart),
        ],
    }
}

pub struct GotocHooks<'tcx> {
    hooks: Vec<Rc<dyn GotocHook<'tcx> + 'tcx>>,
}

impl<'tcx> GotocHooks<'tcx> {
    pub fn hook_applies(
        &self,
        tcx: TyCtxt<'tcx>,
        instance: Instance<'tcx>,
    ) -> Option<Rc<dyn GotocHook<'tcx> + 'tcx>> {
        for h in &self.hooks {
            if h.hook_applies(tcx, instance) {
                return Some(h.clone());
            }
        }
        None
    }
}
