// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
//! This module contains the visitor responsible for collecting initial analysis targets for delayed
//! UB instrumentation.

use crate::{
    intrinsics::Intrinsic,
    kani_middle::transform::check_uninit::ty_layout::tys_layout_equal_to_size,
};
use stable_mir::{
    mir::{
        Body, CastKind, LocalDecl, MirVisitor, NonDivergingIntrinsic, Operand, Place, Rvalue,
        Statement, StatementKind, Terminator, TerminatorKind,
        alloc::GlobalAlloc,
        mono::{Instance, InstanceKind, StaticDef},
        visit::Location,
    },
    ty::{ConstantKind, RigidTy, TyKind},
};

/// Pointer, write through which might trigger delayed UB.
pub enum AnalysisTarget {
    Place(Place),
    Static(StaticDef),
}

/// Visitor that finds initial analysis targets for delayed UB instrumentation. For our purposes,
/// analysis targets are *pointers* to places reading and writing from which should be tracked.
pub struct InitialTargetVisitor {
    body: Body,
    targets: Vec<AnalysisTarget>,
}

impl InitialTargetVisitor {
    pub fn new(body: Body) -> Self {
        Self { body, targets: vec![] }
    }

    pub fn into_targets(self) -> Vec<AnalysisTarget> {
        self.targets
    }

    pub fn push_operand(&mut self, operand: &Operand) {
        match operand {
            Operand::Copy(place) | Operand::Move(place) => {
                self.targets.push(AnalysisTarget::Place(place.clone()));
            }
            Operand::Constant(constant) => {
                // Extract the static from the constant.
                if let ConstantKind::Allocated(allocation) = constant.const_.kind() {
                    for (_, prov) in &allocation.provenance.ptrs {
                        if let GlobalAlloc::Static(static_def) = GlobalAlloc::from(prov.0) {
                            self.targets.push(AnalysisTarget::Static(static_def));
                        };
                    }
                }
            }
        }
    }
}

/// We implement MirVisitor to facilitate target finding, we look for:
/// - pointer casts where pointees have different padding;
/// - calls to `copy`-like intrinsics.
impl MirVisitor for InitialTargetVisitor {
    fn visit_rvalue(&mut self, rvalue: &Rvalue, location: Location) {
        if let Rvalue::Cast(kind, operand, ty) = rvalue {
            let operand_ty = operand.ty(self.body.locals()).unwrap();
            match kind {
                CastKind::Transmute | CastKind::PtrToPtr => {
                    let operand_ty_kind = operand_ty.kind();
                    let from_ty = match operand_ty_kind.rigid().unwrap() {
                        RigidTy::RawPtr(ty, _) | RigidTy::Ref(_, ty, _) => Some(ty),
                        _ => None,
                    };
                    let ty_kind = ty.kind();
                    let to_ty = match ty_kind.rigid().unwrap() {
                        RigidTy::RawPtr(ty, _) | RigidTy::Ref(_, ty, _) => Some(ty),
                        _ => None,
                    };
                    if let (Some(from_ty), Some(to_ty)) = (from_ty, to_ty) {
                        if !tys_layout_equal_to_size(from_ty, to_ty) {
                            self.push_operand(operand);
                        }
                    }
                }
                _ => {}
            };
        }
        self.super_rvalue(rvalue, location);
    }

    fn visit_statement(&mut self, stmt: &Statement, location: Location) {
        if let StatementKind::Intrinsic(NonDivergingIntrinsic::CopyNonOverlapping(copy)) =
            &stmt.kind
        {
            self.push_operand(&copy.dst);
        }
        self.super_statement(stmt, location);
    }

    fn visit_terminator(&mut self, term: &Terminator, location: Location) {
        if let TerminatorKind::Call { func, args, .. } = &term.kind {
            let instance = try_resolve_instance(self.body.locals(), func).unwrap();
            if instance.kind == InstanceKind::Intrinsic {
                match Intrinsic::from_instance(&instance) {
                    Intrinsic::Copy => {
                        // Here, `dst` is the second argument.
                        self.push_operand(&args[1]);
                    }
                    Intrinsic::VolatileCopyMemory | Intrinsic::VolatileCopyNonOverlappingMemory => {
                        // Here, `dst` is the first argument.
                        self.push_operand(&args[0]);
                    }
                    _ => {}
                }
            }
        }
        self.super_terminator(term, location);
    }
}

/// Try retrieving instance for the given function operand.
fn try_resolve_instance(locals: &[LocalDecl], func: &Operand) -> Result<Instance, String> {
    let ty = func.ty(locals).unwrap();
    match ty.kind() {
        TyKind::RigidTy(RigidTy::FnDef(def, args)) => Ok(Instance::resolve(def, &args).unwrap()),
        _ => Err(format!(
            "Kani was not able to resolve the instance of the function operand `{ty:?}`. Currently, memory initialization checks in presence of function pointers and vtable calls are not supported. For more information about planned support, see https://github.com/model-checking/kani/issues/3300."
        )),
    }
}
