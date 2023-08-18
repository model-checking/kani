// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This module is for handling Kani intrinsics, i.e. items whose implementation
//! is defined in the Kani compiler. These items are defined in the `kani`
//! library and are annotated with a `rustc_diagnostic_item`.

use crate::codegen_boogie::BoogieCtx;

use boogie_ast::boogie_program::{Expr, Stmt};
use rustc_middle::mir::{BasicBlock, Place};
use rustc_middle::ty::Instance;
use rustc_span::Span;
use std::str::FromStr;
use strum::VariantNames;
use strum_macros::{EnumString, EnumVariantNames};
use tracing::debug;

// TODO: move this enum up to `kani_middle`
#[derive(Debug, Clone, PartialEq, Eq, EnumString, EnumVariantNames)]
pub enum KaniIntrinsic {
    /// Kani assert statement (`kani::assert`)
    KaniAssert,

    /// Kani assume statement (`kani::assume`)
    KaniAssume,
}

impl<'tcx> BoogieCtx<'tcx> {
    /// If provided function is a Kani intrinsic (e.g. assert, assume, cover), returns it
    // TODO: move this function up to `kani_middle` along with the enum
    pub fn kani_intrinsic(&self, instance: Instance<'tcx>) -> Option<KaniIntrinsic> {
        let intrinsics = KaniIntrinsic::VARIANTS;
        for intrinsic in intrinsics {
            let attr_sym = rustc_span::symbol::Symbol::intern(intrinsic);
            if let Some(attr_id) = self.tcx.all_diagnostic_items(()).name_to_id.get(&attr_sym) {
                if instance.def.def_id() == *attr_id {
                    debug!("matched: {:?} {:?}", attr_id, attr_sym);
                    return Some(KaniIntrinsic::from_str(intrinsic).unwrap());
                }
            }
        }
        None
    }

    pub fn codegen_kani_intrinsic(
        &self,
        intrinsic: KaniIntrinsic,
        instance: Instance<'tcx>,
        fargs: Vec<Expr>,
        assign_to: Place<'tcx>,
        target: Option<BasicBlock>,
        span: Option<Span>,
    ) -> Stmt {
        match intrinsic {
            KaniIntrinsic::KaniAssert => {
                self.codegen_kani_assert(instance, fargs, assign_to, target, span)
            }
            KaniIntrinsic::KaniAssume => {
                self.codegen_kani_assume(instance, fargs, assign_to, target, span)
            }
        }
    }

    pub fn codegen_kani_assert(
        &self,
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
                // TODO: handle target
            ],
        }
    }

    pub fn codegen_kani_assume(
        &self,
        _instance: Instance<'tcx>,
        mut fargs: Vec<Expr>,
        _assign_to: Place<'tcx>,
        _target: Option<BasicBlock>,
        _span: Option<Span>,
    ) -> Stmt {
        assert_eq!(fargs.len(), 1);
        let cond = fargs.remove(0);
        // TODO: handle location

        Stmt::Block {
            statements: vec![
                Stmt::Assume { condition: cond },
                // TODO: handle target
            ],
        }
    }
}
