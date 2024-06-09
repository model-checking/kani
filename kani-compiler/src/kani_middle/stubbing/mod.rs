// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! This module contains code for implementing stubbing.

mod annotations;

use itertools::Itertools;
use rustc_span::DUMMY_SP;
use std::collections::HashMap;
use tracing::{debug, trace};

use kani_metadata::HarnessMetadata;
use rustc_hir::def_id::DefId;
use rustc_middle::mir::Const;
use rustc_middle::ty::{self, EarlyBinder, ParamEnv, TyCtxt, TypeFoldable};
use rustc_smir::rustc_internal;
use stable_mir::mir::mono::Instance;
use stable_mir::mir::visit::{Location, MirVisitor};
use stable_mir::mir::Constant;
use stable_mir::ty::FnDef;
use stable_mir::{CrateDef, CrateItem};

use self::annotations::update_stub_mapping;

/// Collects the stubs from the harnesses in a crate.
pub fn harness_stub_map(
    tcx: TyCtxt,
    harness: Instance,
    metadata: &HarnessMetadata,
) -> HashMap<DefId, DefId> {
    let def_id = rustc_internal::internal(tcx, harness.def.def_id());
    let attrs = &metadata.attributes;
    let mut stub_pairs = HashMap::default();
    for stubs in &attrs.stubs {
        update_stub_mapping(tcx, def_id.expect_local(), stubs, &mut stub_pairs);
    }
    stub_pairs
}

/// Checks whether the stub is compatible with the original function/method: do
/// the arities and types (of the parameters and return values) match up? This
/// does **NOT** check whether the type variables are constrained to implement
/// the same traits; trait mismatches are checked during monomorphization.
pub fn check_compatibility(tcx: TyCtxt, old_def: FnDef, new_def: FnDef) -> Result<(), String> {
    // TODO: Validate stubs that do not have body.
    // We could potentially look at the function signature to see if they match.
    // However, they will include region information which can make types different.
    let Some(old_body) = old_def.body() else { return Ok(()) };
    let Some(new_body) = new_def.body() else { return Ok(()) };
    // Check whether the arities match.
    if old_body.arg_locals().len() != new_body.arg_locals().len() {
        let msg = format!(
            "arity mismatch: original function/method `{}` takes {} argument(s), stub `{}` takes {}",
            old_def.name(),
            old_body.arg_locals().len(),
            new_def.name(),
            new_body.arg_locals().len(),
        );
        return Err(msg);
    }
    // Check whether the numbers of generic parameters match.
    let old_def_id = rustc_internal::internal(tcx, old_def.def_id());
    let new_def_id = rustc_internal::internal(tcx, new_def.def_id());
    let old_num_generics = tcx.generics_of(old_def_id).count();
    let stub_num_generics = tcx.generics_of(new_def_id).count();
    if old_num_generics != stub_num_generics {
        let msg = format!(
            "mismatch in the number of generic parameters: original function/method `{}` takes {} generic parameters(s), stub `{}` takes {}",
            old_def.name(),
            old_num_generics,
            new_def.name(),
            stub_num_generics
        );
        return Err(msg);
    }
    // Check whether the types match. Index 0 refers to the returned value,
    // indices [1, `arg_count`] refer to the parameters.
    // TODO: We currently force generic parameters in the stub to have exactly
    // the same names as their counterparts in the original function/method;
    // instead, we should be checking for the equivalence of types up to the
    // renaming of generic parameters.
    // <https://github.com/model-checking/kani/issues/1953>
    let old_ret_ty = old_body.ret_local().ty;
    let new_ret_ty = new_body.ret_local().ty;
    let mut diff = vec![];
    if old_ret_ty != new_ret_ty {
        diff.push(format!("Expected return type `{old_ret_ty}`, but found `{new_ret_ty}`"));
    }
    for (i, (old_arg, new_arg)) in
        old_body.arg_locals().iter().zip(new_body.arg_locals().iter()).enumerate()
    {
        if old_arg.ty != new_arg.ty {
            diff.push(format!(
                "Expected type `{}` for parameter {}, but found `{}`",
                old_arg.ty,
                i + 1,
                new_arg.ty
            ));
        }
    }
    if !diff.is_empty() {
        Err(format!(
            "Cannot stub `{}` by `{}`.\n - {}",
            old_def.name(),
            new_def.name(),
            diff.iter().join("\n - ")
        ))
    } else {
        Ok(())
    }
}

/// Validate that an instance body can be instantiated.
///
/// Stubbing may cause an instance to not be correctly instantiated since we delay checking its
/// generic bounds.
///
/// In stable MIR, trying to retrieve an `Instance::body()` will ICE if we cannot evaluate a
/// constant as expected. For now, use internal APIs to anticipate this issue.
pub fn validate_stub(tcx: TyCtxt, instance: Instance) -> bool {
    debug!(?instance, "validate_instance");
    let item = CrateItem::try_from(instance).unwrap();
    let internal_instance = rustc_internal::internal(tcx, instance);
    let mut checker = StubConstChecker::new(tcx, internal_instance, item);
    checker.visit_body(&item.body());
    checker.is_valid()
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
        let const_ = self.monomorphize(rustc_internal::internal(self.tcx, &constant.literal));
        debug!(?constant, ?location, ?const_, "visit_constant");
        match const_ {
            Const::Val(..) | Const::Ty(..) => {}
            Const::Unevaluated(un_eval, _) => {
                // Thread local fall into this category.
                if self.tcx.const_eval_resolve(ParamEnv::reveal_all(), un_eval, DUMMY_SP).is_err() {
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
                    tcx.dcx().span_err(rustc_internal::internal(self.tcx, location.span()), msg);
                    self.is_valid = false;
                }
            }
        };
    }
}
