// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! This module contains code related to the MIR-to-MIR pass that performs the
//! stubbing of functions and methods.
use crate::kani_middle::attributes::matches_diagnostic;
use crate::kani_middle::codegen_units::Stubs;
use crate::kani_middle::stubbing::validate_stub;
use crate::kani_middle::transform::body::{CheckType, MutableBody, SourceInstruction};
use crate::kani_middle::transform::{TransformPass, TransformationType};
use crate::kani_queries::QueryDb;
use itertools::Itertools;
use rustc_middle::ty::TyCtxt;
use stable_mir::mir::mono::Instance;
use stable_mir::mir::{
    BinOp, Body, Constant, Operand, Place, Rvalue, Statement, StatementKind, TerminatorKind,
    RETURN_LOCAL,
};
use stable_mir::target::MachineInfo;
use stable_mir::ty::{Abi, Const as MirConst, FnDef, RigidTy, TyKind};
use stable_mir::CrateDef;
use std::collections::HashMap;
use std::fmt::Debug;
use strum_macros::AsRefStr;
use tracing::{debug, trace};

/// Replace the body of a function that is stubbed by the other.
///
/// This pass will replace the entire body, and it should only be applied to stubs
/// that have a body.
#[derive(Debug)]
pub struct FnStubPass {
    stubs: Stubs,
}

impl TransformPass for FnStubPass {
    fn transformation_type() -> TransformationType
    where
        Self: Sized,
    {
        TransformationType::Stubbing
    }

    fn is_enabled(&self, query_db: &QueryDb) -> bool
    where
        Self: Sized,
    {
        query_db.args().stubbing_enabled && !self.stubs.is_empty()
    }

    /// Transform the function body by replacing it with the stub body.
    fn transform(&self, tcx: TyCtxt, body: Body, instance: Instance) -> (bool, Body) {
        trace!(function=?instance.name(), "transform");
        let ty = instance.ty();
        if let TyKind::RigidTy(RigidTy::FnDef(fn_def, args)) = ty.kind() {
            if let Some(replace) = self.stubs.get(&fn_def) {
                let new_instance = Instance::resolve(*replace, &args).unwrap();
                if validate_stub(tcx, new_instance) {
                    debug!(from=?instance.name(), to=?new_instance.name(), "FnStubPass::transform");
                    return (true, new_instance.body().unwrap());
                }
            }
        }
        (false, body)
    }
}

impl FnStubPass {
    /// Build the pass with non-extern function stubs.
    pub fn new(all_stubs: &Stubs) -> FnStubPass {
        let stubs = all_stubs
            .iter()
            .filter_map(|(from, to)| (has_body(*from) && has_body(*to)).then_some((*from, *to)))
            .collect::<HashMap<_, _>>();
        FnStubPass { stubs }
    }
}

/// Replace the body of a function that is stubbed by the other.
///
/// This pass will replace the function call, since one of the functions do not have a body to
/// replace.
#[derive(Debug)]
pub struct ExternFnStubPass {
    pub stubs: Stubs,
}

impl TransformPass for ExternFnStubPass {
    fn transformation_type() -> TransformationType
    where
        Self: Sized,
    {
        TransformationType::Stubbing
    }

    fn is_enabled(&self, query_db: &QueryDb) -> bool
    where
        Self: Sized,
    {
        query_db.args().stubbing_enabled && !self.stubs.is_empty()
    }

    /// Search for calls to extern functions that should be stubbed.
    fn transform(&self, tcx: TyCtxt, mut body: Body, instance: Instance) -> (bool, Body) {
        trace!(function=?instance.name(), "transform");
        let mut changed = false;
        let locals = body.locals().to_vec();
        for bb in body.blocks.iter_mut() {
            let TerminatorKind::Call { func, .. } = &mut bb.terminator.kind else { continue };
            if let TyKind::RigidTy(RigidTy::FnDef(def, args)) = func.ty(&locals).unwrap().kind() {
                if let Some(replace) = self.stubs.get(&def) {
                    debug!(func=?instance.name(), orig=?def.name(), replace=?replace.name(),
                        "ExternFnStubPass::transform");
                    let instance = Instance::resolve(*replace, &args).unwrap();
                    let literal = MirConst::try_new_zero_sized(instance.ty()).unwrap();
                    let span = bb.terminator.span;
                    let new_func = Constant { span, user_ty: None, literal };
                    *func = Operand::Constant(new_func);
                    changed = true;
                }
            }
        }
        (changed, body)
    }
}

impl ExternFnStubPass {
    /// Build the pass with the extern function stubs.
    ///
    /// This will cover any case where the stub doesn't have a body.
    pub fn new(all_stubs: &Stubs) -> ExternFnStubPass {
        let stubs = all_stubs
            .iter()
            .filter_map(|(from, to)| (!has_body(*from) || !has_body(*to)).then_some((*from, *to)))
            .collect::<HashMap<_, _>>();
        ExternFnStubPass { stubs }
    }
}

fn has_body(def: FnDef) -> bool {
    def.body().is_some()
}
