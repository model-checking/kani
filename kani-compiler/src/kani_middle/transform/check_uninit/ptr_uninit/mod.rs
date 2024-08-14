// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
//! A transformation pass that instruments the code to detect possible UB due to the accesses to
//! uninitialized memory via raw pointers.

use crate::args::ExtraChecks;
use crate::kani_middle::transform::{
    body::{CheckType, InsertPosition, MutableBody, SourceInstruction},
    check_uninit::{get_mem_init_fn_def, UninitInstrumenter},
    TransformPass, TransformationType,
};
use crate::kani_queries::QueryDb;
use rustc_middle::ty::TyCtxt;
use rustc_smir::rustc_internal;
use stable_mir::{
    mir::{mono::Instance, Body, Mutability, Place},
    ty::{FnDef, GenericArgs, Ty},
    CrateDef,
};
use std::collections::HashMap;
use std::fmt::Debug;
use tracing::trace;
use uninit_visitor::CheckUninitVisitor;

mod uninit_visitor;

/// Top-level pass that instruments the code with checks for uninitialized memory access through raw
/// pointers.
#[derive(Debug)]
pub struct UninitPass {
    pub check_type: CheckType,
    pub mem_init_fn_cache: HashMap<&'static str, FnDef>,
}

impl TransformPass for UninitPass {
    fn transformation_type() -> TransformationType
    where
        Self: Sized,
    {
        TransformationType::Instrumentation
    }

    fn is_enabled(&self, query_db: &QueryDb) -> bool
    where
        Self: Sized,
    {
        let args = query_db.args();
        args.ub_check.contains(&ExtraChecks::Uninit)
    }

    fn transform(&mut self, tcx: TyCtxt, body: Body, instance: Instance) -> (bool, Body) {
        trace!(function=?instance.name(), "transform");

        let mut changed = false;
        let mut new_body = MutableBody::from(body);

        // Inject a call to set-up memory initialization state if the function is a harness.
        if is_harness(instance, tcx) {
            inject_memory_init_setup(&mut new_body, tcx, &mut self.mem_init_fn_cache);
            changed = true;
        }

        // Call a helper that performs the actual instrumentation.
        let mut instrumenter = UninitInstrumenter {
            check_type: self.check_type.clone(),
            mem_init_fn_cache: &mut self.mem_init_fn_cache,
        };
        let (instrumentation_added, body) =
            instrumenter.instrument(tcx, new_body, instance, CheckUninitVisitor::new());

        (changed || instrumentation_added, body.into())
    }
}

/// Checks if the instance is a harness -- an entry point of Kani analysis.
fn is_harness(instance: Instance, tcx: TyCtxt) -> bool {
    let harness_identifiers = [
        vec![
            rustc_span::symbol::Symbol::intern("kanitool"),
            rustc_span::symbol::Symbol::intern("proof_for_contract"),
        ],
        vec![
            rustc_span::symbol::Symbol::intern("kanitool"),
            rustc_span::symbol::Symbol::intern("proof"),
        ],
    ];
    harness_identifiers.iter().any(|attr_path| {
        tcx.has_attrs_with_path(rustc_internal::internal(tcx, instance.def.def_id()), attr_path)
    })
}

/// Inject an initial call to set-up memory initialization tracking.
fn inject_memory_init_setup(
    new_body: &mut MutableBody,
    tcx: TyCtxt,
    mem_init_fn_cache: &mut HashMap<&'static str, FnDef>,
) {
    // First statement or terminator in the harness.
    let mut source = if !new_body.blocks()[0].statements.is_empty() {
        SourceInstruction::Statement { idx: 0, bb: 0 }
    } else {
        SourceInstruction::Terminator { bb: 0 }
    };

    // Dummy return place.
    let ret_place = Place {
        local: new_body.new_local(
            Ty::new_tuple(&[]),
            source.span(new_body.blocks()),
            Mutability::Not,
        ),
        projection: vec![],
    };

    // Resolve the instance and inject a call to set-up the memory initialization state.
    let memory_initialization_init = Instance::resolve(
        get_mem_init_fn_def(tcx, "KaniInitializeMemoryInitializationState", mem_init_fn_cache),
        &GenericArgs(vec![]),
    )
    .unwrap();

    new_body.insert_call(
        &memory_initialization_init,
        &mut source,
        InsertPosition::Before,
        vec![],
        ret_place,
    );
}
