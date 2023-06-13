// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This module contains various codegen hooks for functions that require special handling.
//!
//! E.g.: Functions in the Kani library that generate assumptions or symbolic variables.
//!
//! It would be too nasty if we spread around these sort of undocumented hooks in place, so
//! this module addresses this issue.

use crate::codegen_boogie::BoogieCtx;
use boogie_ast::boogie_program::{Expr, Stmt};
use rustc_middle::mir::{BasicBlock, Place};
use rustc_middle::ty::{Instance, TyCtxt};
use rustc_span::Span;
use std::rc::Rc;
use tracing::debug;

pub trait BoogieHook<'tcx> {
    /// if the hook applies, it means the codegen would do something special to it
    fn hook_applies(&self, tcx: TyCtxt<'tcx>, instance: Instance<'tcx>) -> bool;
    /// the handler for codegen
    fn handle(
        &self,
        bcx: &BoogieCtx<'tcx>,
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

struct Assume;
impl<'tcx> BoogieHook<'tcx> for Assume {
    fn hook_applies(&self, tcx: TyCtxt<'tcx>, instance: Instance<'tcx>) -> bool {
        matches_function(tcx, instance, "KaniAssume")
    }

    fn handle(
        &self,
        _bcx: &BoogieCtx<'tcx>,
        _instance: Instance<'tcx>,
        mut fargs: Vec<Expr>,
        _assign_to: Place<'tcx>,
        _target: Option<BasicBlock>,
        _span: Option<Span>,
    ) -> Stmt {
        assert_eq!(fargs.len(), 1);
        let cond = fargs.remove(0);
        // handle location

        Stmt::Block {
            statements: vec![
                Stmt::Assume { condition: cond },
                // handle target
            ],
        }
    }
}

struct Assert;
impl<'tcx> BoogieHook<'tcx> for Assert {
    fn hook_applies(&self, tcx: TyCtxt<'tcx>, instance: Instance<'tcx>) -> bool {
        matches_function(tcx, instance, "KaniAssert")
    }

    fn handle(
        &self,
        _bcx: &BoogieCtx<'tcx>,
        _instance: Instance<'tcx>,
        mut fargs: Vec<Expr>,
        _assign_to: Place<'tcx>,
        _target: Option<BasicBlock>,
        _span: Option<Span>,
    ) -> Stmt {
        // TODO: ignoring the `'static str` argument for now
        assert_eq!(fargs.len(), 1); // 2);
        let cond = fargs.remove(0);
        // TODO: handle message
        // TODO: handle location

        Stmt::Block {
            statements: vec![
                Stmt::Assert { condition: cond },
                // handle target
            ],
        }
    }
}

pub fn fn_hooks<'tcx>() -> BoogieHooks<'tcx> {
    BoogieHooks { hooks: vec![Rc::new(Assume), Rc::new(Assert)] }
}

pub struct BoogieHooks<'tcx> {
    hooks: Vec<Rc<dyn BoogieHook<'tcx> + 'tcx>>,
}

impl<'tcx> BoogieHooks<'tcx> {
    pub fn hook_applies(
        &self,
        tcx: TyCtxt<'tcx>,
        instance: Instance<'tcx>,
    ) -> Option<Rc<dyn BoogieHook<'tcx> + 'tcx>> {
        for h in &self.hooks {
            if h.hook_applies(tcx, instance) {
                return Some(h.clone());
            }
        }
        None
    }
}
