// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use crate::kani_queries::QueryDb;
use rustc_middle::ty::{Instance, TyCtxt};
use tracing::debug;

/// A context that provides the main methods for translating MIR constructs to
/// Boogie and stores what has been codegen so far
pub struct BoogieCtx<'tcx> {
    /// the typing context
    pub tcx: TyCtxt<'tcx>,
    /// a snapshot of the query values. The queries shouldn't change at this point,
    /// so we just keep a copy.
    pub queries: QueryDb,
}

impl<'tcx> BoogieCtx<'tcx> {
    pub fn new(tcx: TyCtxt<'tcx>, queries: QueryDb) -> BoogieCtx<'tcx> {
        BoogieCtx { tcx, queries }
    }

    pub fn declare_function(&mut self, instance: Instance) {
        debug!(?instance, "boogie_codegen_declare_function");
    }
}
