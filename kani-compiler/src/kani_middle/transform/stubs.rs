// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! This module contains code related to the MIR-to-MIR pass that performs the
//! stubbing of functions and methods.
use crate::kani_middle::codegen_units::Stubs;
use crate::kani_middle::stubbing::{contract_host_param, validate_stub_const};
use crate::kani_middle::transform::body::{MutMirVisitor, MutableBody};
use crate::kani_middle::transform::{TransformPass, TransformationType};
use crate::kani_queries::QueryDb;
use rustc_middle::ty::TyCtxt;
use rustc_smir::rustc_internal;
use stable_mir::CrateDef;
use stable_mir::mir::mono::Instance;
use stable_mir::mir::visit::{Location, MirVisitor};
use stable_mir::mir::{Body, ConstOperand, LocalDecl, Operand, Terminator, TerminatorKind};
use stable_mir::ty::{FnDef, MirConst, RigidTy, TyKind};
use std::collections::HashMap;
use std::fmt::Debug;
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
    fn transform(&mut self, tcx: TyCtxt, body: Body, instance: Instance) -> (bool, Body) {
        trace!(function=?instance.name(), "transform");
        let ty = instance.ty();
        if let TyKind::RigidTy(RigidTy::FnDef(fn_def, mut args)) = ty.kind() {
            if let Some(replace) = self.stubs.get(&fn_def) {
                if let Some(idx) = contract_host_param(tcx, fn_def, *replace) {
                    debug!(?idx, "FnStubPass::transform remove_host_param");
                    args.0.remove(idx);
                }
                let new_instance = Instance::resolve(*replace, &args).unwrap();
                debug!(from=?instance.name(), to=?new_instance.name(), "FnStubPass::transform");
                if let Some(body) = FnStubValidator::validate(tcx, (fn_def, *replace), new_instance)
                {
                    return (true, body);
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
    ///
    /// We need to find function calls and function pointers.
    /// We should replace this with a visitor once StableMIR includes a mutable one.
    fn transform(&mut self, _tcx: TyCtxt, body: Body, instance: Instance) -> (bool, Body) {
        trace!(function=?instance.name(), "transform");
        let mut new_body = MutableBody::from(body);
        let changed = false;
        let locals = new_body.locals().to_vec();
        let mut visitor = ExternFnStubVisitor { changed, locals, stubs: &self.stubs };
        visitor.visit_body(&mut new_body);
        (visitor.changed, new_body.into())
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

/// Validate that the body of the stub is valid for the given instantiation
struct FnStubValidator<'a, 'tcx> {
    stub: (FnDef, FnDef),
    tcx: TyCtxt<'tcx>,
    locals: &'a [LocalDecl],
    is_valid: bool,
}

impl<'a, 'tcx> FnStubValidator<'a, 'tcx> {
    fn validate(tcx: TyCtxt, stub: (FnDef, FnDef), new_instance: Instance) -> Option<Body> {
        if validate_stub_const(tcx, new_instance) {
            let body = new_instance.body().unwrap();
            let mut validator =
                FnStubValidator { stub, tcx, locals: body.locals(), is_valid: true };
            validator.visit_body(&body);
            validator.is_valid.then_some(body)
        } else {
            None
        }
    }
}

impl<'a, 'tcx> MirVisitor for FnStubValidator<'a, 'tcx> {
    fn visit_operand(&mut self, op: &Operand, loc: Location) {
        let op_ty = op.ty(self.locals).unwrap();
        if let TyKind::RigidTy(RigidTy::FnDef(def, args)) = op_ty.kind() {
            if Instance::resolve(def, &args).is_err() {
                self.is_valid = false;
                let callee = def.name();
                let receiver_ty = args.0[0].expect_ty();
                let sep = callee.rfind("::").unwrap();
                let trait_ = &callee[..sep];
                self.tcx.dcx().span_err(
                    rustc_internal::internal(self.tcx, loc.span()),
                    format!(
                        "`{}` doesn't implement \
                                        `{}`. The function `{}` \
                                        cannot be stubbed by `{}` due to \
                                        generic bounds not being met. Callee: {}",
                        receiver_ty,
                        trait_,
                        self.stub.0.name(),
                        self.stub.1.name(),
                        callee,
                    ),
                );
            }
        }
    }
}

struct ExternFnStubVisitor<'a> {
    changed: bool,
    locals: Vec<LocalDecl>,
    stubs: &'a Stubs,
}

impl<'a> MutMirVisitor for ExternFnStubVisitor<'a> {
    fn visit_terminator(&mut self, term: &mut Terminator) {
        // Replace direct calls
        if let TerminatorKind::Call { func, .. } = &mut term.kind {
            if let TyKind::RigidTy(RigidTy::FnDef(def, args)) =
                func.ty(&self.locals).unwrap().kind()
            {
                if let Some(new_def) = self.stubs.get(&def) {
                    let instance = Instance::resolve(*new_def, &args).unwrap();
                    let literal = MirConst::try_new_zero_sized(instance.ty()).unwrap();
                    let span = term.span;
                    let new_func = ConstOperand { span, user_ty: None, const_: literal };
                    *func = Operand::Constant(new_func);
                    self.changed = true;
                }
            }
        }
        self.super_terminator(term);
    }

    fn visit_operand(&mut self, operand: &mut Operand) {
        let func_ty = operand.ty(&self.locals).unwrap();
        if let TyKind::RigidTy(RigidTy::FnDef(orig_def, args)) = func_ty.kind() {
            if let Some(new_def) = self.stubs.get(&orig_def) {
                let Operand::Constant(ConstOperand { span, .. }) = operand else {
                    unreachable!();
                };
                let instance = Instance::resolve_for_fn_ptr(*new_def, &args).unwrap();
                let literal = MirConst::try_new_zero_sized(instance.ty()).unwrap();
                let new_func = ConstOperand { span: *span, user_ty: None, const_: literal };
                *operand = Operand::Constant(new_func);
                self.changed = true;
            }
        }
    }
}
