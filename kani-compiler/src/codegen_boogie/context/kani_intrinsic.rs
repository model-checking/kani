// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This module is for handling Kani intrinsics, i.e. items whose implementation
//! is defined in the Kani compiler. These items are defined in the `kani`
//! library and are annotated with a `rustc_diagnostic_item`.

use super::boogie_ctx::FunctionCtx;

use boogie_ast::boogie_program::{Expr, Stmt};
use rustc_middle::mir::{BasicBlock, Operand, Place};
use rustc_middle::ty::{Instance, TyCtxt};
use rustc_span::Span;
use std::str::FromStr;
use strum::VariantNames;
use strum_macros::{EnumString, EnumVariantNames};
use tracing::debug;

// TODO: move this enum up to `kani_middle`
#[derive(Debug, Clone, PartialEq, Eq, EnumString, EnumVariantNames)]
#[allow(clippy::enum_variant_names)]
pub enum KaniIntrinsic {
    /// Kani assert statement (`kani::assert`)
    KaniAssert,

    /// Kani assume statement (`kani::assume`)
    KaniAssume,

    /// Kani unbounded array (`kani::array::any_array`)
    KaniAnyArray,

    /// `kani::array::Array::len`
    KaniAnyArrayLen,

    /// `Index<usize> for kani::array::Array`
    KaniAnyArrayIndex,

    /// `IndexMut<usize> for kani::array::Array`
    KaniAnyArrayIndexMut,
}

/// If provided function is a Kani intrinsic (e.g. assert, assume, cover), returns it
// TODO: move this function up to `kani_middle` along with the enum
pub fn get_kani_intrinsic<'tcx>(
    tcx: TyCtxt<'tcx>,
    instance: Instance<'tcx>,
) -> Option<KaniIntrinsic> {
    for intrinsic in KaniIntrinsic::VARIANTS {
        let attr_sym = rustc_span::symbol::Symbol::intern(intrinsic);
        if let Some(attr_id) = tcx.all_diagnostic_items(()).name_to_id.get(&attr_sym) {
            if instance.def.def_id() == *attr_id {
                debug!("matched: {:?} {:?}", attr_id, attr_sym);
                return Some(KaniIntrinsic::from_str(intrinsic).unwrap());
            }
        }
    }
    None
}

impl<'a, 'tcx> FunctionCtx<'a, 'tcx> {
    pub fn codegen_kani_intrinsic(
        &mut self,
        intrinsic: KaniIntrinsic,
        instance: Instance<'tcx>,
        args: &[Operand<'tcx>],
        assign_to: Place<'tcx>,
        target: Option<BasicBlock>,
        span: Option<Span>,
    ) -> Stmt {
        match intrinsic {
            KaniIntrinsic::KaniAssert => {
                self.codegen_kani_assert(instance, args, assign_to, target, span)
            }
            KaniIntrinsic::KaniAssume => {
                self.codegen_kani_assume(instance, args, assign_to, target, span)
            }
            KaniIntrinsic::KaniAnyArray => {
                self.codegen_kani_any_array(instance, args, assign_to, target, span)
            }
            KaniIntrinsic::KaniAnyArrayLen => {
                self.codegen_kani_any_array_len(instance, args, assign_to, target, span)
            }
            KaniIntrinsic::KaniAnyArrayIndex => {
                self.codegen_kani_any_array_index(instance, args, assign_to, target, span)
            }
            KaniIntrinsic::KaniAnyArrayIndexMut => {
                self.codegen_kani_any_array_index_mut(instance, args, assign_to, target, span)
            }
        }
    }

    fn codegen_kani_assert(
        &self,
        _instance: Instance<'tcx>,
        args: &[Operand<'tcx>],
        _assign_to: Place<'tcx>,
        _target: Option<BasicBlock>,
        _span: Option<Span>,
    ) -> Stmt {
        // TODO: ignoring the `'static str` argument for now
        assert_eq!(args.len(), 2);
        let cond = self.codegen_operand(&args[0]);
        // TODO: handle message
        // TODO: handle location

        Stmt::block(vec![
            Stmt::Assert { condition: cond },
            // TODO: handle target
        ])
    }

    fn codegen_kani_assume(
        &self,
        _instance: Instance<'tcx>,
        args: &[Operand<'tcx>],
        _assign_to: Place<'tcx>,
        _target: Option<BasicBlock>,
        _span: Option<Span>,
    ) -> Stmt {
        assert_eq!(args.len(), 1);
        let cond = self.codegen_operand(&args[0]);
        // TODO: handle location

        Stmt::block(vec![
            Stmt::Assume { condition: cond },
            // TODO: handle target
        ])
    }

    fn codegen_kani_any_raw(
        &self,
        _instance: Instance<'tcx>,
        args: &[Operand<'tcx>],
        assign_to: Place<'tcx>,
        target: Option<BasicBlock>,
        _span: Option<Span>,
    ) -> Stmt {
        assert!(args.is_empty());

        let place = self.codegen_place(&assign_to);

        let symbol = if let Expr::Symbol { name } = place {
            name
        } else {
            panic!("expecting place to be a symbol and not {place:?}.")
        };

        Stmt::block(vec![
            Stmt::Havoc { name: symbol },
            Stmt::Goto { label: format!("{:?}", target.unwrap()) },
        ])
    }

    fn codegen_kani_any_array(
        &self,
        instance: Instance<'tcx>,
        args: &[Operand<'tcx>],
        assign_to: Place<'tcx>,
        target: Option<BasicBlock>,
        span: Option<Span>,
    ) -> Stmt {
        assert!(args.is_empty());

        self.codegen_kani_any_raw(instance, args, assign_to, target, span)
    }

    fn codegen_kani_any_array_len(
        &self,
        _instance: Instance<'tcx>,
        args: &[Operand<'tcx>],
        assign_to: Place<'tcx>,
        _target: Option<BasicBlock>,
        _span: Option<Span>,
    ) -> Stmt {
        assert_eq!(args.len(), 1);
        debug!(?args, "codegen_kani_any_array_len");

        let self_ref = &args[0];
        let expr = self
            .operand_to_expr(self_ref)
            .expect("expecting operand to be a ref to an existing expression");
        let len = Expr::Field { base: Box::new(expr.clone()), field: String::from("len") };

        let place = self.codegen_place(&assign_to);

        let Expr::Symbol { name } = place else { panic!("expecting place to be a symbol") };

        Stmt::Assignment { target: name, value: len }
    }

    fn codegen_kani_any_array_index(
        &self,
        _instance: Instance<'tcx>,
        args: &[Operand<'tcx>],
        assign_to: Place<'tcx>,
        _target: Option<BasicBlock>,
        _span: Option<Span>,
    ) -> Stmt {
        assert_eq!(args.len(), 2);
        debug!(?args, "codegen_kani_any_array_index");

        let self_ref = &args[0];
        let expr = self
            .operand_to_expr(self_ref)
            .expect("expecting operand to be a ref to an existing expression");
        let map = Expr::Field { base: Box::new(expr.clone()), field: String::from("data") };

        let index = self.codegen_operand(&args[1]);
        let index_expr = Expr::Select { base: Box::new(map), key: Box::new(index) };

        let place = self.codegen_place(&assign_to);

        let Expr::Symbol { name } = place else { panic!("expecting place to be a symbol") };

        Stmt::Assignment { target: name, value: index_expr }
    }

    fn codegen_kani_any_array_index_mut(
        &mut self,
        _instance: Instance<'tcx>,
        args: &[Operand<'tcx>],
        assign_to: Place<'tcx>,
        _target: Option<BasicBlock>,
        _span: Option<Span>,
    ) -> Stmt {
        assert_eq!(args.len(), 2);
        debug!(?args, "codegen_kani_any_array_index_mut");

        let mut_self_ref = &args[0];
        let expr = self
            .operand_to_expr(mut_self_ref)
            .expect("expecting operand to be a ref to an existing expression");
        let map = Expr::Field { base: Box::new(expr.clone()), field: String::from("data") };

        let index = self.codegen_operand(&args[1]);

        // TODO: change `Stmt::Assignment` to be in terms of a symbol not a
        // string to avoid the hacky code below
        let index_expr = Expr::Select { base: Box::new(map), key: Box::new(index) };
        self.ref_to_expr.insert(assign_to, index_expr);
        Stmt::Skip
    }

    fn operand_to_expr(&self, operand: &Operand<'tcx>) -> Option<&Expr> {
        let Operand::Move(place) = operand else {
            return None;
        };
        self.ref_to_expr.get(place)
    }
}
