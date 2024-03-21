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
use crate::kani_middle::transform::check_values::ValidValuePass;
use crate::kani_queries::QueryDb;
use rustc_middle::ty::TyCtxt;
use stable_mir::mir::mono::Instance;
use stable_mir::mir::Body;
use std::collections::HashMap;
use std::fmt::Debug;

mod body;
mod check_values;

/// Object used to retrieve a transformed instance body.
/// The transformations to be applied may be controlled by user options.
///
/// The order however is always the same, we run optimizations first, and instrument the code
/// after.
#[derive(Debug)]
pub struct BodyTransformation {
    /// The passes that may optimize the function body.
    /// We store them separately from the instrumentation passes because we run the in specific order.
    opt_passes: Vec<Box<dyn TransformPass>>,
    /// The passes that may add safety checks to the function body.
    inst_passes: Vec<Box<dyn TransformPass>>,
    /// Cache transformation results.
    cache: HashMap<Instance, TransformationResult>,
}

impl BodyTransformation {
    pub fn new(queries: &QueryDb, tcx: TyCtxt) -> Self {
        let mut transformer = BodyTransformation {
            opt_passes: vec![],
            inst_passes: vec![],
            cache: Default::default(),
        };
        transformer.add_pass(queries, ValidValuePass::new(tcx));
        transformer
    }

    /// Allow the creation of a dummy transformer that doesn't apply any transformation due to
    /// the stubbing validation hack (see `collect_and_partition_mono_items` override.
    /// Once we move the stubbing logic to a [TransformPass], we should be able to remove this.
    pub fn dummy() -> Self {
        BodyTransformation { opt_passes: vec![], inst_passes: vec![], cache: Default::default() }
    }

    pub fn body(&mut self, tcx: TyCtxt, instance: Instance) -> Option<Body> {
        match self.cache.get(&instance) {
            Some(TransformationResult::Modified(body)) => Some(body.clone()),
            Some(TransformationResult::NotModified) => instance.body(),
            None => {
                let mut body = instance.body()?;
                let mut modified = false;
                for pass in self.opt_passes.iter().chain(self.inst_passes.iter()) {
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
                Some(body)
            }
        }
    }

    fn add_pass<P: TransformPass + 'static>(&mut self, query_db: &QueryDb, pass: P) {
        if pass.is_enabled(&query_db) {
            match P::transformation_type() {
                TransformationType::Instrumentation => self.inst_passes.push(Box::new(pass)),
                TransformationType::Optimization => {
                    unreachable!()
                }
            }
        }
    }
}

/// The type of transformation that a pass may perform.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
enum TransformationType {
    /// Should only add assertion checks to ensure the program is correct.
    Instrumentation,
    /// May replace inefficient code with more performant but equivalent code.
    #[allow(dead_code)]
    Optimization,
}

/// A trait to represent transformation passes that can be used to modify the body of a function.
trait TransformPass: Debug {
    /// The type of transformation that this pass implements.
    fn transformation_type() -> TransformationType
    where
        Self: Sized;

    fn is_enabled(&self, query_db: &QueryDb) -> bool
    where
        Self: Sized;

    /// Run a transformation pass in the function body.
    fn transform(&self, tcx: TyCtxt, body: Body, instance: Instance) -> (bool, Body);
}

/// The transformation result.
/// We currently only cache the body of functions that were instrumented.
#[derive(Clone, Debug)]
enum TransformationResult {
    Modified(Body),
    NotModified,
}
