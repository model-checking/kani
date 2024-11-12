// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
//! Module responsible for implementing a few Rust compiler intrinsics.
//!
//! Note that some rustc intrinsics are lowered to MIR instructions. Those can also be handled
//! here.

use crate::intrinsics::Intrinsic;
use crate::kani_middle::kani_functions::{KaniFunction, KaniModel};
use crate::kani_middle::transform::body::{MutMirVisitor, MutableBody};
use crate::kani_middle::transform::{TransformPass, TransformationType};
use crate::kani_queries::QueryDb;
use rustc_middle::ty::TyCtxt;
use stable_mir::mir::mono::Instance;
use stable_mir::mir::{Body, ConstOperand, LocalDecl, Operand, Terminator, TerminatorKind};
use stable_mir::ty::{FnDef, MirConst, RigidTy, TyKind};
use std::collections::HashMap;
use tracing::debug;

/// Generate the body for a few Kani intrinsics.
#[derive(Debug)]
pub struct RustcIntrinsicsPass {
    /// Used to cache FnDef lookups for intrinsics models.
    models: HashMap<KaniModel, FnDef>,
}

impl TransformPass for RustcIntrinsicsPass {
    fn transformation_type() -> TransformationType
    where
        Self: Sized,
    {
        TransformationType::Stubbing
    }

    fn is_enabled(&self, _query_db: &QueryDb) -> bool
    where
        Self: Sized,
    {
        true
    }

    /// Transform the function body by inserting checks one-by-one.
    /// For every unsafe dereference or a transmute operation, we check all values are valid.
    fn transform(&mut self, _tcx: TyCtxt, body: Body, instance: Instance) -> (bool, Body) {
        debug!(function=?instance.name(), "transform");
        let mut new_body = MutableBody::from(body);
        let mut visitor = ReplaceIntrinsicVisitor::new(&self.models, new_body.locals().to_vec());
        visitor.visit_body(&mut new_body);
        (visitor.changed, new_body.into())
    }
}

impl RustcIntrinsicsPass {
    pub fn new(queries: &QueryDb) -> Self {
        let models = queries
            .kani_functions()
            .iter()
            .filter_map(|(func, def)| {
                if let KaniFunction::Model(model) = func { Some((*model, *def)) } else { None }
            })
            .collect();
        debug!(?models, "RustcIntrinsicsPass::new");
        RustcIntrinsicsPass { models }
    }
}

struct ReplaceIntrinsicVisitor<'a> {
    models: &'a HashMap<KaniModel, FnDef>,
    locals: Vec<LocalDecl>,
    changed: bool,
}

impl<'a> ReplaceIntrinsicVisitor<'a> {
    fn new(models: &'a HashMap<KaniModel, FnDef>, locals: Vec<LocalDecl>) -> Self {
        ReplaceIntrinsicVisitor { models, locals, changed: false }
    }
}

impl MutMirVisitor for ReplaceIntrinsicVisitor<'_> {
    /// Replace the terminator for some intrinsics.
    ///
    /// Note that intrinsics must always be called directly.
    fn visit_terminator(&mut self, term: &mut Terminator) {
        if let TerminatorKind::Call { func, .. } = &mut term.kind {
            if let TyKind::RigidTy(RigidTy::FnDef(def, args)) =
                func.ty(&self.locals).unwrap().kind()
            {
                if def.is_intrinsic() {
                    let instance = Instance::resolve(def, &args).unwrap();
                    let intrinsic = Intrinsic::from_instance(&instance);
                    debug!(?intrinsic, "handle_terminator");
                    let model = match intrinsic {
                        Intrinsic::SizeOfVal => self.models[&KaniModel::SizeOfVal],
                        Intrinsic::MinAlignOfVal => self.models[&KaniModel::AlignOfVal],
                        // The rest is handled in hooks.
                        _ => {
                            return self.super_terminator(term);
                        }
                    };
                    let new_instance = Instance::resolve(model, &args).unwrap();
                    let literal = MirConst::try_new_zero_sized(new_instance.ty()).unwrap();
                    let span = term.span;
                    let new_func = ConstOperand { span, user_ty: None, const_: literal };
                    *func = Operand::Constant(new_func);
                    self.changed = true;
                }
            }
        }
        self.super_terminator(term);
    }
}
