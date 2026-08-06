// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! This module contains code that are backend agnostic. For example, MIR analysis
//! and transformations.

use std::collections::HashSet;

use crate::kani_queries::QueryDb;
use rustc_data_structures::fx::FxHashMap;
use rustc_hir::{def::DefKind, def_id::DefId as InternalDefId, def_id::LOCAL_CRATE};
use rustc_middle::ty::TyCtxt;
use rustc_public::mir::TerminatorKind;
use rustc_public::mir::mono::{Instance, MonoItem};
use rustc_public::rustc_internal;
use rustc_public::ty::{
    AdtDef, AdtKind, FnDef, GenericArgKind, GenericArgs, RigidTy, Span as SpanStable, Ty, TyKind,
};
use rustc_public::visitor::{Visitable, Visitor as TyVisitor};
use rustc_public::{CrateDef, DefId, local_crate};
use std::ops::ControlFlow;

use self::attributes::KaniAttributes;

/// Return an item's name for user-facing output and CBMC symbol pretty-names.
///
/// [`Instance::name`] returns the item's absolute path. As of
/// rust-lang/rust#149401 that path includes the crate name for *local* items
/// too (e.g. `my_crate::my_fn` instead of `my_fn`). Kani's user-facing output
/// ("Checking harness ...", "Verification failed for - ...") and the CBMC
/// symbol pretty-names ("in function ...") historically used the crate-relative
/// form, and tests and users rely on it, so strip the local crate prefix.
/// Non-local items (e.g. `std::...`) keep their fully-qualified names.
pub fn readable_name(instance: Instance) -> String {
    strip_local_crate_prefix(instance.name())
}

/// Strip the local crate name from an absolute item path, restoring the
/// crate-relative form Kani used before rust-lang/rust#149401. That change made
/// `def_path_str` prefix the local crate name at *every* local path component,
/// so it appears not just at the start (`my_crate::f`) but also inside
/// qualifiers and generic args (`<my_crate::T as my_crate::Tr>::m`,
/// `f::<my_crate::T>`). Remove `<crate>::` at each path-component boundary
/// (start of string or after a non-identifier char) so these become `f`,
/// `<T as Tr>::m`, `f::<T>`. Non-local paths (which begin with a different
/// crate name) are unaffected.
pub fn strip_local_crate_prefix(name: String) -> String {
    let needle = format!("{}::", local_crate().name);
    if !name.contains(&needle) {
        return name;
    }
    let mut out = String::with_capacity(name.len());
    let mut rest = name.as_str();
    // The previous char in the input, used to decide whether a `<crate>::` here
    // is a crate-root *qualifier* (droppable) or a path *continuation* segment
    // (a module/item that happens to share the crate's name, which must be
    // kept). A qualifier appears at the start or after a type/path delimiter
    // (`<`, `,`, ` `, `&`, `*`, `(`, `[`, ...); a continuation appears after
    // `::`. So drop `<crate>::` only when the previous char is neither part of
    // an identifier nor `:`. After dropping, pretend the previous char is `:`
    // so an immediately following same-named segment is treated as a
    // continuation (e.g. `main::main::{closure#0}` -> `main::{closure#0}`).
    let mut prev: Option<char> = None;
    loop {
        let at_qualifier = match prev {
            None => true,
            Some(c) => !c.is_alphanumeric() && c != '_' && c != ':',
        };
        if at_qualifier && rest.starts_with(&needle) {
            rest = &rest[needle.len()..];
            prev = Some(':');
            continue;
        }
        let Some(ch) = rest.chars().next() else { break };
        out.push(ch);
        prev = Some(ch);
        rest = &rest[ch.len_utf8()..];
    }
    out
}

pub mod abi;
pub mod analysis;
pub mod attributes;
pub mod codegen_units;
pub mod coercion;
mod intrinsics;
pub mod kani_functions;
pub mod metadata;
pub mod mined_invariants;
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
    let mut all_stub_verified_targets = FxHashMap::default();
    let mut all_contract_targets = HashSet::new();

    for item in tcx.hir_free_items() {
        let def_id = item.owner_id.def_id.to_def_id();
        let (stub_verified_targets, contract_targets) =
            KaniAttributes::for_item(tcx, def_id).check_attributes();
        all_stub_verified_targets.extend(stub_verified_targets);
        all_contract_targets.extend(contract_targets);

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

    // Validate that all stub_verified targets have corresponding proof_for_contract harnesses
    for (stub_verified_target, span) in all_stub_verified_targets {
        if !all_contract_targets.contains(&stub_verified_target) {
            tcx.dcx().struct_span_err(
                span,
                format!(
                    "stub verified target `{}` does not have a corresponding `#[proof_for_contract]` harness",
                    strip_local_crate_prefix(stub_verified_target.name())
                ),
            ).with_help("verified stubs are meant to be sound abstractions for a function's behavior, so Kani enforces that proofs exist for the stub's contract")
            .emit();
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
    let reachable_functions: HashSet<DefId> = items
        .iter()
        .filter_map(|i| match i {
            MonoItem::Fn(instance) => Some(instance.def.def_id()),
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
            // Check whether all `proof_for_contract` targets are reachable
            attributes.check_proof_for_contract_reachability(&reachable_functions);
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

/// Whether generating a value of `ty` (under `--constructor-args`) would use constructor-based
/// generation for some ADT reachable in `ty`'s type tree: an ADT with a private field and a
/// viable public constructor. Used to mark such harnesses "(ctor)" in reports, since their
/// verification results only cover constructor-reachable values.
pub fn uses_ctor_generation(
    tcx: TyCtxt,
    ty: Ty,
    kani_any_def: FnDef,
    kani_assert_def: FnDef,
    ty_arbitrary_cache: &mut FxHashMap<Ty, bool>,
    visited: &mut Vec<Ty>,
) -> bool {
    if visited.contains(&ty) || visited.len() > 32 {
        return false;
    }
    visited.push(ty);
    match ty.kind() {
        TyKind::RigidTy(RigidTy::Ref(_, inner, _)) | TyKind::RigidTy(RigidTy::RawPtr(inner, _)) => {
            uses_ctor_generation(
                tcx,
                inner,
                kani_any_def,
                kani_assert_def,
                ty_arbitrary_cache,
                visited,
            )
        }
        TyKind::RigidTy(RigidTy::Array(inner, _)) | TyKind::RigidTy(RigidTy::Slice(inner)) => {
            uses_ctor_generation(
                tcx,
                inner,
                kani_any_def,
                kani_assert_def,
                ty_arbitrary_cache,
                visited,
            )
        }
        TyKind::RigidTy(RigidTy::Tuple(elems)) => elems.iter().any(|elem| {
            uses_ctor_generation(
                tcx,
                *elem,
                kani_any_def,
                kani_assert_def,
                ty_arbitrary_cache,
                visited,
            )
        }),
        TyKind::RigidTy(RigidTy::Adt(def, args)) => {
            // Hand-written Arbitrary implementations take precedence over ctor generation
            // in the transform (it only rewrites unresolvable kani::any calls).
            if implements_arbitrary_directly(ty, kani_any_def) {
                return false;
            }
            if def.kind() == AdtKind::Struct
                && adt_has_private_field_check(tcx, def)
                && (find_unchecked_constructor(tcx, ty, kani_any_def, ty_arbitrary_cache).is_some()
                    || find_arbitrary_constructor(tcx, ty, kani_any_def, ty_arbitrary_cache)
                        .is_some())
            {
                return true;
            }
            // Mined-invariant assumptions are heuristic filters like constructor-based
            // generation, so harnesses using them carry the same marker.
            if !mined_invariants::mine_self_assert_conjuncts(tcx, ty, kani_assert_def).is_empty() {
                return true;
            }
            def.variants_iter().any(|variant| {
                variant.fields().iter().any(|field| {
                    uses_ctor_generation(
                        tcx,
                        field.ty_with_args(&args),
                        kani_any_def,
                        kani_assert_def,
                        ty_arbitrary_cache,
                        visited,
                    )
                })
            }) || args.0.iter().any(|arg| match arg {
                GenericArgKind::Type(t) => uses_ctor_generation(
                    tcx,
                    *t,
                    kani_any_def,
                    kani_assert_def,
                    ty_arbitrary_cache,
                    visited,
                ),
                _ => false,
            })
        }
        _ => false,
    }
}

/// Whether the ADT has at least one non-public field (in any variant).
pub fn adt_has_private_field_check(tcx: TyCtxt, def: AdtDef) -> bool {
    let did = rustc_internal::internal(tcx, def.def_id());
    tcx.adt_def(did).all_fields().any(|field| !tcx.visibility(field.did).is_public())
}

/// Whether `ty` has a resolvable `<ty as Arbitrary>::any` (a hand-written or derived source
/// implementation), without considering compiler-side derivation. Mirrors the resolvability
/// test in `implements_arbitrary`: `kani::any::<T>` itself always resolves (it is a concrete
/// generic function); what distinguishes a source implementation is whether the `T::any()`
/// call in its body resolves.
fn implements_arbitrary_directly(ty: Ty, kani_any_def: FnDef) -> bool {
    let Ok(inst) = Instance::resolve(kani_any_def, &GenericArgs(vec![GenericArgKind::Type(ty)]))
    else {
        return false;
    };
    let Some(kani_any_body) = inst.body() else { return false };
    for bb in kani_any_body.blocks.iter() {
        let TerminatorKind::Call { func, .. } = &bb.terminator.kind else {
            continue;
        };
        if let TyKind::RigidTy(RigidTy::FnDef(def, args)) =
            func.ty(kani_any_body.arg_locals()).unwrap().kind()
        {
            return Instance::resolve(def, &args).is_ok();
        }
    }
    false
}

/// The outcome of searching for a viable public constructor for a type without an Arbitrary
/// implementation (`--constructor-args`): the constructor's instance, and how its return value
/// wraps `Self` (directly, or inside `Option`/`Result`, in which case generated harnesses
/// assume success).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CtorReturn {
    Direct,
    OptionOf,
    ResultOf,
}

/// Search `ty`'s inherent impls for an assert-guarded *representation constructor*: an
/// associated function returning `Self` directly whose preconditions are stated as
/// (debug_)asserts rather than validated returns — typically `unsafe`, doc-hidden or
/// `_unchecked`-named builders exported for macro use (e.g. time's `Date::from_parts`).
/// Under `--constructor-args`, such a constructor is inlined with panic paths converted to
/// assumptions (c.f. `automatic::inline_with_assumed_panics`), so its own assertions filter
/// the nondeterministic arguments down to exactly the values the crate considers valid.
/// Visibility is irrelevant (the body is inlined, not called). Prefers more arguments over
/// fewer; ties broken by definition order.
pub fn find_unchecked_constructor(
    tcx: TyCtxt,
    ty: Ty,
    kani_any_def: FnDef,
    ty_arbitrary_cache: &mut FxHashMap<Ty, bool>,
) -> Option<Instance> {
    let TyKind::RigidTy(RigidTy::Adt(adt_def, ref adt_args)) = ty.kind() else {
        return None;
    };
    let adt_did = rustc_internal::internal(tcx, adt_def.def_id());
    let mut best: Option<(Instance, usize)> = None;
    for &impl_did in tcx.inherent_impls(adt_did) {
        for &item in tcx.associated_item_def_ids(impl_did) {
            if !tcx.def_kind(item).is_fn_like() || tcx.associated_item(item).is_method() {
                continue;
            }
            if tcx
                .generics_of(item)
                .own_params
                .iter()
                .any(|p| !matches!(p.kind, rustc_middle::ty::GenericParamDefKind::Lifetime))
            {
                continue;
            }
            let Some(ctor_def) = to_fn_def(tcx, item) else { continue };
            // For generic ADTs (e.g. deranged's RangedI32<MIN, MAX>), instantiate the
            // constructor with the ADT's own generic arguments: for inherent impls whose
            // parameters mirror the type's, this is the correct substitution; when it is
            // not, resolution fails and the constructor is skipped.
            let Ok(instance) = Instance::resolve(ctor_def, adt_args) else {
                continue;
            };
            if !instance.has_body() {
                continue;
            }
            let TyKind::RigidTy(RigidTy::FnDef(..)) = instance.ty().kind() else { continue };
            let Some(binder) = instance.ty().kind().fn_sig() else { continue };
            let fn_sig = binder.skip_binder();
            if fn_sig.output() != ty {
                continue;
            }
            // The unchecked-builder heuristic: unsafe, doc-hidden, or *_unchecked-named.
            let name = tcx.item_name(item).to_string();
            let is_unchecked = fn_sig.safety == rustc_public::mir::Safety::Unsafe
                || tcx.is_doc_hidden(item)
                || name.contains("unchecked");
            if !is_unchecked {
                continue;
            }
            if fn_sig.inputs().is_empty()
                || !fn_sig
                    .inputs()
                    .iter()
                    .all(|input| implements_arbitrary(*input, kani_any_def, ty_arbitrary_cache))
            {
                continue;
            }
            let n_args = fn_sig.inputs().len();
            if best.as_ref().is_none_or(|(_, best_n)| n_args > *best_n) {
                best = Some((instance, n_args));
            }
        }
    }
    best.map(|(inst, _)| inst)
}

/// Search `ty`'s inherent impls for a public associated function usable as a constructor:
/// one that returns `Self`, `Option<Self>` or `Result<Self, E>`, takes no `self` argument,
/// has no remaining generic parameters of its own, and whose every argument implements (or
/// can derive) Arbitrary. Prefer `Self` over `Option<Self>` over `Result<Self, E>` returns
/// (fewer assumptions), and among equal shapes, prefer the constructor with the most
/// arguments (heuristically the least-constrained coverage of the value space); ties are
/// broken by definition order for determinism.
pub fn find_arbitrary_constructor(
    tcx: TyCtxt,
    ty: Ty,
    kani_any_def: FnDef,
    ty_arbitrary_cache: &mut FxHashMap<Ty, bool>,
) -> Option<(Instance, CtorReturn)> {
    let TyKind::RigidTy(RigidTy::Adt(adt_def, ref adt_args)) = ty.kind() else {
        return None;
    };
    let adt_did = rustc_internal::internal(tcx, adt_def.def_id());
    let mut best: Option<(Instance, CtorReturn, usize)> = None;
    for &impl_did in tcx.inherent_impls(adt_did) {
        for &item in tcx.associated_item_def_ids(impl_did) {
            if !tcx.def_kind(item).is_fn_like() || tcx.associated_item(item).is_method() {
                continue;
            }
            if !tcx.visibility(item).is_public() {
                continue;
            }
            // Exclude doc-hidden constructors: they are de-facto internal (commonly
            // `_unchecked` variants exported for macro use that assert their preconditions
            // instead of validating, e.g. time's `Date::__from_ordinal_date_unchecked`),
            // and calling them with nondeterministic arguments manufactures false alarms
            // in every harness that generates the type. Unsafe constructors are excluded
            // for the same reason: their preconditions are the caller's obligation.
            if tcx.is_doc_hidden(item) {
                continue;
            }
            // The constructor may only use the ADT's own generic parameters (inherited via
            // the impl); reject constructors introducing their own generics.
            if tcx
                .generics_of(item)
                .own_params
                .iter()
                .any(|p| !matches!(p.kind, rustc_middle::ty::GenericParamDefKind::Lifetime))
            {
                continue;
            }
            let Some(ctor_def) = to_fn_def(tcx, item) else { continue };
            // Instantiate the impl's generics with the ADT instantiation's arguments. For
            // phase 1, only support non-generic ADTs (no substitution needed).
            if !adt_args.0.is_empty() {
                continue;
            }
            let fn_sig = ctor_def.fn_sig().skip_binder();
            if fn_sig.safety == rustc_public::mir::Safety::Unsafe {
                continue;
            }
            // Zero-argument constructors produce a single value, which destroys the coverage
            // a nondeterministic harness is meant to provide, and is actively harmful for
            // environment-reading constructors (e.g. Instant::now() reaches clock_gettime,
            // which Kani does not support, failing every harness that generates the type).
            if fn_sig.inputs().is_empty() {
                continue;
            }
            let ret = fn_sig.output();
            let shape = if ret == ty {
                CtorReturn::Direct
            } else if let TyKind::RigidTy(RigidTy::Adt(wrap_def, wrap_args)) = ret.kind() {
                let name = wrap_def.name();
                let payload = wrap_args.0.first().and_then(|a| match a {
                    GenericArgKind::Type(t) => Some(*t),
                    _ => None,
                });
                if payload != Some(ty) {
                    continue;
                } else if name == "core::option::Option" || name == "std::option::Option" {
                    CtorReturn::OptionOf
                } else if name == "core::result::Result" || name == "std::result::Result" {
                    CtorReturn::ResultOf
                } else {
                    continue;
                }
            } else {
                continue;
            };
            // Every constructor argument must be plainly generatable (implements or derives
            // Arbitrary); constructor arguments do not get the argument-position extensions
            // (slices, smart pointers, nested constructors) in phase 1.
            if !fn_sig
                .inputs()
                .iter()
                .all(|input| implements_arbitrary(*input, kani_any_def, ty_arbitrary_cache))
            {
                continue;
            }
            let Ok(instance) = Instance::resolve(ctor_def, &GenericArgs(vec![])) else {
                continue;
            };
            if !instance.has_body() {
                continue;
            }
            let n_args = fn_sig.inputs().len();
            let better = match &best {
                None => true,
                Some((_, best_shape, best_n)) => {
                    (shape as u8, std::cmp::Reverse(n_args))
                        < (*best_shape as u8, std::cmp::Reverse(*best_n))
                }
            };
            if better {
                best = Some((instance, shape, n_args));
            }
        }
    }
    best.map(|(inst, shape, _)| (inst, shape))
}

/// Convert an internal DefId of a function-like item to a stable FnDef.
fn to_fn_def(tcx: TyCtxt, def_id: rustc_span::def_id::DefId) -> Option<FnDef> {
    let ty = rustc_internal::stable(tcx.type_of(def_id).instantiate_identity());
    match ty.kind() {
        TyKind::RigidTy(RigidTy::FnDef(def, _)) => Some(def),
        _ => None,
    }
}

/// If `ty` is `Vec<T>` with the default allocator, return `T`.
pub fn vec_elem_ty(ty: Ty) -> Option<Ty> {
    let TyKind::RigidTy(RigidTy::Adt(def, ref args)) = ty.kind() else { return None };
    let name = def.name();
    if name != "std::vec::Vec" && name != "alloc::vec::Vec" {
        return None;
    }
    // Vec<T, A = Global>: only the default allocator is supported (the model allocates via
    // the global allocator). The allocator parameter is defaulted, so a crate naming a
    // custom allocator produces a second type argument != Global.
    let mut ty_args = args.0.iter().filter_map(|a| match a {
        GenericArgKind::Type(t) => Some(*t),
        _ => None,
    });
    let elem = ty_args.next()?;
    if let Some(alloc_ty) = ty_args.next()
        && !alloc_ty.to_string().contains("Global")
    {
        return None;
    }
    Some(elem)
}

/// Whether `&[T]` arguments with this element type qualify for *unbounded* generation
/// (`KaniModel::AnySliceRefUnbounded`): raw nondeterministic memory must be a sound AND
/// complete model of the element's values *without any validity assumption*, i.e. every bit
/// pattern must be a valid element. This holds exactly for the primitive integer and float
/// types.
///
/// Types with validity constraints (bool, char, NonZero, ranged newtypes) are excluded even
/// though the `SliceValidityAssume` hook can express byte-width niche constraints: CBMC's
/// default (SAT) backend only instantiates quantifiers with *constant* bounds, and silently
/// degrades symbolic-bound quantifiers to unconstrained free variables
/// (`boolbvt::finish_eager_conversion_quantifiers` -> `conversion_failed`), which would make
/// the validity assumption vacuous. SMT backends (e.g. `--solver z3`) handle the quantified
/// assumption, including multi-byte elements; routing niched element types here can be
/// revisited when CBMC's SAT backend learns symbolic-bound instantiation or Kani selects
/// backends per harness.
pub fn slice_elem_unbounded_ok(_tcx: TyCtxt, ty: Ty) -> bool {
    matches!(
        ty.kind(),
        TyKind::RigidTy(RigidTy::Int(_))
            | TyKind::RigidTy(RigidTy::Uint(_))
            | TyKind::RigidTy(RigidTy::Float(_))
    )
}

/// The bit width of a scalar-ABI type (integer primitives and enum discriminant types).
pub fn scalar_width_bits(tcx: TyCtxt, ty: Ty) -> Option<u64> {
    use rustc_abi::{BackendRepr, Primitive, Scalar};
    let internal_ty = rustc_internal::internal(tcx, ty);
    let layout = tcx
        .layout_of(rustc_middle::ty::TypingEnv::fully_monomorphized().as_query_input(internal_ty))
        .ok()?;
    let BackendRepr::Scalar(scalar) = layout.backend_repr else { return None };
    let Scalar::Initialized { value, .. } = scalar else { return None };
    let Primitive::Int(int, _) = value else { return None };
    Some(int.size().bits())
}

/// The niche constraint of a scalar-ABI type: the width of the scalar in bits, and the
/// (possibly wrapping) inclusive range of valid bit patterns.
/// Returns None for non-scalar ABIs, pointer/float scalars, and scalars whose valid range
/// covers every bit pattern.
///
/// Rationale: a layout niche is a language-level validity invariant (rustc packs enum
/// variants into the invalid patterns), so a synthesized `kani::any` body must not produce
/// values outside it -- they are as invalid as a `bool` holding 3. Assuming the range is
/// therefore sound by construction and requires no reporting caveat.
pub struct ScalarNiche {
    /// Width of the scalar in bits (8, 16, 32, 64 or 128).
    pub bits: u64,
    /// Inclusive start of the valid range (bit pattern).
    pub start: u128,
    /// Inclusive end of the valid range (bit pattern). If `end < start`, the range wraps.
    pub end: u128,
}

pub fn scalar_niche(tcx: TyCtxt, ty: Ty) -> Option<ScalarNiche> {
    use rustc_abi::{BackendRepr, Primitive, Scalar};
    let internal_ty = rustc_internal::internal(tcx, ty);
    let layout = tcx
        .layout_of(rustc_middle::ty::TypingEnv::fully_monomorphized().as_query_input(internal_ty))
        .ok()?;
    let BackendRepr::Scalar(scalar) = layout.backend_repr else { return None };
    let Scalar::Initialized { value, valid_range } = scalar else { return None };
    let Primitive::Int(int, _signed) = value else { return None };
    let bits = int.size().bits();
    let full = if bits == 128 { u128::MAX } else { (1u128 << bits) - 1 };
    if valid_range.start == 0 && valid_range.end == full {
        return None;
    }
    Some(ScalarNiche { bits, start: valid_range.start, end: valid_range.end })
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
