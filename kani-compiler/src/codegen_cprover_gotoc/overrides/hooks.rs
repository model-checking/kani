// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! this module contains various codegen hooks for functions.
//! e.g.
//! functions start with [__nondet] is silently replaced by nondeterministic values, and
//! [begin_panic] is replaced by [assert(false)], etc.
//!
//! It would be too nasty if we spread around these sort of undocumented hooks in place, so
//! this module addresses this issue.

use crate::codegen_cprover_gotoc::codegen::PropertyClass;
use crate::codegen_cprover_gotoc::utils;
use crate::codegen_cprover_gotoc::GotocCtx;
use crate::unwrap_or_return_codegen_unimplemented_stmt;
use cbmc::goto_program::{BuiltinFn, Expr, Location, Stmt, Type};
use kani_queries::UserInput;
use rustc_middle::mir::{BasicBlock, Place};
use rustc_middle::ty::print::with_no_trimmed_paths;
use rustc_middle::ty::{Instance, TyCtxt};
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
        assign_to: Place<'tcx>,
        target: Option<BasicBlock>,
        span: Option<Span>,
    ) -> Stmt;
}

fn matches_function(tcx: TyCtxt, instance: Instance, attr_name: &str) -> bool {
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
        _assign_to: Place<'tcx>,
        target: Option<BasicBlock>,
        span: Option<Span>,
    ) -> Stmt {
        assert_eq!(fargs.len(), 2);
        let target = target.unwrap();
        let cond = fargs.remove(0).cast_to(Type::bool());

        // Add "EXPECTED FAIL" to the message because compiletest relies on it
        let msg =
            format!("EXPECTED FAIL: {}", utils::extract_const_message(&fargs.remove(0)).unwrap());

        let property_class = PropertyClass::ExpectFail;

        let loc = tcx.codegen_span_option(span);
        Stmt::block(
            vec![
                tcx.codegen_assert(cond, property_class, &msg, loc),
                Stmt::goto(tcx.current_fn().find_label(&target), loc),
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
        _assign_to: Place<'tcx>,
        target: Option<BasicBlock>,
        span: Option<Span>,
    ) -> Stmt {
        assert_eq!(fargs.len(), 1);
        let cond = fargs.remove(0).cast_to(Type::bool());
        let target = target.unwrap();
        let loc = tcx.codegen_span_option(span);

        Stmt::block(
            vec![Stmt::assume(cond, loc), Stmt::goto(tcx.current_fn().find_label(&target), loc)],
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
        _assign_to: Place<'tcx>,
        target: Option<BasicBlock>,
        span: Option<Span>,
    ) -> Stmt {
        assert_eq!(fargs.len(), 2);
        let cond = fargs.remove(0).cast_to(Type::bool());
        let msg = fargs.remove(0);
        let msg = utils::extract_const_message(&msg).unwrap();
        let target = target.unwrap();
        let caller_loc = tcx.codegen_caller_span(&span);

        // TODO: switch to tagging assertions via the property class once CBMC allows that:
        // https://github.com/diffblue/cbmc/issues/6692
        let (msg, reach_stmt) = if tcx.queries.get_check_assertion_reachability() {
            // Generate a unique ID for the assert
            let assert_id = tcx.next_check_id();
            // Add this ID as a prefix to the assert message so that it can be
            // easily paired with the reachability check
            let msg = GotocCtx::add_prefix_to_msg(&msg, &assert_id);
            let reach_msg = GotocCtx::reachability_check_message(&assert_id);
            // inject a reachability (cover) check to the current location
            (msg, tcx.codegen_cover_loc(&reach_msg, span))
        } else {
            (msg, Stmt::skip(caller_loc))
        };

        // Since `cond` might have side effects, assign it to a temporary
        // variable so that it's evaluated once, then assert and assume it
        let tmp = tcx.gen_temp_variable(cond.typ().clone(), caller_loc).to_expr();
        Stmt::block(
            vec![
                reach_stmt,
                Stmt::decl(tmp.clone(), Some(cond), caller_loc),
                tcx.codegen_assert(tmp.clone(), PropertyClass::DefaultAssertion, &msg, caller_loc),
                Stmt::assume(tmp, caller_loc),
                Stmt::goto(tcx.current_fn().find_label(&target), caller_loc),
            ],
            caller_loc,
        )
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
        assign_to: Place<'tcx>,
        target: Option<BasicBlock>,
        span: Option<Span>,
    ) -> Stmt {
        assert!(fargs.is_empty());
        let loc = tcx.codegen_span_option(span);
        let target = target.unwrap();
        let pt = tcx.place_ty(&assign_to);
        if pt.is_unit() {
            Stmt::goto(tcx.current_fn().find_label(&target), loc)
        } else {
            let pe =
                unwrap_or_return_codegen_unimplemented_stmt!(tcx, tcx.codegen_place(&assign_to))
                    .goto_expr;
            Stmt::block(
                vec![
                    pe.assign(tcx.codegen_ty(pt).nondet(), loc),
                    Stmt::goto(tcx.current_fn().find_label(&target), loc),
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
        _assign_to: Place<'tcx>,
        _target: Option<BasicBlock>,
        span: Option<Span>,
    ) -> Stmt {
        tcx.codegen_panic(span, fargs)
    }
}

struct PtrRead;

impl<'tcx> GotocHook<'tcx> for PtrRead {
    fn hook_applies(&self, tcx: TyCtxt<'tcx>, instance: Instance<'tcx>) -> bool {
        let name = with_no_trimmed_paths!(tcx.def_path_str(instance.def_id()));
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
        assign_to: Place<'tcx>,
        target: Option<BasicBlock>,
        span: Option<Span>,
    ) -> Stmt {
        let loc = tcx.codegen_span_option(span);
        let target = target.unwrap();
        let src = fargs.remove(0);
        Stmt::block(
            vec![
                unwrap_or_return_codegen_unimplemented_stmt!(tcx, tcx.codegen_place(&assign_to))
                    .goto_expr
                    .assign(src.dereference().with_location(loc), loc),
                Stmt::goto(tcx.current_fn().find_label(&target), loc),
            ],
            loc,
        )
    }
}

struct PtrWrite;

impl<'tcx> GotocHook<'tcx> for PtrWrite {
    fn hook_applies(&self, tcx: TyCtxt<'tcx>, instance: Instance<'tcx>) -> bool {
        let name = with_no_trimmed_paths!(tcx.def_path_str(instance.def_id()));
        name == "core::ptr::write"
            || name == "core::ptr::write_unaligned"
            || name == "std::ptr::write"
            || name == "std::ptr::write_unaligned"
    }

    fn handle(
        &self,
        tcx: &mut GotocCtx<'tcx>,
        _instance: Instance<'tcx>,
        mut fargs: Vec<Expr>,
        _assign_to: Place<'tcx>,
        target: Option<BasicBlock>,
        span: Option<Span>,
    ) -> Stmt {
        let loc = tcx.codegen_span_option(span);
        let target = target.unwrap();
        let dst = fargs.remove(0);
        let src = fargs.remove(0);
        Stmt::block(
            vec![
                dst.dereference().assign(src, loc).with_location(loc),
                Stmt::goto(tcx.current_fn().find_label(&target), loc),
            ],
            loc,
        )
    }
}

struct RustAlloc;
// Removing this hook causes regression failures.
// https://github.com/model-checking/kani/issues/1170
impl<'tcx> GotocHook<'tcx> for RustAlloc {
    fn hook_applies(&self, tcx: TyCtxt<'tcx>, instance: Instance<'tcx>) -> bool {
        let full_name = with_no_trimmed_paths!(tcx.def_path_str(instance.def_id()));
        full_name == "alloc::alloc::exchange_malloc"
    }

    fn handle(
        &self,
        tcx: &mut GotocCtx<'tcx>,
        instance: Instance<'tcx>,
        mut fargs: Vec<Expr>,
        assign_to: Place<'tcx>,
        target: Option<BasicBlock>,
        span: Option<Span>,
    ) -> Stmt {
        debug!(?instance, "Replace allocation");
        let loc = tcx.codegen_span_option(span);
        let target = target.unwrap();
        let size = fargs.remove(0);
        Stmt::block(
            vec![
                unwrap_or_return_codegen_unimplemented_stmt!(tcx, tcx.codegen_place(&assign_to))
                    .goto_expr
                    .assign(
                        BuiltinFn::Malloc
                            .call(vec![size], loc)
                            .cast_to(Type::unsigned_int(8).to_pointer()),
                        loc,
                    ),
                Stmt::goto(tcx.current_fn().find_label(&target), Location::none()),
            ],
            Location::none(),
        )
    }
}

struct SliceFromRawPart;

impl<'tcx> GotocHook<'tcx> for SliceFromRawPart {
    fn hook_applies(&self, tcx: TyCtxt<'tcx>, instance: Instance<'tcx>) -> bool {
        let name = with_no_trimmed_paths!(tcx.def_path_str(instance.def_id()));
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
        assign_to: Place<'tcx>,
        target: Option<BasicBlock>,
        span: Option<Span>,
    ) -> Stmt {
        let loc = tcx.codegen_span_option(span);
        let target = target.unwrap();
        let pt = tcx.codegen_ty(tcx.place_ty(&assign_to));
        let data = fargs.remove(0);
        let len = fargs.remove(0);
        let code = unwrap_or_return_codegen_unimplemented_stmt!(tcx, tcx.codegen_place(&assign_to))
            .goto_expr
            .assign(Expr::struct_expr_from_values(pt, vec![data, len], &tcx.symbol_table), loc)
            .with_location(loc);
        Stmt::block(vec![code, Stmt::goto(tcx.current_fn().find_label(&target), loc)], loc)
    }
}

pub fn fn_hooks<'tcx>() -> GotocHooks<'tcx> {
    GotocHooks {
        hooks: vec![
            Rc::new(Panic),
            Rc::new(Assume),
            Rc::new(Assert),
            Rc::new(ExpectFail),
            Rc::new(Nondet),
            Rc::new(PtrRead),
            Rc::new(PtrWrite),
            Rc::new(RustAlloc),
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
