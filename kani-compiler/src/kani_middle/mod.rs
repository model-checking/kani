// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! This module contains code that are backend agnostic. For example, MIR analysis
//! and transformations.

use std::collections::HashSet;

use crate::kani_queries::QueryDb;
use fxhash::FxHashMap;
use rustc_hir::{def::DefKind, def_id::DefId as InternalDefId, def_id::LOCAL_CRATE};
use rustc_middle::ty::TyCtxt;
use stable_mir::mir::TerminatorKind;
use stable_mir::mir::mono::{Instance, MonoItem};
use stable_mir::rustc_internal;
use stable_mir::ty::{
    AdtDef, AdtKind, FnDef, GenericArgKind, GenericArgs, RigidTy, Span as SpanStable, Ty, TyKind,
};
use stable_mir::visitor::{Visitable, Visitor as TyVisitor};
use stable_mir::{CrateDef, DefId};
use std::ops::ControlFlow;

use self::attributes::KaniAttributes;

pub mod abi;
pub mod analysis;
pub mod attributes;
pub mod codegen_units;
pub mod coercion;
mod intrinsics;
pub mod kani_functions;
pub mod metadata;
pub mod points_to;
pub mod provide;
pub mod reachability;
pub mod resolve;
pub mod stubbing;
pub mod transform;

/// Check that all crate items are supported and there's no misconfiguration.
/// This method will exhaustively print any error / warning and it will abort at the end if any
/// error was found.
pub fn check_crate_items(tcx: TyCtxt, ignore_asm: bool) {
    let krate = tcx.crate_name(LOCAL_CRATE);
    for item in tcx.hir_free_items() {
        let def_id = item.owner_id.def_id.to_def_id();
        KaniAttributes::for_item(tcx, def_id).check_attributes();
        if tcx.def_kind(def_id) == DefKind::GlobalAsm {
            if !ignore_asm {
                let error_msg = format!(
                    "Crate {krate} contains global ASM, which is not supported by Kani. Rerun with \
                    `-Z unstable-options --ignore-global-asm` to suppress this error \
                    (**Verification results may be impacted**).",
                );
                tcx.dcx().err(error_msg);
            } else {
                tcx.dcx().warn(format!(
                    "Ignoring global ASM in crate {krate}. Verification results may be impacted.",
                ));
            }
        }
    }
    tcx.dcx().abort_if_errors();
}

/// Traverse the type definition to see if the type contains interior mutability.
///
/// See <https://doc.rust-lang.org/reference/interior-mutability.html> for more details.
pub fn is_interior_mut(tcx: TyCtxt, ty: Ty) -> bool {
    let mut visitor = FindUnsafeCell { tcx };
    visitor.visit_ty(&ty) == ControlFlow::Break(())
}

struct FindUnsafeCell<'tcx> {
    tcx: TyCtxt<'tcx>,
}

impl TyVisitor for FindUnsafeCell<'_> {
    type Break = ();
    fn visit_ty(&mut self, ty: &Ty) -> ControlFlow<Self::Break> {
        match ty.kind() {
            TyKind::RigidTy(RigidTy::Adt(def, _))
                if rustc_internal::internal(self.tcx, def).is_unsafe_cell() =>
            {
                ControlFlow::Break(())
            }
            TyKind::RigidTy(RigidTy::Ref(..) | RigidTy::RawPtr(..)) => {
                // We only care about the current memory space.
                ControlFlow::Continue(())
            }
            _ => ty.super_visit(self),
        }
    }
}

/// Check that all given items are supported and there's no misconfiguration.
/// This method will exhaustively print any error / warning and it will abort at the end if any
/// error was found.
pub fn check_reachable_items(tcx: TyCtxt, queries: &QueryDb, items: &[MonoItem]) {
    // Avoid printing the same error multiple times for different instantiations of the same item.
    let mut def_ids = HashSet::new();
    let reachable_functions: HashSet<InternalDefId> = items
        .iter()
        .filter_map(|i| match i {
            MonoItem::Fn(instance) => Some(rustc_internal::internal(tcx, instance.def.def_id())),
            _ => None,
        })
        .collect();
    for item in items.iter().filter(|i| matches!(i, MonoItem::Fn(..) | MonoItem::Static(..))) {
        let def_id = match item {
            MonoItem::Fn(instance) => instance.def.def_id(),
            MonoItem::Static(def) => def.def_id(),
            MonoItem::GlobalAsm(_) => {
                unreachable!()
            }
        };
        if !def_ids.contains(&def_id) {
            let attributes = KaniAttributes::for_def_id(tcx, def_id);
            // Check if any unstable attribute was reached.
            attributes.check_unstable_features(&queries.args().unstable_features);
            // Check whether all `proof_for_contract` functions are reachable
            attributes.check_proof_for_contract(&reachable_functions);
            def_ids.insert(def_id);
        }
    }
    tcx.dcx().abort_if_errors();
}

/// Structure that represents the source location of a definition.
/// TODO: Use `InternedString` once we move it out of the cprover_bindings.
/// <https://github.com/model-checking/kani/issues/2435>
pub struct SourceLocation {
    pub filename: String,
    pub start_line: usize,
    #[allow(dead_code)]
    pub start_col: usize, // set, but not currently used in Goto output
    pub end_line: usize,
    #[allow(dead_code)]
    pub end_col: usize, // set, but not currently used in Goto output
}

impl SourceLocation {
    pub fn new(span: SpanStable) -> Self {
        let loc = span.get_lines();
        let filename = span.get_filename().to_string();
        let start_line = loc.start_line;
        let start_col = loc.start_col;
        let end_line = loc.end_line;
        let end_col = loc.end_col;
        SourceLocation { filename, start_line, start_col, end_line, end_col }
    }
}

/// Return whether `def_id` refers to a nested static allocation.
pub fn is_anon_static(tcx: TyCtxt, def_id: DefId) -> bool {
    let int_def_id = rustc_internal::internal(tcx, def_id);
    match tcx.def_kind(int_def_id) {
        rustc_hir::def::DefKind::Static { nested, .. } => nested,
        _ => false,
    }
}

/// Try to convert an internal `DefId` to a `FnDef`.
pub fn stable_fn_def(tcx: TyCtxt, def_id: InternalDefId) -> Option<FnDef> {
    if let TyKind::RigidTy(RigidTy::FnDef(def, _)) =
        rustc_internal::stable(tcx.type_of(def_id)).value.kind()
    {
        Some(def)
    } else {
        None
    }
}

/// Inspect a `kani::any<T>()` call to determine if `T: Arbitrary`
/// `kani_any_def` refers to a function that looks like:
/// ```rust
/// fn any<T: Arbitrary>() -> T {
///   T::any()
/// }
/// ```
/// So we select the terminator that calls T::kani::Arbitrary::any(), then try to resolve it to an Instance.
/// `T` implements Arbitrary iff we successfully resolve the Instance.
fn implements_arbitrary(
    ty: Ty,
    kani_any_def: FnDef,
    ty_arbitrary_cache: &mut FxHashMap<Ty, bool>,
) -> bool {
    if let Some(v) = ty_arbitrary_cache.get(&ty) {
        return *v;
    }

    if ty.kind().rigid().is_none() {
        return false;
    }

    if let TyKind::RigidTy(RigidTy::Ref(_, inner_ty, _)) = ty.kind() {
        if let TyKind::RigidTy(RigidTy::Adt(..)) = inner_ty.kind() {
            return can_derive_arbitrary(inner_ty, kani_any_def, ty_arbitrary_cache);
        } else {
            return implements_arbitrary(inner_ty, kani_any_def, ty_arbitrary_cache);
        }
    }

    let kani_any_body =
        Instance::resolve(kani_any_def, &GenericArgs(vec![GenericArgKind::Type(ty)]))
            .unwrap()
            .body()
            .unwrap();

    for bb in kani_any_body.blocks.iter() {
        let TerminatorKind::Call { func, .. } = &bb.terminator.kind else {
            continue;
        };
        if let TyKind::RigidTy(RigidTy::FnDef(def, args)) =
            func.ty(kani_any_body.arg_locals()).unwrap().kind()
        {
            let res = Instance::resolve(def, &args).is_ok();
            ty_arbitrary_cache.insert(ty, res);
            return res;
        }
    }
    false
}

/// Is `ty` a struct or enum whose fields/variants implement Arbitrary, or a reference to such a
/// type?
fn can_derive_arbitrary(
    ty: Ty,
    kani_any_def: FnDef,
    ty_arbitrary_cache: &mut FxHashMap<Ty, bool>,
) -> bool {
    let mut variants_can_derive = |def: AdtDef, args: GenericArgs| {
        for variant in def.variants_iter() {
            let fields = variant.fields();
            let mut fields_impl_arbitrary = true;
            for ty in fields.iter().map(|field| field.ty_with_args(&args)) {
                if let TyKind::RigidTy(RigidTy::Adt(..)) = ty.kind() {
                    fields_impl_arbitrary &=
                        can_derive_arbitrary(ty, kani_any_def, ty_arbitrary_cache);
                } else {
                    fields_impl_arbitrary &=
                        implements_arbitrary(ty, kani_any_def, ty_arbitrary_cache);
                }
            }
            if !fields_impl_arbitrary {
                return false;
            }
        }
        true
    };

    if let TyKind::RigidTy(RigidTy::Adt(def, args)) = ty.kind() {
        for arg in &args.0 {
            if let GenericArgKind::Lifetime(..) = arg {
                return false;
            }
        }

        match def.kind() {
            AdtKind::Enum => {
                // Enums with no variants cannot be instantiated
                if def.num_variants() == 0 {
                    return false;
                }
                variants_can_derive(def, args)
            }
            AdtKind::Struct => variants_can_derive(def, args),
            AdtKind::Union => false,
        }
    } else if let TyKind::RigidTy(RigidTy::Ref(_, inner_ty, _)) = ty.kind() {
        can_derive_arbitrary(inner_ty, kani_any_def, ty_arbitrary_cache)
    } else {
        false
    }
}
