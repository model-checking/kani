// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This module contains two passes:
//! 1. `AutomaticHarnessPass`, which transforms the body of an automatic harness to verify a function.
//! 2. `AutomaticArbitraryPass`, which creates `T::any()` implementations for `T`s that do not implement Arbitrary in source code,
//!    but we have determined can derive it.

use crate::args::ReachabilityType;
use crate::kani_middle::attributes::KaniAttributes;
use crate::kani_middle::codegen_units::CodegenUnit;
use crate::kani_middle::kani_functions::{KaniHook, KaniIntrinsic, KaniModel};
use crate::kani_middle::transform::body::{InsertPosition, MutableBody, SourceInstruction};
use crate::kani_middle::transform::{TransformPass, TransformationType};
use crate::kani_middle::{
    FmtTrait, SmartPointerModels, can_derive_arbitrary, fmt_impl_self_ty, implements_arbitrary,
    implements_invariant, smart_pointer_model_instance,
};
use crate::kani_queries::QueryDb;
use rustc_data_structures::fx::FxHashMap;
use rustc_middle::ty::TyCtxt;
use rustc_public::CrateDef;
use rustc_public::mir::mono::Instance;
use rustc_public::mir::{
    AggregateKind, BasicBlockIdx, Body, BorrowKind, CastKind, Local, MutBorrowKind, Mutability,
    Operand, Place, ProjectionElem, Rvalue, SwitchTargets, Terminator, TerminatorKind,
};
use rustc_public::ty::{
    AdtDef, AdtKind, FnDef, GenericArgKind, GenericArgs, Region, RegionKind, RigidTy, Ty, TyConst,
    TyKind, UintTy, VariantDef, VariantIdx,
};
use rustc_public_bridge::IndexedVal;
use tracing::debug;

/// The Kani model functions used to construct nondeterministic values.
/// These are always needed together, so they are grouped to be threaded through
/// `call_kani_any_for_ty` and its callers as a single unit.
#[derive(Debug, Clone, Copy)]
struct AnyModels {
    /// The FnDef of KaniModel::Any
    kani_any: FnDef,
    /// The FnDef of KaniModel::AnyPtr
    kani_any_ptr: FnDef,
    /// The FnDef of KaniModel::AnySliceRef
    kani_any_slice_ref: FnDef,
    /// The FnDef of KaniModel::AnyStrRef
    kani_any_str_ref: FnDef,
    /// The FnDef of KaniModel::AssumeSafe
    kani_assume_safe: FnDef,
    /// The FnDef of KaniModel::BoundedAny
    kani_bounded_any: FnDef,
    /// The (optional) smart-pointer generation models (`Box`/`Rc`/`Arc`).
    smart_pointer_models: SmartPointerModels,
}

impl AnyModels {
    fn new(query_db: &QueryDb) -> Self {
        let kani_fns = query_db.kani_functions();
        Self {
            kani_any: *kani_fns.get(&KaniModel::Any.into()).unwrap(),
            kani_any_ptr: *kani_fns.get(&KaniModel::AnyPtr.into()).unwrap(),
            kani_any_slice_ref: *kani_fns.get(&KaniModel::AnySliceRef.into()).unwrap(),
            kani_any_str_ref: *kani_fns.get(&KaniModel::AnyStrRef.into()).unwrap(),
            kani_assume_safe: *kani_fns.get(&KaniModel::AssumeSafe.into()).unwrap(),
            kani_bounded_any: *kani_fns.get(&KaniModel::BoundedAny.into()).unwrap(),
            smart_pointer_models: SmartPointerModels::from_kani_functions(kani_fns),
        }
    }
}

/// Generate `T::any()` implementations for `T`s that do not implement Arbitrary in source code.
/// Currently limited to structs and enums.
#[derive(Debug, Clone)]
pub struct AutomaticArbitraryPass {
    /// The Kani model functions used to construct nondeterministic values.
    models: AnyModels,
}

impl AutomaticArbitraryPass {
    pub fn new(_unit: &CodegenUnit, query_db: &QueryDb) -> Self {
        Self { models: AnyModels::new(query_db) }
    }
}

impl TransformPass for AutomaticArbitraryPass {
    fn transformation_type() -> TransformationType
    where
        Self: Sized,
    {
        TransformationType::Stubbing
    }

    fn is_enabled(&self, query_db: &QueryDb) -> bool
    where
        Self: Sized,
    {
        matches!(query_db.args().reachability_analysis, ReachabilityType::AllFns)
    }

    /// Transform the body of a kani::any::<T>() call if `T` does not implement `Arbitrary`.
    /// This occurs if an automatic harness calls kani::any() for a type that `automatic_harness_partition` determined can derive Arbitrary.
    /// The default implementation for `kani::any()` (c.f. kani_core::kani_intrinsics) is:
    /// ```ignore
    /// pub fn any<T: Arbitrary>() -> T {
    ///   T::any()
    /// }
    /// ```
    /// We need to overwrite this implementation because `T` doesn't implement `Arbitrary`, so trying to call `T::any()` will fail.
    /// Instead, we inline the body of what `T::any()` would be if it existed.
    /// For example:
    /// ```ignore
    /// enum Foo {
    ///   Variant1,
    ///   Variant2,
    /// }
    /// ```
    /// we replace the body:
    /// ```ignore
    /// pub fn any() -> Foo {
    ///   Foo::any() // doesn't exist, must replace
    /// }
    /// ```
    /// so that instead, we have:
    /// ```ignore
    /// pub fn any() -> Foo {
    ///   match kani::any() {
    ///     0 => Foo::Variant1,
    ///     _ => Foo::Variant2,
    ///   }
    /// }
    /// ```
    /// We match the implementations that kani_macros::derive creates for structs and enums,
    /// so see that module for full documentation of what the generated bodies look like.
    fn transform(&mut self, tcx: TyCtxt, body: Body, instance: Instance) -> (bool, Body) {
        debug!(function=?instance.name(), "AutomaticArbitraryPass::transform");

        let unexpected_ty = |ty: &Ty| {
            panic!(
                "AutomaticArbitraryPass: should only find compiler-inserted kani::any() calls for structs or enums, found {ty}"
            )
        };

        if instance.def.def_id() != self.models.kani_any.def_id() {
            return (false, body);
        }

        // Get the `ty` we're calling `kani::any()` on
        let binding = instance.args();
        let ty = binding.0[0].expect_ty();

        if implements_arbitrary(*ty, self.models.kani_any, &mut FxHashMap::default()) {
            return (false, body);
        }

        if let TyKind::RigidTy(RigidTy::Adt(def, args)) = ty.kind() {
            match def.kind() {
                AdtKind::Enum => (true, self.generate_enum_body(tcx, def, args, body)),
                AdtKind::Struct => (true, self.generate_struct_body(tcx, def, args, body)),
                AdtKind::Union => unexpected_ty(ty),
            }
        } else {
            unexpected_ty(ty)
        }
    }
}

/// The maximum length for nondeterministic slices that automatic harnesses generate.
/// Verification results for functions taking `&[T]`/`&mut [T]` arguments are only valid up to
/// this bound. The value must stay below Kani's default unwinding bound (20), so that loops
/// iterating over such a slice can be fully unwound by default.
const AUTOHARNESS_SLICE_BOUND: u64 = 16;

/// The maximum length (in bytes) for nondeterministic strings that automatic harnesses
/// generate. Strings use a much smaller bound than slices: the generated string is the longest
/// valid-UTF-8 prefix of nondeterministic bytes, and reasoning about UTF-8 validity is
/// expensive. On top of that, a harness that decodes every `char` (e.g. `s.chars().count()`)
/// unwinds the decoding loop up to the default bound (20) over symbolic bytes, so the cost grows
/// steeply with the number of bytes: on a typical machine 8 bytes already exceed Kani's default
/// 60s harness timeout for such harnesses, while 4 stay comfortably within it (though a
/// char-decoding harness can still approach the timeout on slow machines, so callers that must
/// not time out should raise `--harness-timeout`).
const AUTOHARNESS_STR_BOUND: u64 = 4;

/// The bound for nondeterministic values of types that implement `BoundedArbitrary` (rather
/// than `Arbitrary`) that automatic harnesses generate, e.g. `Vec<T>` or `String`.
/// Verification results for functions with such arguments are only valid up to this bound.
/// This is smaller than the slice/str bounds since `BoundedArbitrary` values are heap
/// allocated, and for `String` additionally involve UTF-8 reasoning; a bound of 8 already
/// makes simple `String` harnesses exceed Kani's default 60s harness timeout.
const AUTOHARNESS_BOUNDED_ANY_BOUND: u64 = 4;

/// For raw pointer types, insert a call to the `KaniModel::AnyPtr` model instead, which generates
/// a pointer in a nondeterministic allocation state (null, out of bounds, or valid);
/// in the valid case, the pointer points to a nondeterministic value stored in a dedicated local
/// of `body`, which keeps it alive for as long as the transformed body executes.
/// For `&[T]`/`&mut [T]`/`&str`, insert calls to the `KaniModel::AnySliceRef`/`AnyStrRef`
/// models instead, which return a slice of nondeterministic length (bounded by
/// [AUTOHARNESS_SLICE_BOUND]) backed by a nondeterministic array stored in a dedicated local,
/// which stays alive for the entire harness.
/// If `ty` does not implement `Arbitrary` (and cannot derive it) but implements `BoundedArbitrary`
/// (e.g. `Vec<T>` or `String`), insert a call to the `KaniModel::BoundedAny` model, which returns
/// a *bounded* nondeterministic value (bounded by [AUTOHARNESS_BOUNDED_ANY_BOUND]).
/// If `ty` is an ADT that implements `Invariant`, additionally insert a call to the
/// `KaniModel::AssumeSafe` model (`kani_assume_safe`), which assumes that the nondeterministic
/// value respects the type's safety invariant, c.f.
/// <https://rust-lang.github.io/unsafe-code-guidelines/glossary.html#validity-and-safety-invariant>.
/// For `Box<T>`/`Rc<T>`/`Arc<T>` whose pointee only *can derive* Arbitrary (so the smart
/// pointer's blanket `Arbitrary` implementation is unresolvable), insert a call to the
/// corresponding smart-pointer model (`any_box`/`any_rc`/`any_arc`) instead; the model's internal
/// `kani::any::<T>()` call is replaced with the compiler-synthesized implementation by this pass.
/// Panics if `ty` does not implement Arbitrary or BoundedArbitrary and is not a supported smart
/// pointer (and is not a reference or raw pointer to such a type, or a reference to a slice or str
/// of such a type).
fn call_kani_any_for_ty(
    tcx: TyCtxt,
    models: AnyModels,
    body: &mut MutableBody,
    ty: Ty,
    mutability: Mutability,
    source: &mut SourceInstruction,
    invariant_cache: &mut FxHashMap<Ty, bool>,
) -> Local {
    if let TyKind::RigidTy(RigidTy::Ref(region, inner_ty, inner_mutability)) = ty.kind()
        && matches!(
            inner_ty.kind(),
            TyKind::RigidTy(RigidTy::Slice(..)) | TyKind::RigidTy(RigidTy::Str)
        )
    {
        let is_str = matches!(inner_ty.kind(), TyKind::RigidTy(RigidTy::Str));
        let (elem_ty, model, model_args) = match inner_ty.kind() {
            TyKind::RigidTy(RigidTy::Slice(elem_ty)) => (
                elem_ty,
                models.kani_any_slice_ref,
                GenericArgs(vec![
                    GenericArgKind::Type(elem_ty),
                    GenericArgKind::Const(
                        TyConst::try_from_target_usize(AUTOHARNESS_SLICE_BOUND).unwrap(),
                    ),
                ]),
            ),
            TyKind::RigidTy(RigidTy::Str) => (
                Ty::unsigned_ty(UintTy::U8),
                models.kani_any_str_ref,
                GenericArgs(vec![GenericArgKind::Const(
                    TyConst::try_from_target_usize(AUTOHARNESS_STR_BOUND).unwrap(),
                )]),
            ),
            _ => unreachable!(),
        };

        // Generate the backing storage: a local holding a nondeterministic array of the
        // element type. Since it is a local of the harness body, it stays alive for the
        // entire harness. The array is built element-wise (rather than via
        // `kani::any::<[T; N]>()`) so that element types whose Arbitrary implementation the
        // compiler derives are supported: `<[T; N] as Arbitrary>::any` is unresolvable for
        // such `T`, whereas each element can be generated through the same path as any other
        // value of that type.
        let bound = if is_str { AUTOHARNESS_STR_BOUND } else { AUTOHARNESS_SLICE_BOUND };
        let storage_ty = Ty::try_new_array(elem_ty, bound).unwrap();
        let elem_lcls = (0..bound)
            .map(|_| {
                call_kani_any_for_ty(
                    tcx,
                    models,
                    body,
                    elem_ty,
                    Mutability::Not,
                    source,
                    invariant_cache,
                )
            })
            .collect::<Vec<_>>();
        let storage_lcl = body.new_local(storage_ty, source.span(body.blocks()), Mutability::Mut);
        body.assign_to(
            Place::from(storage_lcl),
            Rvalue::Aggregate(
                AggregateKind::Array(elem_ty),
                elem_lcls.into_iter().map(|lcl| Operand::Move(Place::from(lcl))).collect(),
            ),
            source,
            InsertPosition::Before,
        );

        // Pass a mutable reference to the storage to the model, which returns a slice of
        // nondeterministic (bounded) length.
        let storage_ref_ty = Ty::new_ref(region.clone(), storage_ty, Mutability::Mut);
        let storage_ref_lcl =
            body.new_local(storage_ref_ty, source.span(body.blocks()), Mutability::Not);
        body.assign_to(
            Place::from(storage_ref_lcl),
            Rvalue::Ref(
                region.clone(),
                BorrowKind::Mut { kind: MutBorrowKind::Default },
                storage_lcl.into(),
            ),
            source,
            InsertPosition::Before,
        );
        let model_inst = Instance::resolve(model, &model_args).unwrap();
        // For `&str`, the model already returns the shared-reference type (there is no
        // `&mut str` in practice); for slices it returns `&mut [T]`.
        let model_ret_ty =
            if is_str { ty } else { Ty::new_ref(region.clone(), inner_ty, Mutability::Mut) };
        let slice_lcl = body.new_local(model_ret_ty, source.span(body.blocks()), mutability);
        body.insert_call(
            &model_inst,
            source,
            InsertPosition::Before,
            vec![Operand::Move(Place::from(storage_ref_lcl))],
            Place::from(slice_lcl),
        );

        if inner_mutability == Mutability::Not && !is_str {
            // Reborrow the `&mut [T]` the model returned as `&[T]`.
            let shared_lcl = body.new_local(ty, source.span(body.blocks()), mutability);
            body.assign_to(
                Place::from(shared_lcl),
                Rvalue::Ref(
                    region,
                    BorrowKind::Shared,
                    Place { local: slice_lcl, projection: vec![ProjectionElem::Deref] },
                ),
                source,
                InsertPosition::Before,
            );
            shared_lcl
        } else {
            slice_lcl
        }
    } else if let TyKind::RigidTy(RigidTy::Ref(region, inner_ty, inner_mutability)) = ty.kind() {
        let inner_lcl = call_kani_any_for_ty(
            tcx,
            models,
            body,
            inner_ty,
            inner_mutability,
            source,
            invariant_cache,
        );
        let ref_lcl = body.new_local(ty, source.span(body.blocks()), mutability);
        let borrow_kind = if inner_mutability == Mutability::Not {
            BorrowKind::Shared
        } else {
            BorrowKind::Mut { kind: MutBorrowKind::Default }
        };
        body.assign_to(
            Place::from(ref_lcl),
            Rvalue::Ref(region, borrow_kind, Place::from(inner_lcl)),
            source,
            InsertPosition::Before,
        );
        ref_lcl
    } else if let TyKind::RigidTy(RigidTy::RawPtr(inner_ty, ptr_mutability)) = ty.kind() {
        // Generate the storage for the valid-pointer case: a local with a nondeterministic value
        // of the pointee type. Since it is a local of the body being transformed, it stays alive
        // for as long as that body executes.
        let storage_lcl = call_kani_any_for_ty(
            tcx,
            models,
            body,
            inner_ty,
            Mutability::Mut,
            source,
            invariant_cache,
        );

        // Pass a mutable reference to the storage to the AnyPtr model, which returns a `*mut T`
        // that is either null, out of bounds (one past the end), or pointing to the storage.
        let region = Region { kind: RegionKind::ReErased };
        let ref_ty = Ty::new_ref(region.clone(), inner_ty, Mutability::Mut);
        let ref_lcl = body.new_local(ref_ty, source.span(body.blocks()), Mutability::Not);
        body.assign_to(
            Place::from(ref_lcl),
            Rvalue::Ref(
                region,
                BorrowKind::Mut { kind: MutBorrowKind::Default },
                storage_lcl.into(),
            ),
            source,
            InsertPosition::Before,
        );
        let any_ptr_inst = Instance::resolve(
            models.kani_any_ptr,
            &GenericArgs(vec![GenericArgKind::Type(inner_ty)]),
        )
        .unwrap();
        let mut_ptr_ty = Ty::new_ptr(inner_ty, Mutability::Mut);
        let ptr_lcl = body.new_local(mut_ptr_ty, source.span(body.blocks()), mutability);
        body.insert_call(
            &any_ptr_inst,
            source,
            InsertPosition::Before,
            vec![Operand::Move(Place::from(ref_lcl))],
            Place::from(ptr_lcl),
        );

        // For `*const T` arguments, cast the `*mut T` the model returns.
        if ptr_mutability == Mutability::Not {
            let cast_lcl = body.new_local(ty, source.span(body.blocks()), mutability);
            body.assign_to(
                Place::from(cast_lcl),
                Rvalue::Cast(CastKind::PtrToPtr, Operand::Move(Place::from(ptr_lcl)), ty),
                source,
                InsertPosition::Before,
            );
            cast_lcl
        } else {
            ptr_lcl
        }
    } else {
        // Prefer an unbounded nondeterministic value via (implemented or compiler-derived)
        // Arbitrary; fall back to a smart-pointer model (`Box`/`Rc`/`Arc` of a derivable pointee)
        // and then to BoundedArbitrary for container types like Vec<T> or String.
        // Note: use a fresh cache for the Arbitrary check -- `invariant_cache` memoizes a
        // different predicate (Invariant), so the two must not share a map.
        let mut arbitrary_cache = FxHashMap::default();
        let use_arbitrary = implements_arbitrary(ty, models.kani_any, &mut arbitrary_cache)
            || can_derive_arbitrary(ty, models.kani_any, &mut arbitrary_cache);
        let any_inst = if use_arbitrary {
            Instance::resolve(models.kani_any, &GenericArgs(vec![GenericArgKind::Type(ty)]))
                .unwrap_or_else(|_| panic!("expected a ty that implements Arbitrary, got {ty}"))
        } else if let Some((inst, _)) =
            smart_pointer_model_instance(tcx, ty, &models.smart_pointer_models)
        {
            // `Box<T>`/`Rc<T>`/`Arc<T>` whose pointee only *can derive* Arbitrary: the smart
            // pointer's own blanket `Arbitrary` is unresolvable, so use the dedicated model,
            // whose internal `kani::any::<T>()` call this pass rewrites to the synthesized impl.
            inst
        } else {
            // `Instance::resolve` does not check trait bounds, so ensure the type actually
            // implements BoundedArbitrary before emitting the call: an unresolvable
            // `T::bounded_any` would otherwise only surface as an ICE during reachability.
            assert!(
                crate::kani_middle::implements_bounded_arbitrary(tcx, ty, models.kani_bounded_any),
                "expected a ty that implements Arbitrary or BoundedArbitrary, got {ty}"
            );
            Instance::resolve(
                models.kani_bounded_any,
                &GenericArgs(vec![
                    GenericArgKind::Type(ty),
                    GenericArgKind::Const(
                        TyConst::try_from_target_usize(AUTOHARNESS_BOUNDED_ANY_BOUND).unwrap(),
                    ),
                ]),
            )
            .unwrap_or_else(|_| {
                panic!("expected a ty that implements Arbitrary or BoundedArbitrary, got {ty}")
            })
        };
        let lcl = body.new_local(ty, source.span(body.blocks()), mutability);
        body.insert_call(&any_inst, source, InsertPosition::Before, vec![], Place::from(lcl));

        // If the type has a safety invariant, assume that it holds for the nondeterministic value.
        // We only check ADTs since those are the only types for which users can implement
        // `Invariant` in a way that constrains the values (the library's implementations for
        // primitive types are trivially `true`). This applies regardless of whether the value was
        // generated via Arbitrary or BoundedArbitrary.
        if matches!(ty.kind(), TyKind::RigidTy(RigidTy::Adt(..)))
            && implements_invariant(ty, models.kani_assume_safe, invariant_cache)
        {
            let assume_safe_inst = Instance::resolve(
                models.kani_assume_safe,
                &GenericArgs(vec![GenericArgKind::Type(ty)]),
            )
            .unwrap();
            let safe_lcl = body.new_local(ty, source.span(body.blocks()), mutability);
            body.insert_call(
                &assume_safe_inst,
                source,
                InsertPosition::Before,
                vec![Operand::Move(Place::from(lcl))],
                Place::from(safe_lcl),
            );
            safe_lcl
        } else {
            lcl
        }
    }
}

impl AutomaticArbitraryPass {
    /// Insert the basic blocks for generating an arbitrary variant into `body`.
    /// Return the index of the first inserted basic block.
    /// We generate an arbitrary variant by:
    ///   1. Calling kani::any() for each of the variant's field types, then
    ///   2. Constructing the variant from the results of 1) and assigning it to the return local.
    ///
    /// This function will panic if a field type does not implement Arbitrary.
    // The parameters are all distinct pieces of context that the callers already hold
    // individually; grouping them would not make the call sites clearer.
    #[allow(clippy::too_many_arguments)]
    fn call_kani_any_for_variant(
        &self,
        tcx: TyCtxt,
        adt_def: AdtDef,
        adt_args: &GenericArgs,
        body: &mut MutableBody,
        source: &mut SourceInstruction,
        variant_idx: VariantIdx,
        variant: VariantDef,
        invariant_cache: &mut FxHashMap<Ty, bool>,
    ) -> BasicBlockIdx {
        let fields = variant.fields();
        let mut field_locals = vec![];

        // Construct nondeterministic values for each of the variant's fields
        for ty in fields.iter().map(|field| field.ty_with_args(adt_args)) {
            let lcl = call_kani_any_for_ty(
                tcx,
                self.models,
                body,
                ty,
                Mutability::Not,
                source,
                invariant_cache,
            );
            field_locals.push(lcl);
        }

        // Insert a basic block that constructs the variant from each of the nondet fields, then returns it
        body.insert_terminator(
            source,
            InsertPosition::Before,
            Terminator { kind: TerminatorKind::Return, span: source.span(body.blocks()) },
        );
        let mut assign_instr = SourceInstruction::Terminator { bb: source.bb() - 1 };
        let rvalue = Rvalue::Aggregate(
            AggregateKind::Adt(adt_def, variant_idx, adt_args.clone(), None, None),
            field_locals.into_iter().map(|lcl| Operand::Move(lcl.into())).collect(),
        );
        body.assign_to(Place::from(0), rvalue, &mut assign_instr, InsertPosition::Before);

        // The index of the first block we inserted is (last bb index - number of bbs we inserted above it)
        source.bb() - (fields.len() + 1)
    }

    /// Overwrite the default kani::any() implementation `body` for the enum described by `def`.
    /// The returned body is equivalent to:
    /// ```ignore
    /// let discriminant = kani::any();
    /// match discriminant {
    ///   0 => Enum::Variant1(field1, field2),
    ///   1 => Enum::Variant2(..),
    ///   ... (cont.)
    ///   _ => Enum::LastVariant
    /// }
    /// ```
    fn generate_enum_body(&self, tcx: TyCtxt, def: AdtDef, args: GenericArgs, body: Body) -> Body {
        // Autoharness only deems a function with an enum eligible if it has at least one variant, c.f. `can_derive_arbitrary`
        assert!(def.num_variants() > 0);

        let mut new_body = MutableBody::from(body);
        new_body.clear_body(TerminatorKind::Unreachable);
        let mut source = SourceInstruction::Terminator { bb: 0 };
        let mut invariant_cache = FxHashMap::default();

        // Generate a nondet u128 to switch on
        let discr_lcl = call_kani_any_for_ty(
            tcx,
            self.models,
            &mut new_body,
            Ty::from_rigid_kind(RigidTy::Uint(UintTy::U128)),
            Mutability::Not,
            &mut source,
            &mut invariant_cache,
        );

        // Insert a placeholder for the SwitchInt terminator
        let span = source.span(new_body.blocks());
        new_body.insert_terminator(
            &mut source,
            InsertPosition::Before,
            Terminator { kind: TerminatorKind::Unreachable, span },
        );
        let switch_int_instr = SourceInstruction::Terminator { bb: source.bb() - 1 };

        let mut branches: Vec<(u128, BasicBlockIdx)> = vec![];
        // `variants_iter` yields variants in source-declaration order, so the
        // enumeration index is the variant's `VariantIdx` (see `AdtDef::variant`).
        // `VariantDef::idx` is no longer publicly accessible, so reconstruct it.
        for (idx, variant) in def.variants_iter().enumerate() {
            let variant_idx = VariantIdx::to_val(idx);
            let target_bb = self.call_kani_any_for_variant(
                tcx,
                def,
                &args,
                &mut new_body,
                &mut source,
                variant_idx,
                variant,
                &mut invariant_cache,
            );
            branches.push((idx as u128, target_bb));
        }

        let otherwise = branches.pop().unwrap().1;
        let match_term = Terminator {
            kind: TerminatorKind::SwitchInt {
                discr: Operand::Copy(Place::from(discr_lcl)),
                targets: SwitchTargets::new(branches, otherwise),
            },
            span: source.span(new_body.blocks()),
        };
        new_body.replace_terminator(&switch_int_instr, match_term);

        new_body.into()
    }

    /// Overwrite the default kani::any() implementation `body` for the struct described by `def`.
    /// The returned body is equivalent to:
    /// ```ignore
    /// struct Struct {
    ///   field1: kani::any(),
    ///   field2: kani::any(),
    ///   ...
    /// }
    /// ```
    fn generate_struct_body(
        &self,
        tcx: TyCtxt,
        def: AdtDef,
        args: GenericArgs,
        body: Body,
    ) -> Body {
        assert_eq!(def.num_variants(), 1);

        let mut new_body = MutableBody::from(body);
        new_body.clear_body(TerminatorKind::Unreachable);
        let mut source = SourceInstruction::Terminator { bb: 0 };
        let mut invariant_cache = FxHashMap::default();

        let variant = def.variants()[0];
        // A struct has a single variant at index 0.
        self.call_kani_any_for_variant(
            tcx,
            def,
            &args,
            &mut new_body,
            &mut source,
            VariantIdx::to_val(0),
            variant,
            &mut invariant_cache,
        );

        new_body.into()
    }
}
/// Transform the dummy body of an automatic_harness Kani intrinsic to be a proof harness for a given function.
#[derive(Debug, Clone)]
pub struct AutomaticHarnessPass {
    /// The Kani model functions used to construct nondeterministic values.
    models: AnyModels,
    /// The FnDef of KaniModel::CheckDebugFmt
    kani_check_debug_fmt: FnDef,
    /// The FnDef of KaniModel::CheckDisplayFmt
    kani_check_display_fmt: FnDef,
    init_contracts_hook: Instance,
    reset_clause_depth: Instance,
    kani_autoharness_intrinsic: FnDef,
}

impl AutomaticHarnessPass {
    pub fn new(query_db: &QueryDb) -> Self {
        let kani_fns = query_db.kani_functions();
        let kani_autoharness_intrinsic =
            *kani_fns.get(&KaniIntrinsic::AutomaticHarness.into()).unwrap();
        let kani_check_debug_fmt = *kani_fns.get(&KaniModel::CheckDebugFmt.into()).unwrap();
        let kani_check_display_fmt = *kani_fns.get(&KaniModel::CheckDisplayFmt.into()).unwrap();
        let init_contracts_hook = *kani_fns.get(&KaniHook::InitContracts.into()).unwrap();
        let init_contracts_hook =
            Instance::resolve(init_contracts_hook, &GenericArgs(vec![])).unwrap();
        let reset_clause_depth =
            *kani_fns.get(&KaniModel::ResetContractClauseDepth.into()).unwrap();
        let reset_clause_depth =
            Instance::resolve(reset_clause_depth, &GenericArgs(vec![])).unwrap();
        Self {
            models: AnyModels::new(query_db),
            kani_check_debug_fmt,
            kani_check_display_fmt,
            init_contracts_hook,
            reset_clause_depth,
            kani_autoharness_intrinsic,
        }
    }
}

impl TransformPass for AutomaticHarnessPass {
    fn transformation_type() -> TransformationType
    where
        Self: Sized,
    {
        TransformationType::Stubbing
    }

    fn is_enabled(&self, query_db: &QueryDb) -> bool
    where
        Self: Sized,
    {
        matches!(query_db.args().reachability_analysis, ReachabilityType::AllFns)
    }

    fn transform(&mut self, tcx: TyCtxt, body: Body, instance: Instance) -> (bool, Body) {
        debug!(function=?instance.name(), "AutomaticHarnessPass::transform");

        if instance.def.def_id() != self.kani_autoharness_intrinsic.def_id() {
            return (false, body);
        }

        // Retrieve the generic arguments of the harness, which is the type of the function it is verifying,
        // and then resolve `fn_to_verify`.
        let kind = instance.args().0[0].expect_ty().kind();
        let (def, args) = kind.fn_def().unwrap();
        let fn_to_verify = Instance::resolve(def, args).unwrap();
        let fn_to_verify_body = fn_to_verify.body().unwrap();

        let mut harness_body = MutableBody::from(body);
        harness_body.clear_body(TerminatorKind::Return);
        let mut source = SourceInstruction::Terminator { bb: 0 };

        // Debug/Display fmt implementations are exercised through the corresponding check
        // model, which formats a nondeterministic value of the self type into a discarding
        // sink: their `&mut Formatter` argument cannot be generated nondeterministically,
        // and the model reaches `fn_to_verify` through the core formatting machinery with a
        // real `Formatter`.
        if let Some((fmt_trait, self_ty)) = fmt_impl_self_ty(tcx, fn_to_verify) {
            // Generate `&self_ty` directly: `call_kani_any_for_ty` creates the nondeterministic
            // value (assuming its safety invariant, if any) in a local of the harness body --
            // which therefore outlives the model call -- and borrows it.
            let ref_ty =
                Ty::new_ref(Region { kind: RegionKind::ReErased }, self_ty, Mutability::Not);
            let mut invariant_cache = FxHashMap::default();
            let ref_lcl = call_kani_any_for_ty(
                tcx,
                self.models,
                &mut harness_body,
                ref_ty,
                Mutability::Not,
                &mut source,
                &mut invariant_cache,
            );
            let model = match fmt_trait {
                FmtTrait::Debug => self.kani_check_debug_fmt,
                FmtTrait::Display => self.kani_check_display_fmt,
            };
            let model_inst =
                Instance::resolve(model, &GenericArgs(vec![GenericArgKind::Type(self_ty)]))
                    .unwrap();
            let ret_lcl = harness_body.new_local(
                Ty::from_rigid_kind(RigidTy::Tuple(vec![])),
                source.span(harness_body.blocks()),
                Mutability::Not,
            );
            harness_body.insert_call(
                &model_inst,
                &mut source,
                InsertPosition::Before,
                vec![Operand::Move(Place::from(ref_lcl))],
                Place::from(ret_lcl),
            );
            return (true, harness_body.into());
        }

        // Contract harnesses need a free(NULL) statement, c.f. kani_core::init_contracts().
        //
        // Order matters: `reset_contract_clause_depth` must execute BEFORE
        // `init_contracts` (which triggers the first contract dispatch that
        // reads the counter). Because `insert_call(..., Before, ...)`
        // splits the block so that the newly-inserted call runs FIRST (and
        // then advances `source` past it), we insert `reset` first (so it
        // becomes the current first call) and `init_contracts` second (so
        // it runs after `reset`). The manual `proof_for_contract` path in
        // `library/kani_macros/src/sysroot/contracts/mod.rs` produces the
        // same final order (two `stmts.insert(0, ...)` calls with reset
        // inserted last so it ends up first).
        let attrs = KaniAttributes::for_def_id(tcx, def.def_id());
        if attrs.has_contract() {
            let reset_ret = harness_body.new_local(
                Ty::new_tuple(&[]),
                source.span(harness_body.blocks()),
                Mutability::Not,
            );
            harness_body.insert_call(
                &self.reset_clause_depth,
                &mut source,
                InsertPosition::Before,
                vec![],
                Place::from(reset_ret),
            );
            let ret_local = harness_body.new_local(
                Ty::from_rigid_kind(RigidTy::Tuple(vec![])),
                source.span(harness_body.blocks()),
                Mutability::Not,
            );
            harness_body.insert_call(
                &self.init_contracts_hook,
                &mut source,
                InsertPosition::Before,
                vec![],
                Place::from(ret_local),
            );
        }

        // For each argument of `fn_to_verify`, create a nondeterministic value of its type
        // by generating a kani::any() call and saving the result in `arg_local`.
        // If the argument type implements `Invariant`, we additionally assume that the
        // nondeterministic value respects the type's safety invariant,
        // c.f. `call_kani_any_for_ty`.
        let mut invariant_cache = FxHashMap::default();
        let arg_locals = fn_to_verify_body
            .arg_locals()
            .iter()
            .map(|local_decl| {
                call_kani_any_for_ty(
                    tcx,
                    self.models,
                    &mut harness_body,
                    local_decl.ty,
                    local_decl.mutability,
                    &mut source,
                    &mut invariant_cache,
                )
            })
            .collect::<Vec<_>>();

        let func_to_verify_ret = fn_to_verify_body.ret_local();
        let ret_place = Place::from(harness_body.new_local(
            func_to_verify_ret.ty,
            source.span(harness_body.blocks()),
            func_to_verify_ret.mutability,
        ));

        // Call `fn_to_verify` on the nondeterministic arguments generated above.
        harness_body.insert_call(
            &fn_to_verify,
            &mut source,
            InsertPosition::Before,
            arg_locals.iter().map(|lcl| Operand::Copy(Place::from(*lcl))).collect::<Vec<_>>(),
            ret_place,
        );

        (true, harness_body.into())
    }
}
