// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! This module contains code that are backend agnostic. For example, MIR analysis
//! and transformations.

use std::collections::HashSet;

use crate::kani_queries::QueryDb;
use rustc_data_structures::fx::FxHashMap;
use rustc_hir::{def::DefKind, def_id::DefId as InternalDefId, def_id::LOCAL_CRATE};
use rustc_middle::ty::TyCtxt;
use rustc_public::mir::mono::{Instance, MonoItem};
use rustc_public::mir::{Mutability, TerminatorKind};
use rustc_public::rustc_internal;
use rustc_public::ty::{
    AdtDef, AdtKind, FnDef, GenericArgKind, GenericArgs, RigidTy, Span as SpanStable, Ty, TyConst,
    TyKind,
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
pub mod codegen_order;
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
            // The harness owns the referent's storage, so a top-level reference argument is
            // supported whenever its pointee is: prefer the pointee's own `Arbitrary` impl (which
            // may exist even for a type containing references), falling back to synthesizing one.
            return implements_arbitrary(inner_ty, kani_any_def, ty_arbitrary_cache)
                || can_derive_arbitrary(inner_ty, kani_any_def, ty_arbitrary_cache);
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

/// Inspect an `assume_safe::<T>()` (c.f. `KaniModel::AssumeSafe`) instantiation to determine if
/// `T: Invariant`. The model looks like:
/// ```rust
/// fn assume_safe<T: Invariant>(value: T) -> T {
///   kani::assume(value.is_safe());
///   value
/// }
/// ```
/// So we iterate over the call terminators in its body (`<T as Invariant>::is_safe` and
/// `kani::assume`) and try to resolve them to Instances.
/// `T` implements `Invariant` iff every callee resolves successfully (only the `is_safe` call
/// can fail to resolve, and it does iff `T` does not implement `Invariant`).
fn implements_invariant(
    ty: Ty,
    assume_safe_def: FnDef,
    ty_invariant_cache: &mut FxHashMap<Ty, bool>,
) -> bool {
    if let Some(v) = ty_invariant_cache.get(&ty) {
        return *v;
    }

    if ty.kind().rigid().is_none() {
        return false;
    }

    let assume_safe_body =
        Instance::resolve(assume_safe_def, &GenericArgs(vec![GenericArgKind::Type(ty)]))
            .unwrap()
            .body()
            .unwrap();

    let res = assume_safe_body.blocks.iter().all(|bb| {
        let TerminatorKind::Call { func, .. } = &bb.terminator.kind else {
            return true;
        };
        if let TyKind::RigidTy(RigidTy::FnDef(def, args)) =
            func.ty(assume_safe_body.arg_locals()).unwrap().kind()
        {
            Instance::resolve(def, &args).is_ok()
        } else {
            true
        }
    });
    ty_invariant_cache.insert(ty, res);
    res
}

/// Whether `ty` is statically sized. Nondeterministic-value generation (and the resolution
/// checks probing for it) must not instantiate generic models with unsized types: apart from
/// being ungeneratable, this can crash constant evaluation during body retrieval.
fn ty_is_sized(tcx: TyCtxt, ty: Ty) -> bool {
    rustc_internal::internal(tcx, ty)
        .is_sized(*tcx.at(rustc_span::DUMMY_SP), rustc_middle::ty::TypingEnv::fully_monomorphized())
}

/// The smart-pointer types for which automatic harnesses can generate nondeterministic values
/// via dedicated (optional) models, c.f. `KaniModel::is_optional`.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum SmartPointerKind {
    Arc,
    Box,
    Rc,
}

/// If `ty` is `Box<T>`, `Rc<T>`, or `Arc<T>`, return the pointer kind and the pointee type `T`.
/// Note that `Box<T, A>`/`Rc<T, A>`/`Arc<T, A>` with a non-default allocator are not
/// recognized, and unsized pointees (e.g. `Box<[T]>`) are rejected by the caller's
/// Arbitrary check on the pointee.
fn smart_pointer_pointee(tcx: TyCtxt, ty: Ty) -> Option<(SmartPointerKind, Ty)> {
    let TyKind::RigidTy(RigidTy::Adt(def, args)) = ty.kind() else {
        return None;
    };

    let kind = if def.is_box() {
        SmartPointerKind::Box
    } else {
        let def_id = rustc_internal::internal(tcx, def.def_id());
        if tcx.get_diagnostic_item(rustc_span::sym::Rc) == Some(def_id) {
            SmartPointerKind::Rc
        } else if tcx.get_diagnostic_item(rustc_span::sym::Arc) == Some(def_id) {
            SmartPointerKind::Arc
        } else {
            return None;
        }
    };

    // The first generic argument is the pointee. (The remaining argument is the allocator,
    // which is validated by the model return-type check in `smart_pointer_model_instance`.)
    let pointee = args.0.iter().find_map(|arg| match arg {
        GenericArgKind::Type(ty) => Some(*ty),
        _ => None,
    })?;
    if !ty_is_sized(tcx, pointee) {
        return None;
    }
    Some((kind, pointee))
}

/// If `ty` is a supported smart-pointer type whose generation model is available, resolve the
/// model for the pointee and return it together with the pointee type. Returns `None` if the
/// model's return type does not match `ty` exactly (e.g. for a non-default allocator).
fn smart_pointer_model_instance(
    tcx: TyCtxt,
    ty: Ty,
    smart_pointer_models: &SmartPointerModels,
) -> Option<(Instance, Ty)> {
    let (kind, pointee) = smart_pointer_pointee(tcx, ty)?;
    let model = smart_pointer_models.for_kind(kind)?;
    let instance =
        Instance::resolve(model, &GenericArgs(vec![GenericArgKind::Type(pointee)])).ok()?;
    let TyKind::RigidTy(RigidTy::FnDef(..)) = instance.ty().kind() else {
        return None;
    };
    let ret_ty = instance.ty().kind().fn_sig()?.skip_binder().output();
    (ret_ty == ty).then_some((instance, pointee))
}

/// The (optional) smart-pointer generation models, c.f. `smart_pointer_pointee`.
/// `None` entries mean the model is unavailable (e.g. in the `no_core` flow, which has no
/// `alloc`), in which case the corresponding smart-pointer types are not supported.
#[derive(Copy, Clone, Debug)]
pub struct SmartPointerModels {
    pub any_arc: Option<FnDef>,
    pub any_box: Option<FnDef>,
    pub any_rc: Option<FnDef>,
}

impl SmartPointerModels {
    pub fn from_kani_functions(
        kani_fns: &std::collections::HashMap<
            crate::kani_middle::kani_functions::KaniFunction,
            FnDef,
        >,
    ) -> Self {
        use crate::kani_middle::kani_functions::KaniModel;
        SmartPointerModels {
            any_arc: kani_fns.get(&KaniModel::AnyArc.into()).copied(),
            any_box: kani_fns.get(&KaniModel::AnyBox.into()).copied(),
            any_rc: kani_fns.get(&KaniModel::AnyRc.into()).copied(),
        }
    }

    pub fn for_kind(&self, kind: SmartPointerKind) -> Option<FnDef> {
        match kind {
            SmartPointerKind::Arc => self.any_arc,
            SmartPointerKind::Box => self.any_box,
            SmartPointerKind::Rc => self.any_rc,
        }
    }
}

/// Inspect a `kani::bounded_any::<T, N>()` (c.f. `KaniModel::BoundedAny`) instantiation to
/// determine if `T: BoundedArbitrary`. The model looks like:
/// ```rust
/// fn bounded_any<T: BoundedArbitrary, const N: usize>() -> T {
///   T::bounded_any::<N>()
/// }
/// ```
/// So we select the terminator that calls `T::bounded_any::<N>()`, then try to resolve it to an
/// Instance; `T` implements `BoundedArbitrary` iff we successfully resolve the Instance
/// (mirroring `implements_arbitrary`).
fn implements_bounded_arbitrary(tcx: TyCtxt, ty: Ty, kani_bounded_any_def: FnDef) -> bool {
    if ty.kind().rigid().is_none() || !ty_is_sized(tcx, ty) {
        return false;
    }

    let args = GenericArgs(vec![
        GenericArgKind::Type(ty),
        GenericArgKind::Const(TyConst::try_from_target_usize(1).unwrap()),
    ]);
    let Ok(instance) = Instance::resolve(kani_bounded_any_def, &args) else {
        return false;
    };
    let Some(body) = instance.body() else {
        return false;
    };

    for bb in body.blocks.iter() {
        let TerminatorKind::Call { func, .. } = &bb.terminator.kind else {
            continue;
        };
        if let TyKind::RigidTy(RigidTy::FnDef(def, args)) =
            func.ty(body.arg_locals()).unwrap().kind()
        {
            return Instance::resolve(def, &args).is_ok();
        }
    }
    false
}

/// The formatting traits whose implementations automatic harnesses can verify via dedicated
/// models (c.f. `KaniModel::CheckDebugFmt`/`CheckDisplayFmt`).
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum FmtTrait {
    Debug,
    Display,
}

/// If `instance` is the `fmt` method of a `Debug` or `Display` implementation, return the
/// trait and the implementing (self) type. Such methods take a `&mut Formatter` argument that
/// cannot be generated nondeterministically; instead, the generated harness formats a
/// nondeterministic value of the self type into a discarding sink, which exercises `fmt`
/// through the core formatting machinery with a real `Formatter`.
///
/// Both the eligibility check (`automatic_harness_partition`) and the harness generation
/// (`AutomaticHarnessPass`) go through this function, so that they cannot disagree on whether a
/// given function is handled by the formatting models.
fn fmt_impl_self_ty(tcx: TyCtxt, instance: Instance) -> Option<(FmtTrait, Ty)> {
    let def_id = rustc_internal::internal(tcx, instance.def.def_id());

    // A `fmt` method carrying a function contract is out of scope: the automatic *contract*
    // harness calls the function directly (and would therefore need a `Formatter`), and the
    // formatting models reach `fmt` through a function pointer, where contract dispatch does
    // not apply. Bail out so that such a function is reported as skipped for its
    // `&mut Formatter` argument, just like any other function we cannot call.
    if KaniAttributes::for_item(tcx, def_id).has_contract() {
        return None;
    }

    let impl_def_id = tcx.trait_impl_of_assoc(def_id)?;
    let trait_def_id = tcx.impl_trait_ref(impl_def_id).skip_binder().def_id;

    let fmt_trait = if Some(trait_def_id) == tcx.get_diagnostic_item(rustc_span::sym::Debug) {
        FmtTrait::Debug
    } else if Some(trait_def_id) == tcx.get_diagnostic_item(rustc_span::sym::Display) {
        FmtTrait::Display
    } else {
        return None;
    };

    // The `fmt` method's first input is `&Self`; peel the reference to obtain the
    // (monomorphic) self type.
    let sig = instance.ty().kind().fn_sig()?.skip_binder();
    let TyKind::RigidTy(RigidTy::Ref(_, self_ty, _)) = sig.inputs().first()?.kind() else {
        return None;
    };
    Some((fmt_trait, self_ty))
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
                    // Prefer the field type's own `Arbitrary` implementation: a hand-written
                    // impl can exist even for a type that itself contains references (and so is
                    // not compiler-derivable), and the synthesized `any()` would call it via
                    // `kani::any::<Field>()`. Only fall back to synthesizing one if it has none.
                    fields_impl_arbitrary &=
                        implements_arbitrary(ty, kani_any_def, ty_arbitrary_cache)
                            || can_derive_arbitrary(ty, kani_any_def, ty_arbitrary_cache);
                } else if let TyKind::RigidTy(RigidTy::Ref(..)) = ty.kind() {
                    // A bare reference *field* cannot be synthesized: the storage for the referent
                    // would live inside the synthesized `any()` body and dangle once it returns,
                    // and (unlike an ADT field) a reference type cannot carry a hand-written
                    // `Arbitrary` impl (orphan rule). (Only `&'static` fields reach this point:
                    // reference fields with a lifetime parameter make the ADT's generic arguments
                    // contain a lifetime, which is rejected below.)
                    // Note that this differs from *top-level argument* references, for which
                    // the harness itself owns the storage.
                    fields_impl_arbitrary = false;
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

/// How an automatic harness can generate a nondeterministic value of a given argument type,
/// c.f. `autoharness_supported_arg_ty`.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ArgSupport {
    /// An unbounded nondeterministic value, via (implemented or compiler-derived) `Arbitrary`.
    Arbitrary,
    /// A *bounded* nondeterministic value; verification results only hold up to the bound.
    /// Only used if the user passed `--bounded-arguments`.
    Bounded,
    /// The type is not supported.
    Unsupported,
}

/// Determine how an automatic harness can generate a nondeterministic value of type `ty` for a
/// harness argument.
/// In addition to the types that implement or can derive `Arbitrary`, automatic harnesses support:
/// - raw pointer arguments, as long as the pointee type (after peeling all raw pointer layers)
///   implements or can derive `Arbitrary`: for those, the harness generates a pointer in a
///   nondeterministic allocation state (null, out of bounds, or valid), c.f. `KaniModel::AnyPtr`;
/// - slice references (`&[T]`/`&mut [T]`, provided `T` implements or can derive `Arbitrary`) and
///   string slices (`&str`): for those, the harness generates a slice of *bounded* nondeterministic
///   length backed by harness-local storage, c.f. `KaniModel::AnySliceRef` and
///   `KaniModel::AnyStrRef` (reported as [ArgSupport::Bounded]);
/// - types that implement `BoundedArbitrary` (e.g. `Vec<T>`, `String`, or user types deriving
///   it): the harness generates a bounded nondeterministic value via `KaniModel::BoundedAny`
///   (reported as [ArgSupport::Bounded]).
/// - `Box<T>`/`Rc<T>`/`Arc<T>` arguments whose pointee `T` implements or can derive `Arbitrary`,
///   provided the corresponding (optional) generation model is available, c.f.
///   `smart_pointer_model_instance`: a smart pointer to `T` covers exactly the values of `T`, so
///   these are *unbounded* (reported as [ArgSupport::Arbitrary]).
///
/// Note that raw pointers and slice/string references are only supported as immediate harness
/// arguments (raw pointers also through other raw pointers): such a type behind a reference or
/// inside an ADT remains unsupported, since the pointee/backing storage that the generated harness
/// allocates would not outlive the generated value.
fn autoharness_supported_arg_ty(
    tcx: TyCtxt,
    ty: Ty,
    kani_any_def: FnDef,
    kani_bounded_any_def: FnDef,
    smart_pointer_models: &SmartPointerModels,
    ty_arbitrary_cache: &mut FxHashMap<Ty, bool>,
) -> ArgSupport {
    let arbitrary_or_derive = |ty: Ty, cache: &mut FxHashMap<Ty, bool>| {
        if implements_arbitrary(ty, kani_any_def, cache)
            || can_derive_arbitrary(ty, kani_any_def, cache)
        {
            ArgSupport::Arbitrary
        } else {
            ArgSupport::Unsupported
        }
    };

    if let TyKind::RigidTy(RigidTy::RawPtr(inner_ty, _)) = ty.kind() {
        // A raw pointer is supported as long as its pointee is: propagate the pointee's verdict,
        // so a pointer to a bounded pointee (e.g. `*mut &[T]`) is itself reported as bounded.
        autoharness_supported_arg_ty(
            tcx,
            inner_ty,
            kani_any_def,
            kani_bounded_any_def,
            smart_pointer_models,
            ty_arbitrary_cache,
        )
    } else if let TyKind::RigidTy(RigidTy::Ref(_, inner_ty, inner_mutability)) = ty.kind() {
        match inner_ty.kind() {
            TyKind::RigidTy(RigidTy::Slice(elem_ty)) => {
                if arbitrary_or_derive(elem_ty, ty_arbitrary_cache) == ArgSupport::Arbitrary {
                    ArgSupport::Bounded
                } else {
                    ArgSupport::Unsupported
                }
            }
            // There is no way to obtain a `&mut str` from our nondeterministic byte storage
            // without breaking the UTF-8 safety invariant on writes, so only support `&str`.
            TyKind::RigidTy(RigidTy::Str) => {
                if inner_mutability == Mutability::Not {
                    ArgSupport::Bounded
                } else {
                    ArgSupport::Unsupported
                }
            }
            _ => arbitrary_or_derive(ty, ty_arbitrary_cache),
        }
    } else {
        if arbitrary_or_derive(ty, ty_arbitrary_cache) == ArgSupport::Arbitrary {
            ArgSupport::Arbitrary
        } else if smart_pointer_model_instance(tcx, ty, smart_pointer_models).is_some_and(
            |(_, pointee)| {
                arbitrary_or_derive(pointee, ty_arbitrary_cache) == ArgSupport::Arbitrary
            },
        ) {
            // A smart pointer to `T` covers exactly the values of `T`, so it is unbounded.
            ArgSupport::Arbitrary
        } else if implements_bounded_arbitrary(tcx, ty, kani_bounded_any_def) {
            ArgSupport::Bounded
        } else {
            ArgSupport::Unsupported
        }
    }
}
