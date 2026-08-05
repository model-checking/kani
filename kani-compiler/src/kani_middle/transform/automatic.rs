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
use crate::kani_middle::{can_derive_arbitrary, implements_arbitrary};
use crate::kani_queries::QueryDb;
use rustc_data_structures::fx::FxHashMap;
use rustc_middle::ty::TyCtxt;
use rustc_public::CrateDef;
use rustc_public::mir::mono::Instance;
use rustc_public::mir::{
    AggregateKind, BasicBlockIdx, Body, BorrowKind, Local, MutBorrowKind, Mutability, Operand,
    Place, ProjectionElem, Rvalue, SwitchTargets, Terminator, TerminatorKind,
};
use rustc_public::ty::{
    AdtDef, AdtKind, FnDef, GenericArgKind, GenericArgs, RigidTy, Ty, TyConst, TyKind, UintTy,
    VariantDef,
};
use rustc_public_bridge::IndexedVal;
use tracing::debug;

/// Generate `T::any()` implementations for `T`s that do not implement Arbitrary in source code.
/// Currently limited to structs and enums.
#[derive(Debug, Clone)]
pub struct AutomaticArbitraryPass {
    /// The FnDef of KaniModel::Any
    kani_any: FnDef,
    /// The FnDef of KaniModel::AnySliceRef
    kani_any_slice_ref: FnDef,
    /// The FnDef of KaniModel::AnyStrRef
    kani_any_str_ref: FnDef,
    /// The FnDef of KaniModel::BoundedAny
    kani_bounded_any: FnDef,
}

impl AutomaticArbitraryPass {
    pub fn new(_unit: &CodegenUnit, query_db: &QueryDb) -> Self {
        let kani_fns = query_db.kani_functions();
        let kani_any = *kani_fns.get(&KaniModel::Any.into()).unwrap();
        let kani_any_slice_ref = *kani_fns.get(&KaniModel::AnySliceRef.into()).unwrap();
        let kani_any_str_ref = *kani_fns.get(&KaniModel::AnyStrRef.into()).unwrap();
        let kani_bounded_any = *kani_fns.get(&KaniModel::BoundedAny.into()).unwrap();
        Self { kani_any, kani_any_slice_ref, kani_any_str_ref, kani_bounded_any }
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

        if instance.def.def_id() != self.kani_any.def_id() {
            return (false, body);
        }

        // Get the `ty` we're calling `kani::any()` on
        let binding = instance.args();
        let ty = binding.0[0].expect_ty();

        if implements_arbitrary(*ty, self.kani_any, &mut FxHashMap::default()) {
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
/// generate. Strings use a smaller bound than slices: the generated string is the longest
/// valid-UTF-8 prefix of nondeterministic bytes, and reasoning about UTF-8 validity is
/// expensive enough that 16 nondeterministic bytes exceed Kani's default 60s harness timeout,
/// while 8 stay well within it.
const AUTOHARNESS_STR_BOUND: u64 = 8;

/// The bound for nondeterministic values of types that implement `BoundedArbitrary` (rather
/// than `Arbitrary`) that automatic harnesses generate, e.g. `Vec<T>` or `String`.
/// Verification results for functions with such arguments are only valid up to this bound.
/// This is smaller than the slice/str bounds since `BoundedArbitrary` values are heap
/// allocated, and for `String` additionally involve UTF-8 reasoning; a bound of 8 already
/// makes simple `String` harnesses exceed Kani's default 60s harness timeout.
const AUTOHARNESS_BOUNDED_ANY_BOUND: u64 = 4;

/// Insert a call to kani::any::<ty>() in `body`; return the local storing the result.
/// For `&[T]`/`&mut [T]`/`&str`, insert calls to the `KaniModel::AnySliceRef`/`AnyStrRef`
/// models instead, which return a slice of nondeterministic length (bounded by
/// [AUTOHARNESS_SLICE_BOUND]) backed by a nondeterministic array stored in a dedicated local,
/// which stays alive for the entire harness.
/// Panics if `ty` does not implement Arbitrary (and is not a reference to a slice or str of
/// such a type).
fn call_kani_any_for_ty(
    tcx: TyCtxt,
    kani_any: FnDef,
    kani_any_slice_ref: FnDef,
    kani_any_str_ref: FnDef,
    kani_bounded_any: FnDef,
    body: &mut MutableBody,
    ty: Ty,
    mutability: Mutability,
    source: &mut SourceInstruction,
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
                kani_any_slice_ref,
                GenericArgs(vec![
                    GenericArgKind::Type(elem_ty),
                    GenericArgKind::Const(
                        TyConst::try_from_target_usize(AUTOHARNESS_SLICE_BOUND).unwrap(),
                    ),
                ]),
            ),
            TyKind::RigidTy(RigidTy::Str) => (
                Ty::unsigned_ty(UintTy::U8),
                kani_any_str_ref,
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
                    kani_any,
                    kani_any_slice_ref,
                    kani_any_str_ref,
                    kani_bounded_any,
                    body,
                    elem_ty,
                    Mutability::Not,
                    source,
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
            kani_any,
            kani_any_slice_ref,
            kani_any_str_ref,
            kani_bounded_any,
            body,
            inner_ty,
            inner_mutability,
            source,
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
    } else {
        // Prefer an unbounded nondeterministic value via (implemented or compiler-derived)
        // Arbitrary; fall back to BoundedArbitrary for container types like Vec<T> or String.
        let mut cache = FxHashMap::default();
        let use_arbitrary = implements_arbitrary(ty, kani_any, &mut cache)
            || can_derive_arbitrary(ty, kani_any, &mut cache);
        let (inst, generic_args) = if use_arbitrary {
            (kani_any, GenericArgs(vec![GenericArgKind::Type(ty)]))
        } else {
            // `Instance::resolve` does not check trait bounds, so ensure the type actually
            // implements BoundedArbitrary before emitting the call: an unresolvable
            // `T::bounded_any` would otherwise only surface as an ICE during reachability.
            assert!(
                crate::kani_middle::implements_bounded_arbitrary(tcx, ty, kani_bounded_any),
                "expected a ty that implements Arbitrary or BoundedArbitrary, got {ty}"
            );
            (
                kani_bounded_any,
                GenericArgs(vec![
                    GenericArgKind::Type(ty),
                    GenericArgKind::Const(
                        TyConst::try_from_target_usize(AUTOHARNESS_BOUNDED_ANY_BOUND).unwrap(),
                    ),
                ]),
            )
        };
        let inst = Instance::resolve(inst, &generic_args).unwrap_or_else(|_| {
            panic!("expected a ty that implements Arbitrary or BoundedArbitrary, got {ty}")
        });
        let lcl = body.new_local(ty, source.span(body.blocks()), mutability);
        body.insert_call(&inst, source, InsertPosition::Before, vec![], Place::from(lcl));
        lcl
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
    fn call_kani_any_for_variant(
        &self,
        tcx: TyCtxt,
        adt_def: AdtDef,
        adt_args: &GenericArgs,
        body: &mut MutableBody,
        source: &mut SourceInstruction,
        variant: VariantDef,
    ) -> BasicBlockIdx {
        let fields = variant.fields();
        let mut field_locals = vec![];

        // Construct nondeterministic values for each of the variant's fields
        for ty in fields.iter().map(|field| field.ty_with_args(adt_args)) {
            let lcl = call_kani_any_for_ty(
                tcx,
                self.kani_any,
                self.kani_any_slice_ref,
                self.kani_any_str_ref,
                self.kani_bounded_any,
                body,
                ty,
                Mutability::Not,
                source,
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
            AggregateKind::Adt(adt_def, variant.idx, adt_args.clone(), None, None),
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

        // Generate a nondet u128 to switch on
        let discr_lcl = call_kani_any_for_ty(
            tcx,
            self.kani_any,
            self.kani_any_slice_ref,
            self.kani_any_str_ref,
            self.kani_bounded_any,
            &mut new_body,
            Ty::from_rigid_kind(RigidTy::Uint(UintTy::U128)),
            Mutability::Not,
            &mut source,
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
        for variant in def.variants_iter() {
            let target_bb = self.call_kani_any_for_variant(
                tcx,
                def,
                &args,
                &mut new_body,
                &mut source,
                variant,
            );
            branches.push((variant.idx.to_index() as u128, target_bb));
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

        let variant = def.variants()[0];
        self.call_kani_any_for_variant(tcx, def, &args, &mut new_body, &mut source, variant);

        new_body.into()
    }
}
/// Transform the dummy body of an automatic_harness Kani intrinsic to be a proof harness for a given function.
#[derive(Debug, Clone)]
pub struct AutomaticHarnessPass {
    kani_any: FnDef,
    kani_any_slice_ref: FnDef,
    kani_any_str_ref: FnDef,
    kani_bounded_any: FnDef,
    init_contracts_hook: Instance,
    kani_autoharness_intrinsic: FnDef,
}

impl AutomaticHarnessPass {
    pub fn new(query_db: &QueryDb) -> Self {
        let kani_fns = query_db.kani_functions();
        let kani_autoharness_intrinsic =
            *kani_fns.get(&KaniIntrinsic::AutomaticHarness.into()).unwrap();
        let kani_any = *kani_fns.get(&KaniModel::Any.into()).unwrap();
        let kani_any_slice_ref = *kani_fns.get(&KaniModel::AnySliceRef.into()).unwrap();
        let kani_any_str_ref = *kani_fns.get(&KaniModel::AnyStrRef.into()).unwrap();
        let kani_bounded_any = *kani_fns.get(&KaniModel::BoundedAny.into()).unwrap();
        let init_contracts_hook = *kani_fns.get(&KaniHook::InitContracts.into()).unwrap();
        let init_contracts_hook =
            Instance::resolve(init_contracts_hook, &GenericArgs(vec![])).unwrap();
        Self {
            kani_any,
            kani_any_slice_ref,
            kani_any_str_ref,
            kani_bounded_any,
            init_contracts_hook,
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

        // Contract harnesses need a free(NULL) statement, c.f. kani_core::init_contracts().
        let attrs = KaniAttributes::for_def_id(tcx, def.def_id());
        if attrs.has_contract() {
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
        let arg_locals = fn_to_verify_body
            .arg_locals()
            .iter()
            .map(|local_decl| {
                call_kani_any_for_ty(
                    tcx,
                    self.kani_any,
                    self.kani_any_slice_ref,
                    self.kani_any_str_ref,
                    self.kani_bounded_any,
                    &mut harness_body,
                    local_decl.ty,
                    local_decl.mutability,
                    &mut source,
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
