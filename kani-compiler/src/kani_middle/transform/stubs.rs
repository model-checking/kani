// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! This module contains code related to the MIR-to-MIR pass that performs the
//! stubbing of functions and methods.
use crate::kani_middle::attributes::matches_diagnostic;
use crate::kani_middle::codegen_units::Stubs;
use crate::kani_middle::stubbing::validate_instance;
use crate::kani_middle::transform::body::{CheckType, MutableBody, SourceInstruction};
use crate::kani_middle::transform::{TransformPass, TransformationType};
use crate::kani_queries::QueryDb;
use rustc_middle::ty::TyCtxt;
use stable_mir::mir::mono::Instance;
use stable_mir::mir::{
    BinOp, Body, Constant, Operand, Place, Rvalue, Statement, StatementKind, RETURN_LOCAL,
};
use stable_mir::target::MachineInfo;
use stable_mir::ty::{Const, RigidTy, TyKind};
use std::fmt::Debug;
use strum_macros::AsRefStr;
use tracing::trace;

/// Replace the body of a function that is stubbed by the other.
#[derive(Debug)]
pub struct FnStubPass {
    pub stubs: Stubs,
}

impl TransformPass for FnStubPass {
    fn transformation_type() -> TransformationType
    where
        Self: Sized,
    {
        TransformationType::UserTransformation
    }

    fn is_enabled(&self, _query_db: &QueryDb) -> bool
    where
        Self: Sized,
    {
        true
    }

    /// Transform the function body by replacing it with the stub body.
    fn transform(&self, tcx: TyCtxt, body: Body, instance: Instance) -> (bool, Body) {
        trace!(function=?instance.name(), "transform");
        let ty = instance.ty();
        if let TyKind::RigidTy(RigidTy::FnDef(fn_def, args)) = ty.kind() {
            if let Some(replace) = self.stubs.get(&fn_def) {
                let new_instance = Instance::resolve(*replace, &args).unwrap();
                if validate_instance(tcx, new_instance) {
                    return (true, new_instance.body().unwrap());
                }
            }
        }
        (false, body)
    }
}

impl FnStubPass {}
