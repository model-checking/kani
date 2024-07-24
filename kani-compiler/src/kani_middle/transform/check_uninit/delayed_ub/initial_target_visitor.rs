// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
//! This module contains the visitor responsible for collecting initial analysis targets for delayed
//! UB instrumentation.

use stable_mir::{
    mir::{
        mono::{Instance, InstanceKind},
        visit::Location,
        Body, CastKind, LocalDecl, MirVisitor, Mutability, NonDivergingIntrinsic, Operand, Place,
        Rvalue, Statement, StatementKind, Terminator, TerminatorKind,
    },
    ty::{RigidTy, TyKind},
};

use crate::kani_middle::transform::check_uninit::ty_layout::tys_layout_compatible;

/// Visitor that finds initial analysis targets for delayed UB instrumentation. For our purposes,
/// analysis targets are *pointers* to places reading and writing from which should be tracked.
pub struct InitialTargetVisitor {
    body: Body,
    targets: Vec<Place>,
}

impl InitialTargetVisitor {
    pub fn new(body: Body) -> Self {
        Self { body, targets: vec![] }
    }

    pub fn into_targets(self) -> Vec<Place> {
        self.targets
    }
}

impl MirVisitor for InitialTargetVisitor {
    fn visit_rvalue(&mut self, rvalue: &Rvalue, location: Location) {
        if let Rvalue::Cast(kind, operand, ty) = rvalue {
            let operand_ty = operand.ty(self.body.locals()).unwrap();
            match kind {
                CastKind::Transmute | CastKind::PtrToPtr => {
                    if let (
                        RigidTy::RawPtr(from_ty, Mutability::Mut),
                        RigidTy::RawPtr(to_ty, Mutability::Mut),
                    ) = (operand_ty.kind().rigid().unwrap(), ty.kind().rigid().unwrap())
                    {
                        match operand {
                            Operand::Copy(place) | Operand::Move(place) => {
                                if !tys_layout_compatible(from_ty, to_ty) {
                                    self.targets.push(place.clone());
                                }
                            }
                            Operand::Constant(_) => {
                                unreachable!("cannot be a constant")
                            }
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
            match &copy.dst {
                Operand::Copy(place) | Operand::Move(place) => {
                    self.targets.push(place.clone());
                }
                Operand::Constant(_) => {
                    unreachable!("cannot be a constant")
                }
            }
        }
        self.super_statement(stmt, location);
    }

    fn visit_terminator(&mut self, term: &Terminator, location: Location) {
        if let TerminatorKind::Call { func, args, .. } = &term.kind {
            let instance = match try_resolve_instance(self.body.locals(), func) {
                Ok(instance) => instance,
                Err(reason) => {
                    panic!("{reason}");
                }
            };
            if instance.kind == InstanceKind::Intrinsic {
                match instance.intrinsic_name().unwrap().as_str() {
                    "copy" => {
                        assert_eq!(args.len(), 3, "Unexpected number of arguments for `copy`");
                        assert!(matches!(
                            args[0].ty(self.body.locals()).unwrap().kind(),
                            TyKind::RigidTy(RigidTy::RawPtr(_, Mutability::Not))
                        ));
                        assert!(matches!(
                            args[1].ty(self.body.locals()).unwrap().kind(),
                            TyKind::RigidTy(RigidTy::RawPtr(_, Mutability::Mut))
                        ));
                        // Here, `dst` is the second argument.
                        match &args[1] {
                            Operand::Copy(place) | Operand::Move(place) => {
                                self.targets.push(place.clone());
                            }
                            Operand::Constant(_) => unreachable!("cannot be a constant"),
                        }
                    }
                    "volatile_copy_memory" | "volatile_copy_nonoverlapping_memory" => {
                        assert_eq!(
                            args.len(),
                            3,
                            "Unexpected number of arguments for `volatile_copy`"
                        );
                        assert!(matches!(
                            args[0].ty(self.body.locals()).unwrap().kind(),
                            TyKind::RigidTy(RigidTy::RawPtr(_, Mutability::Mut))
                        ));
                        assert!(matches!(
                            args[1].ty(self.body.locals()).unwrap().kind(),
                            TyKind::RigidTy(RigidTy::RawPtr(_, Mutability::Not))
                        ));
                        // Here, `dst` is the first argument.
                        match &args[0] {
                            Operand::Copy(place) | Operand::Move(place) => {
                                self.targets.push(place.clone());
                            }
                            Operand::Constant(_) => unreachable!("cannot be a constant"),
                        }
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
        _ => Err(format!("Kani does not support reasoning about arguments to `{ty:?}`.")),
    }
}
