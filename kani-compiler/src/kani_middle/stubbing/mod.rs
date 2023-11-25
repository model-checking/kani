// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! This module contains code for implementing stubbing.

mod annotations;
mod transform;

use std::collections::BTreeMap;
use tracing::{debug, trace};

pub use self::transform::*;
use kani_metadata::HarnessMetadata;
use rustc_hir::def_id::DefId;
use rustc_hir::definitions::DefPathHash;
use rustc_middle::mir::Const;
use rustc_middle::ty::{self, EarlyBinder, ParamEnv, TyCtxt, TypeFoldable};
use rustc_smir::rustc_internal;
use stable_mir::mir::mono::Instance;
use stable_mir::mir::visit::{Location, MirVisitor};
use stable_mir::mir::Constant;
use stable_mir::{CrateDef, CrateItem};

use self::annotations::update_stub_mapping;

/// Collects the stubs from the harnesses in a crate.
pub fn harness_stub_map(
    tcx: TyCtxt,
    harness: DefId,
    metadata: &HarnessMetadata,
) -> BTreeMap<DefPathHash, DefPathHash> {
    let attrs = &metadata.attributes;
    let mut stub_pairs = BTreeMap::default();
    for stubs in &attrs.stubs {
        update_stub_mapping(tcx, harness.expect_local(), stubs, &mut stub_pairs);
    }
    stub_pairs
}

/// Validate that an instance body can be instantiated.
///
/// Stubbing may cause an instance to not be correctly instantiated since we delay checking its
/// generic bounds.
///
/// In stable mir, trying to retrieve an Instance::body() will ICE if we cannot evaluate a
/// constant as expected. For now, use internal APIs to anticipate this issue.
pub fn validate_instance(tcx: TyCtxt, instance: Instance) -> bool {
    let internal_instance = rustc_internal::internal(instance);
    if get_stub(tcx, internal_instance.def_id()).is_some() {
        let item = CrateItem::try_from(instance).unwrap();
        let mut checker = StubConstChecker::new(tcx, internal_instance, item);
        checker.visit_body(&item.body());
        checker.is_valid()
    } else {
        true
    }
}

struct StubConstChecker<'tcx> {
    tcx: TyCtxt<'tcx>,
    instance: ty::Instance<'tcx>,
    source: CrateItem,
    is_valid: bool,
}

impl<'tcx> StubConstChecker<'tcx> {
    fn new(tcx: TyCtxt<'tcx>, instance: ty::Instance<'tcx>, source: CrateItem) -> Self {
        StubConstChecker { tcx, instance, is_valid: true, source }
    }
    fn monomorphize<T>(&self, value: T) -> T
    where
        T: TypeFoldable<TyCtxt<'tcx>>,
    {
        trace!(instance=?self.instance, ?value, "monomorphize");
        self.instance.instantiate_mir_and_normalize_erasing_regions(
            self.tcx,
            ParamEnv::reveal_all(),
            EarlyBinder::bind(value),
        )
    }

    fn is_valid(&self) -> bool {
        self.is_valid
    }
}

impl<'tcx> MirVisitor for StubConstChecker<'tcx> {
    /// Collect constants that are represented as static variables.
    fn visit_constant(&mut self, constant: &Constant, location: Location) {
        let const_ = self.monomorphize(rustc_internal::internal(&constant.literal));
        debug!(?constant, ?location, ?const_, "visit_constant");
        match const_ {
            Const::Val(..) | Const::Ty(..) => {}
            Const::Unevaluated(un_eval, _) => {
                // Thread local fall into this category.
                if self.tcx.const_eval_resolve(ParamEnv::reveal_all(), un_eval, None).is_err() {
                    // The `monomorphize` call should have evaluated that constant already.
                    let tcx = self.tcx;
                    let mono_const = &un_eval;
                    let implementor = match mono_const.args.as_slice() {
                        [one] => one.as_type().unwrap(),
                        _ => unreachable!(),
                    };
                    let trait_ = tcx.trait_of_item(mono_const.def).unwrap();
                    let msg = format!(
                        "Type `{implementor}` does not implement trait `{}`. \
        This is likely because `{}` is used as a stub but its \
        generic bounds are not being met.",
                        tcx.def_path_str(trait_),
                        self.source.name()
                    );
                    tcx.sess.span_err(rustc_internal::internal(location.span()), msg);
                    self.is_valid = false;
                }
            }
        };
    }
}
