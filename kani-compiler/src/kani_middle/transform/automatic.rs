// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This module transforms the body of an automatic harness to verify a function.
//! Upon entry to this module, a harness has the dummy body of the automatic_harness Kani intrinsic.
//! We obtain the function its meant to verify by inspecting its generic arguments,
//! then transform its body to be a harness for that function.

use crate::args::ReachabilityType;
use crate::kani_middle::codegen_units::CodegenUnit;
use crate::kani_middle::kani_functions::{KaniIntrinsic, KaniModel};
use crate::kani_middle::transform::body::{InsertPosition, MutableBody, SourceInstruction};
use crate::kani_middle::transform::{TransformPass, TransformationType};
use crate::kani_queries::QueryDb;
use rustc_middle::ty::TyCtxt;
use stable_mir::mir::mono::Instance;
use stable_mir::mir::{Body, Operand, Place, TerminatorKind};
use stable_mir::ty::{FnDef, GenericArgKind, GenericArgs};
use tracing::debug;

#[derive(Debug)]
pub struct AutomaticHarnessPass {
    /// The FnDef of KaniModel::Any
    kani_any: FnDef,
    /// All of the automatic harness Instances that we generated in the CodegenUnits constructor
    automatic_harnesses: Vec<Instance>,
}

impl AutomaticHarnessPass {
    // FIXME: this is a bit clunky.
    // Historically, in codegen_crate, we reset the BodyTransformation cache on a per-unit basis,
    // so the BodyTransformation constructor only accepts a CodegenUnit and thus this constructor can only accept a unit.
    // Later, we changed codegen to reset the cache on a per-harness basis (for uninitialized memory instrumentation).
    // So BodyTransformation should really be changed to reflect that, so that this constructor can just save the one automatic harness it should transform
    // and not all of the possibilities.
    pub fn new(unit: &CodegenUnit, query_db: &QueryDb) -> Self {
        let kani_fns = query_db.kani_functions();
        let harness_intrinsic = *kani_fns.get(&KaniIntrinsic::AutomaticHarness.into()).unwrap();
        let kani_any = *kani_fns.get(&KaniModel::Any.into()).unwrap();
        let automatic_harnesses = unit
            .harnesses
            .iter()
            .cloned()
            .filter(|harness| {
                let (def, _) = harness.ty().kind().fn_def().unwrap();
                def == harness_intrinsic
            })
            .collect::<Vec<_>>();
        Self { kani_any, automatic_harnesses }
    }
}

impl TransformPass for AutomaticHarnessPass {
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
        matches!(query_db.args().reachability_analysis, ReachabilityType::AllFns)
    }

    fn transform(&mut self, _tcx: TyCtxt, body: Body, instance: Instance) -> (bool, Body) {
        debug!(function=?instance.name(), "AutomaticHarnessPass::transform");

        if !self.automatic_harnesses.contains(&instance) {
            return (false, body);
        }

        // Retrieve the generic arguments of the harness, which is the type of the function it is verifying,
        // and then resolve `fn_to_verify`.
        let kind = instance.args().0[0].expect_ty().kind();
        let (def, args) = kind.fn_def().unwrap();
        let fn_to_verify = Instance::resolve(def, &args).unwrap();
        let fn_to_verify_body = fn_to_verify.body().unwrap();

        let mut harness_body = MutableBody::from(body);
        harness_body.clear_body(TerminatorKind::Return);
        let mut source = SourceInstruction::Terminator { bb: 0 };

        let mut arg_locals = vec![];

        // For each argument of `fn_to_verify`, create a nondeterministic value of its type
        // by generating a kani::any() call and saving the result in `arg_local`.
        for local_decl in fn_to_verify_body.arg_locals().iter() {
            let arg_local = harness_body.new_local(
                local_decl.ty,
                source.span(harness_body.blocks()),
                local_decl.mutability,
            );
            let kani_any_inst = Instance::resolve(
                self.kani_any,
                &GenericArgs(vec![GenericArgKind::Type(local_decl.ty)]),
            )
            .unwrap();
            harness_body.insert_call(
                &kani_any_inst,
                &mut source,
                InsertPosition::Before,
                vec![],
                Place::from(arg_local),
            );
            arg_locals.push(arg_local);
        }

        let func_to_verify_ret = fn_to_verify_body.ret_local();
        let ret_place = Place::from(harness_body.new_local(
            func_to_verify_ret.ty,
            source.span(harness_body.blocks()),
            func_to_verify_ret.mutability,
        ));

        // Call `fn_to_verify` on the nondeterministic arguments generated above.
        harness_body.insert_call(
            &fn_to_verify,
            &mut source,
            InsertPosition::Before,
            arg_locals.iter().map(|lcl| Operand::Copy(Place::from(*lcl))).collect::<Vec<_>>(),
            ret_place,
        );

        (true, harness_body.into())
    }
}
