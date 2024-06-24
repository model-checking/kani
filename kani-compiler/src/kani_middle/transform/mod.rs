// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
//! This module is responsible for optimizing and instrumenting function bodies.
//!
//! We make transformations on bodies already monomorphized, which allow us to make stronger
//! decisions based on the instance types and constants.
//!
//! The main downside is that some transformation that don't depend on the specialized type may be
//! applied multiple times, one per specialization.
//!
//! Another downside is that these modifications cannot be applied to concrete playback, since they
//! are applied on the top of StableMIR body, which cannot be propagated back to rustc's backend.
//!
//! # Warn
//!
//! For all instrumentation passes, always use exhaustive matches to ensure soundness in case a new
//! case is added.
use crate::kani_middle::codegen_units::CodegenUnit;
use crate::kani_middle::transform::body::CheckType;
use crate::kani_middle::transform::check_uninit::UninitPass;
use crate::kani_middle::transform::check_values::ValidValuePass;
use crate::kani_middle::transform::contracts::AnyModifiesPass;
use crate::kani_middle::transform::kani_intrinsics::IntrinsicGeneratorPass;
use crate::kani_middle::transform::stubs::{ExternFnStubPass, FnStubPass};
use crate::kani_queries::QueryDb;
use rustc_middle::ty::TyCtxt;
use stable_mir::mir::mono::Instance;
use stable_mir::mir::Body;
use std::collections::HashMap;
use std::fmt::Debug;

pub(crate) mod body;
mod check_uninit;
mod check_values;
mod contracts;
mod kani_intrinsics;
mod stubs;

/// Object used to retrieve a transformed instance body.
/// The transformations to be applied may be controlled by user options.
///
/// The order however is always the same, we run optimizations first, and instrument the code
/// after.
#[derive(Debug)]
pub struct BodyTransformation {
    /// The passes that may change the function body according to harness configuration.
    /// The stubbing passes should be applied before so user stubs take precedence.
    stub_passes: Vec<Box<dyn TransformPass>>,
    /// The passes that may add safety checks to the function body.
    inst_passes: Vec<Box<dyn TransformPass>>,
    /// Cache transformation results.
    cache: HashMap<Instance, TransformationResult>,
}

impl BodyTransformation {
    pub fn new(queries: &QueryDb, tcx: TyCtxt, unit: &CodegenUnit) -> Self {
        let mut transformer = BodyTransformation {
            stub_passes: vec![],
            inst_passes: vec![],
            cache: Default::default(),
        };
        let check_type = CheckType::new(tcx);
        transformer.add_pass(queries, FnStubPass::new(&unit.stubs));
        transformer.add_pass(queries, ExternFnStubPass::new(&unit.stubs));
        // This has to come after stubs since we want this to replace the stubbed body.
        transformer.add_pass(queries, AnyModifiesPass::new(tcx, &unit));
        transformer.add_pass(queries, ValidValuePass { check_type: check_type.clone() });
        transformer.add_pass(
            queries,
            UninitPass { check_type: check_type.clone(), mem_init_fn_cache: HashMap::new() },
        );
        transformer.add_pass(
            queries,
            IntrinsicGeneratorPass { check_type, mem_init_fn_cache: HashMap::new() },
        );
        transformer
    }

    /// Retrieve the body of an instance.
    ///
    /// Note that this assumes that the instance does have a body since existing consumers already
    /// assume that. Use `instance.has_body()` to check if an instance has a body.
    pub fn body(&mut self, tcx: TyCtxt, instance: Instance) -> Body {
        match self.cache.get(&instance) {
            Some(TransformationResult::Modified(body)) => body.clone(),
            Some(TransformationResult::NotModified) => instance.body().unwrap(),
            None => {
                let mut body = instance.body().unwrap();
                let mut modified = false;
                for pass in self.stub_passes.iter_mut().chain(self.inst_passes.iter_mut()) {
                    let result = pass.transform(tcx, body, instance);
                    modified |= result.0;
                    body = result.1;
                }

                let result = if modified {
                    TransformationResult::Modified(body.clone())
                } else {
                    TransformationResult::NotModified
                };
                self.cache.insert(instance, result);
                body
            }
        }
    }

    fn add_pass<P: TransformPass + 'static>(&mut self, query_db: &QueryDb, pass: P) {
        if pass.is_enabled(&query_db) {
            match P::transformation_type() {
                TransformationType::Instrumentation => self.inst_passes.push(Box::new(pass)),
                TransformationType::Stubbing => self.stub_passes.push(Box::new(pass)),
            }
        }
    }
}

/// The type of transformation that a pass may perform.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub(crate) enum TransformationType {
    /// Should only add assertion checks to ensure the program is correct.
    Instrumentation,
    /// Apply some sort of stubbing.
    Stubbing,
}

/// A trait to represent transformation passes that can be used to modify the body of a function.
pub(crate) trait TransformPass: Debug {
    /// The type of transformation that this pass implements.
    fn transformation_type() -> TransformationType
    where
        Self: Sized;

    fn is_enabled(&self, query_db: &QueryDb) -> bool
    where
        Self: Sized;

    /// Run a transformation pass in the function body.
    fn transform(&mut self, tcx: TyCtxt, body: Body, instance: Instance) -> (bool, Body);
}

/// The transformation result.
/// We currently only cache the body of functions that were instrumented.
#[derive(Clone, Debug)]
enum TransformationResult {
    Modified(Body),
    NotModified,
}
