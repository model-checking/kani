// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This module contains various codegen hooks for functions that require special handling.
//!
//! E.g.: Functions in the Kani library that generate assumptions or symbolic variables.
//!
//! It would be too nasty if we spread around these sort of undocumented hooks in place, so
//! this module addresses this issue.

use crate::codegen_cprover_gotoc::codegen::{bb_label, PropertyClass};
use crate::codegen_cprover_gotoc::GotocCtx;
use crate::unwrap_or_return_codegen_unimplemented_stmt;
use cbmc::goto_program::{BuiltinFn, Expr, Location, Stmt, Type};
use rustc_middle::ty::TyCtxt;
use rustc_smir::rustc_internal;
use stable_mir::mir::mono::Instance;
use stable_mir::mir::{BasicBlockIdx, Place};
use stable_mir::{ty::Span, CrateDef};
use std::rc::Rc;
use tracing::debug;

pub trait GotocHook {
    /// if the hook applies, it means the codegen would do something special to it
    fn hook_applies(&self, tcx: TyCtxt, instance: Instance) -> bool;
    /// the handler for codegen
    fn handle(
        &self,
        gcx: &mut GotocCtx,
        instance: Instance,
        fargs: Vec<Expr>,
        assign_to: Place,
        target: Option<BasicBlockIdx>,
        span: Span,
    ) -> Stmt;
}

fn matches_function(tcx: TyCtxt, instance: Instance, attr_name: &str) -> bool {
    let attr_sym = rustc_span::symbol::Symbol::intern(attr_name);
    if let Some(attr_id) = tcx.all_diagnostic_items(()).name_to_id.get(&attr_sym) {
        if rustc_internal::internal(instance.def.def_id()) == *attr_id {
            debug!("matched: {:?} {:?}", attr_id, attr_sym);
            return true;
        }
    }
    false
}

/// A hook for Kani's `cover` function (declared in `library/kani/src/lib.rs`).
/// The function takes two arguments: a condition expression (bool) and a
/// message (&'static str).
/// The hook codegens the function as a cover property that checks whether the
/// condition is satisfiable. Unlike assertions, cover properties currently do
/// not have an impact on verification success or failure. See
/// <https://github.com/model-checking/kani/blob/main/rfc/src/rfcs/0003-cover-statement.md>
/// for more details.
struct Cover;
impl GotocHook for Cover {
    fn hook_applies(&self, tcx: TyCtxt, instance: Instance) -> bool {
        matches_function(tcx, instance, "KaniCover")
    }

    fn handle(
        &self,
        gcx: &mut GotocCtx,
        _instance: Instance,
        mut fargs: Vec<Expr>,
        _assign_to: Place,
        target: Option<BasicBlockIdx>,
        span: Span,
    ) -> Stmt {
        assert_eq!(fargs.len(), 2);
        let cond = fargs.remove(0).cast_to(Type::bool());
        let msg = fargs.remove(0);
        let msg = gcx.extract_const_message(&msg).unwrap();
        let target = target.unwrap();
        let caller_loc = gcx.codegen_caller_span_stable(span);

        let (msg, reach_stmt) = gcx.codegen_reachability_check(msg, span);

        Stmt::block(
            vec![
                reach_stmt,
                gcx.codegen_cover(cond, &msg, span),
                Stmt::goto(bb_label(target), caller_loc),
            ],
            caller_loc,
        )
    }
}

struct Assume;
impl GotocHook for Assume {
    fn hook_applies(&self, tcx: TyCtxt, instance: Instance) -> bool {
        matches_function(tcx, instance, "KaniAssume")
    }

    fn handle(
        &self,
        gcx: &mut GotocCtx,
        _instance: Instance,
        mut fargs: Vec<Expr>,
        _assign_to: Place,
        target: Option<BasicBlockIdx>,
        span: Span,
    ) -> Stmt {
        assert_eq!(fargs.len(), 1);
        let cond = fargs.remove(0).cast_to(Type::bool());
        let target = target.unwrap();
        let loc = gcx.codegen_span_stable(span);

        Stmt::block(vec![gcx.codegen_assume(cond, loc), Stmt::goto(bb_label(target), loc)], loc)
    }
}

struct Assert;
impl GotocHook for Assert {
    fn hook_applies(&self, tcx: TyCtxt, instance: Instance) -> bool {
        matches_function(tcx, instance, "KaniAssert")
    }

    fn handle(
        &self,
        gcx: &mut GotocCtx,
        _instance: Instance,
        mut fargs: Vec<Expr>,
        _assign_to: Place,
        target: Option<BasicBlockIdx>,
        span: Span,
    ) -> Stmt {
        assert_eq!(fargs.len(), 2);
        let cond = fargs.remove(0).cast_to(Type::bool());
        let msg = fargs.remove(0);
        let msg = gcx.extract_const_message(&msg).unwrap();
        let target = target.unwrap();
        let caller_loc = gcx.codegen_caller_span_stable(span);

        let (msg, reach_stmt) = gcx.codegen_reachability_check(msg, span);

        // Since `cond` might have side effects, assign it to a temporary
        // variable so that it's evaluated once, then assert and assume it
        // TODO: I don't think `cond` can have side effects, this is MIR, it's going to be temps
        let (tmp, decl) = gcx.decl_temp_variable(cond.typ().clone(), Some(cond), caller_loc);
        Stmt::block(
            vec![
                reach_stmt,
                decl,
                gcx.codegen_assert_assume(tmp, PropertyClass::Assertion, &msg, caller_loc),
                Stmt::goto(bb_label(target), caller_loc),
            ],
            caller_loc,
        )
    }
}

struct Nondet;

impl GotocHook for Nondet {
    fn hook_applies(&self, tcx: TyCtxt, instance: Instance) -> bool {
        matches_function(tcx, instance, "KaniAnyRaw")
    }

    fn handle(
        &self,
        gcx: &mut GotocCtx,
        _instance: Instance,
        fargs: Vec<Expr>,
        assign_to: Place,
        target: Option<BasicBlockIdx>,
        span: Span,
    ) -> Stmt {
        assert!(fargs.is_empty());
        let loc = gcx.codegen_span_stable(span);
        let target = target.unwrap();
        let pt = gcx.place_ty_stable(&assign_to);
        if pt.kind().is_unit() {
            Stmt::goto(bb_label(target), loc)
        } else {
            let pe = unwrap_or_return_codegen_unimplemented_stmt!(
                gcx,
                gcx.codegen_place_stable(&assign_to)
            )
            .goto_expr;
            Stmt::block(
                vec![
                    pe.assign(gcx.codegen_ty_stable(pt).nondet(), loc),
                    Stmt::goto(bb_label(target), loc),
                ],
                loc,
            )
        }
    }
}

struct Panic;

impl GotocHook for Panic {
    fn hook_applies(&self, tcx: TyCtxt, instance: Instance) -> bool {
        let def_id = rustc_internal::internal(instance.def.def_id());
        Some(def_id) == tcx.lang_items().panic_fn()
            || tcx.has_attr(def_id, rustc_span::sym::rustc_const_panic_str)
            || Some(def_id) == tcx.lang_items().panic_fmt()
            || Some(def_id) == tcx.lang_items().begin_panic_fn()
            || matches_function(tcx, instance, "KaniPanic")
    }

    fn handle(
        &self,
        gcx: &mut GotocCtx,
        _instance: Instance,
        fargs: Vec<Expr>,
        _assign_to: Place,
        _target: Option<BasicBlockIdx>,
        span: Span,
    ) -> Stmt {
        gcx.codegen_panic(span, fargs)
    }
}

struct RustAlloc;
// Removing this hook causes regression failures.
// https://github.com/model-checking/kani/issues/1170
impl GotocHook for RustAlloc {
    fn hook_applies(&self, _tcx: TyCtxt, instance: Instance) -> bool {
        let full_name = instance.name();
        full_name == "alloc::alloc::exchange_malloc"
    }

    fn handle(
        &self,
        gcx: &mut GotocCtx,
        instance: Instance,
        mut fargs: Vec<Expr>,
        assign_to: Place,
        target: Option<BasicBlockIdx>,
        span: Span,
    ) -> Stmt {
        debug!(?instance, "Replace allocation");
        let loc = gcx.codegen_span_stable(span);
        let target = target.unwrap();
        let size = fargs.remove(0);
        Stmt::block(
            vec![
                unwrap_or_return_codegen_unimplemented_stmt!(
                    gcx,
                    gcx.codegen_place_stable(&assign_to)
                )
                .goto_expr
                .assign(
                    BuiltinFn::Malloc
                        .call(vec![size], loc)
                        .cast_to(Type::unsigned_int(8).to_pointer()),
                    loc,
                ),
                Stmt::goto(bb_label(target), Location::none()),
            ],
            Location::none(),
        )
    }
}

/// This hook intercepts calls to `memcmp` and skips CBMC's pointer checks if the number of bytes to be compared is zero.
/// See issue <https://github.com/model-checking/kani/issues/1489>
///
/// This compiles `memcmp(first, second, count)` to:
/// ```c
/// count_var = count;
/// first_var = first;
/// second_var = second;
/// count_var == 0 && first_var != NULL && second_var != NULL ? 0 : memcmp(first_var, second_var, count_var)
/// ```
pub struct MemCmp;

impl GotocHook for MemCmp {
    fn hook_applies(&self, _tcx: TyCtxt, instance: Instance) -> bool {
        let name = instance.name();
        name == "core::slice::cmp::memcmp" || name == "std::slice::cmp::memcmp"
    }

    fn handle(
        &self,
        gcx: &mut GotocCtx,
        instance: Instance,
        mut fargs: Vec<Expr>,
        assign_to: Place,
        target: Option<BasicBlockIdx>,
        span: Span,
    ) -> Stmt {
        let loc = gcx.codegen_span_stable(span);
        let target = target.unwrap();
        let first = fargs.remove(0);
        let second = fargs.remove(0);
        let count = fargs.remove(0);
        let (count_var, count_decl) = gcx.decl_temp_variable(count.typ().clone(), Some(count), loc);
        let (first_var, first_decl) = gcx.decl_temp_variable(first.typ().clone(), Some(first), loc);
        let (second_var, second_decl) =
            gcx.decl_temp_variable(second.typ().clone(), Some(second), loc);
        let is_count_zero = count_var.clone().is_zero();
        // We have to ensure that the pointers are valid even if we're comparing zero bytes.
        // According to Rust's current definition (see https://github.com/model-checking/kani/issues/1489),
        // this means they have to be non-null and aligned.
        // But alignment is automatically satisfied because `memcmp` takes `*const u8` pointers.
        let is_first_ok = first_var.clone().is_nonnull();
        let is_second_ok = second_var.clone().is_nonnull();
        let should_skip_pointer_checks = is_count_zero.and(is_first_ok).and(is_second_ok);
        let place_expr =
            unwrap_or_return_codegen_unimplemented_stmt!(gcx, gcx.codegen_place_stable(&assign_to))
                .goto_expr;
        let rhs = should_skip_pointer_checks.ternary(
            Expr::int_constant(0, place_expr.typ().clone()), // zero bytes are always equal (as long as pointers are nonnull and aligned)
            gcx.codegen_func_expr(instance, Some(span))
                .call(vec![first_var, second_var, count_var]),
        );
        let code = place_expr.assign(rhs, loc).with_location(loc);
        Stmt::block(
            vec![count_decl, first_decl, second_decl, code, Stmt::goto(bb_label(target), loc)],
            loc,
        )
    }
}

/// A builtin that is essentially a C-style dereference operation, creating an
/// unsafe shallow copy. Importantly either this copy or the original needs to
/// be `mem::forget`en or a double-free will occur.
///
/// Takes in a `&T` reference and returns a `T` (like clone would but without
/// cloning). Breaks ownership rules and is only used in the context of function
/// contracts where we can structurally guarantee the use is safe.
struct UntrackedDeref;

impl GotocHook for UntrackedDeref {
    fn hook_applies(&self, tcx: TyCtxt, instance: Instance) -> bool {
        matches_function(tcx, instance, "KaniUntrackedDeref")
    }

    fn handle(
        &self,
        gcx: &mut GotocCtx,
        _instance: Instance,
        mut fargs: Vec<Expr>,
        assign_to: Place,
        _target: Option<BasicBlockIdx>,
        span: Span,
    ) -> Stmt {
        assert_eq!(
            fargs.len(),
            1,
            "Invariant broken. `untracked_deref` should only be given one argument. \
            This function should only be called from code generated by kani macros, \
            as such this is likely a code-generation error."
        );
        let loc = gcx.codegen_span_stable(span);
        Stmt::block(
            vec![Stmt::assign(
                unwrap_or_return_codegen_unimplemented_stmt!(
                    gcx,
                    gcx.codegen_place_stable(&assign_to)
                )
                .goto_expr,
                fargs.pop().unwrap().dereference(),
                loc,
            )],
            loc,
        )
    }
}

pub fn fn_hooks() -> GotocHooks {
    GotocHooks {
        hooks: vec![
            Rc::new(Panic),
            Rc::new(Assume),
            Rc::new(Assert),
            Rc::new(Cover),
            Rc::new(Nondet),
            Rc::new(RustAlloc),
            Rc::new(MemCmp),
            Rc::new(UntrackedDeref),
        ],
    }
}

pub struct GotocHooks {
    hooks: Vec<Rc<dyn GotocHook>>,
}

impl GotocHooks {
    pub fn hook_applies(&self, tcx: TyCtxt, instance: Instance) -> Option<Rc<dyn GotocHook>> {
        for h in &self.hooks {
            if h.hook_applies(tcx, instance) {
                return Some(h.clone());
            }
        }
        None
    }
}
