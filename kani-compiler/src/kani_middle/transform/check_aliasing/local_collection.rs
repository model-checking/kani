use super::function_cache::*;
use super::{
    Body, BodyMutationPassState, CachedBodyMutator, InstrumentationData, Local, LocalDecl,
    MirVisitor, TyCtxt,
};
use std::collections::HashMap;

/// Collect local visitor visits the body
/// and collects all of the (non argument)
/// locals. In the future it will collect
/// argument locals and initialize arguments.
struct CollectLocalVisitor {
    values: Vec<Local>,
}

impl CollectLocalVisitor {
    fn new() -> Self {
        let values = Vec::new();
        CollectLocalVisitor { values }
    }
}

impl MirVisitor for CollectLocalVisitor {
    fn visit_local_decl(&mut self, local: Local, decl: &LocalDecl) {
        let _ = decl;
        self.values.push(local);
    }
}

/// The local collection pass state collects locals
/// from function bodies based on whether they are
/// function arguments or locals from the function body.
pub struct LocalCollectionPassState<'tcx, 'cache> {
    /// The function body
    body: Body,
    /// The compilation context
    tcx: TyCtxt<'tcx>,
    /// The function instance cache, which may
    /// be populated by previous runs of the aliasing
    /// pass.
    cache: &'cache mut Cache,
    /// Values
    values: CollectLocalVisitor,
}

impl<'tcx, 'cache> LocalCollectionPassState<'tcx, 'cache> {
    pub fn new(body: Body, tcx: TyCtxt<'tcx>, cache: &'cache mut Cache) -> Self {
        let values = CollectLocalVisitor::new();
        Self { body, tcx, cache, values }
    }

    pub fn collect_locals(mut self) -> Self {
        self.values.visit_body(&self.body);
        self
    }

    pub fn collect_body(self) -> BodyMutationPassState<'tcx, 'cache> {
        let values = self.values.values;
        let body = CachedBodyMutator::from(self.body);
        let instrumentation_data =
            InstrumentationData::new(self.tcx, self.cache, HashMap::new(), body);
        BodyMutationPassState::new(values, instrumentation_data)
    }
}
