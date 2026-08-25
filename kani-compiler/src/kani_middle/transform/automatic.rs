// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This module contains two passes:
//! 1. `AutomaticHarnessPass`, which transforms the body of an automatic harness to verify a function.
//! 2. `AutomaticArbitraryPass`, which creates `T::any()` implementations for `T`s that do not implement Arbitrary in source code,
//!    but we have determined can derive it.

use crate::args::ReachabilityType;
use crate::kani_middle::attributes::KaniAttributes;
use crate::kani_middle::codegen_units::CodegenUnit;
use crate::kani_middle::kani_functions::{KaniFunction, KaniHook, KaniIntrinsic, KaniModel};
use crate::kani_middle::mined_invariants::{MinedConjunct, MinedExpr, mine_self_assert_conjuncts};
use crate::kani_middle::transform::body::{
    InsertPosition, MutableBody, SourceInstruction, synthetic_source_info,
};
use crate::kani_middle::transform::{TransformPass, TransformationType};
use crate::kani_middle::{
    CtorReturn, FmtTrait, SmartPointerModels, adt_has_private_field_check, can_derive_arbitrary,
    find_arbitrary_constructor, fmt_impl_self_ty, implements_arbitrary, implements_invariant,
    scalar_niche, smart_pointer_model_instance,
};
use crate::kani_queries::QueryDb;
use rustc_data_structures::fx::FxHashMap;
use rustc_middle::ty::TyCtxt;
use rustc_public::mir::mono::Instance;
use rustc_public::mir::{
    AggregateKind, BasicBlock, BasicBlockIdx, BinOp, Body, BorrowKind, CastKind, ConstOperand,
    Local, MutBorrowKind, Mutability, NonDivergingIntrinsic, Operand, Place, ProjectionElem,
    Rvalue, Statement, StatementKind, SwitchTargets, Terminator, TerminatorKind, UnOp,
    UnwindAction, WithRetag,
};
use rustc_public::ty::{
    AdtDef, AdtKind, FnDef, GenericArgKind, GenericArgs, MirConst, Region, RegionKind, RigidTy, Ty,
    TyConst, TyKind, UintTy, VariantDef, VariantIdx,
};
use rustc_public::{CrateDef, CrateDefType};
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
    /// The FnDef of KaniHook::Assume (used for layout-niche assumptions and constructor
    /// success).
    kani_assume: FnDef,
    /// The FnDef of KaniHook::Assert (rewritten into assumptions when inlining
    /// assert-guarded constructors).
    kani_assert: FnDef,
    /// The FnDef of KaniModel::AssumeSafe
    kani_assume_safe: FnDef,
    /// The FnDef of KaniModel::BoundedAny
    kani_bounded_any: FnDef,
    /// The (optional) smart-pointer generation models (`Box`/`Rc`/`Arc`).
    smart_pointer_models: SmartPointerModels,
    /// The (optional, alloc-requiring) unbounded slice/`Vec` generation models.
    unbounded_models: UnboundedModels,
    /// Whether --constructor-args is enabled: under this heuristic-filter umbrella, generated
    /// ADT values additionally assume the type's mined invariant conjuncts.
    constructor_args: bool,
}

impl AnyModels {
    fn new(query_db: &QueryDb) -> Self {
        let kani_fns = query_db.kani_functions();
        Self {
            kani_any: *kani_fns.get(&KaniModel::Any.into()).unwrap(),
            kani_any_ptr: *kani_fns.get(&KaniModel::AnyPtr.into()).unwrap(),
            kani_any_slice_ref: *kani_fns.get(&KaniModel::AnySliceRef.into()).unwrap(),
            kani_any_str_ref: *kani_fns.get(&KaniModel::AnyStrRef.into()).unwrap(),
            kani_assume: *kani_fns.get(&KaniHook::Assume.into()).unwrap(),
            kani_assert: *kani_fns.get(&KaniHook::Assert.into()).unwrap(),
            kani_assume_safe: *kani_fns.get(&KaniModel::AssumeSafe.into()).unwrap(),
            kani_bounded_any: *kani_fns.get(&KaniModel::BoundedAny.into()).unwrap(),
            smart_pointer_models: SmartPointerModels::from_kani_functions(kani_fns),
            unbounded_models: UnboundedModels::from_kani_functions(kani_fns),
            constructor_args: query_db.args().autoharness_constructor_args,
        }
    }
}

/// Generate `T::any()` implementations for `T`s that do not implement Arbitrary in source code.
/// Currently limited to structs and enums.
#[derive(Debug, Clone)]
pub struct AutomaticArbitraryPass {
    /// The Kani model functions used to construct nondeterministic values.
    models: AnyModels,
    /// Whether --constructor-args is enabled: generate values of private-field types through
    /// their public constructors instead of raw field synthesis.
    constructor_args: bool,
}

impl AutomaticArbitraryPass {
    pub fn new(_unit: &CodegenUnit, query_db: &QueryDb) -> Self {
        Self {
            models: AnyModels::new(query_db),
            constructor_args: query_db.args().autoharness_constructor_args,
        }
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
            // Under --constructor-args, generate values of structs with private fields
            // through one of their public constructors (raw field synthesis can violate the
            // type's representation invariant, producing false alarms); fall through to
            // field synthesis when no viable constructor exists.
            if self.constructor_args
                && def.kind() == AdtKind::Struct
                && adt_has_private_field_check(tcx, def)
            {
                // Prefer assert-guarded representation constructors, inlined with panic
                // paths converted to assumptions: their own assertions state the type's
                // validity contract, and they are typically surjective onto the valid value
                // space (unlike checked constructors, which may reach only a subset).
                if let Some(ctor) = crate::kani_middle::find_unchecked_constructor(
                    tcx,
                    *ty,
                    self.models.kani_any,
                    &mut FxHashMap::default(),
                ) && let Some(new_body) =
                    self.generate_unchecked_ctor_body(tcx, ctor, *ty, body.clone())
                {
                    debug!(?ty, ctor=?ctor.name(), "generate_unchecked_ctor_body");
                    return (true, new_body);
                }
                if let Some((ctor, shape)) = find_arbitrary_constructor(
                    tcx,
                    *ty,
                    self.models.kani_any,
                    &mut FxHashMap::default(),
                ) {
                    debug!(?ty, ctor=?ctor.name(), ?shape, "generate_ctor_body");
                    return (true, self.generate_ctor_body(tcx, ctor, shape, *ty, body));
                }
            }
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

/// Remap all locals and block targets of an inlined basic block. Returns false (bail out)
/// when the block contains a construct the remapper does not support; the caller then falls
/// back to non-inlined generation. The allowlist covers everything rustc emits for
/// assert-guarded field-packing constructors (the C14 mining target).
fn remap_block(bb: &mut BasicBlock, local_map: &[Local], block_offset: usize) -> bool {
    let remap_place = |p: &mut Place| {
        p.local = local_map[p.local];
        for elem in p.projection.iter_mut() {
            if let ProjectionElem::Index(l) = elem {
                *l = local_map[*l];
            }
        }
    };
    let remap_operand = |op: &mut Operand| match op {
        Operand::Copy(p) | Operand::Move(p) => remap_place(p),
        Operand::Constant(_) | Operand::RuntimeChecks(_) => {}
    };
    for stmt in bb.statements.iter_mut() {
        match &mut stmt.kind {
            StatementKind::Assign(place, rvalue) => {
                remap_place(place);
                match rvalue {
                    Rvalue::Use(op, _) | Rvalue::Repeat(op, _) | Rvalue::Cast(_, op, _) => {
                        remap_operand(op)
                    }
                    Rvalue::BinaryOp(_, a, b) | Rvalue::CheckedBinaryOp(_, a, b) => {
                        remap_operand(a);
                        remap_operand(b);
                    }
                    Rvalue::UnaryOp(_, op) => remap_operand(op),
                    Rvalue::Ref(_, _, p)
                    | Rvalue::Reborrow(_, _, p)
                    | Rvalue::AddressOf(_, p)
                    | Rvalue::CopyForDeref(p)
                    | Rvalue::Discriminant(p)
                    | Rvalue::Len(p) => remap_place(p),
                    Rvalue::Aggregate(_, ops) => ops.iter_mut().for_each(remap_operand),
                    Rvalue::ThreadLocalRef(_) => return false,
                }
            }
            StatementKind::StorageLive(l) | StatementKind::StorageDead(l) => {
                *l = local_map[*l];
            }
            StatementKind::SetDiscriminant { place, .. }
            | StatementKind::PlaceMention(place)
            | StatementKind::FakeRead(_, place) => remap_place(place),
            StatementKind::Intrinsic(NonDivergingIntrinsic::Assume(op)) => remap_operand(op),
            StatementKind::Intrinsic(NonDivergingIntrinsic::CopyNonOverlapping(cp)) => {
                remap_operand(&mut cp.src);
                remap_operand(&mut cp.dst);
                remap_operand(&mut cp.count);
            }
            StatementKind::AscribeUserType { .. }
            | StatementKind::Coverage(_)
            | StatementKind::ConstEvalCounter
            | StatementKind::Nop => {}
        }
    }
    match &mut bb.terminator.kind {
        TerminatorKind::Goto { target } => *target += block_offset,
        TerminatorKind::SwitchInt { discr, targets } => {
            remap_operand(discr);
            let branches: Vec<_> = targets.branches().map(|(v, t)| (v, t + block_offset)).collect();
            *targets = SwitchTargets::new(branches, targets.otherwise() + block_offset);
        }
        TerminatorKind::Call { func, args, destination, target, unwind } => {
            remap_operand(func);
            args.iter_mut().for_each(remap_operand);
            remap_place(destination);
            if let Some(t) = target {
                *t += block_offset;
            }
            if let UnwindAction::Cleanup(t) = unwind {
                *t += block_offset;
            }
        }
        TerminatorKind::Assert { cond, target, unwind, .. } => {
            remap_operand(cond);
            *target += block_offset;
            if let UnwindAction::Cleanup(t) = unwind {
                *t += block_offset;
            }
        }
        TerminatorKind::Drop { place, target, unwind, .. } => {
            remap_place(place);
            *target += block_offset;
            if let UnwindAction::Cleanup(t) = unwind {
                *t += block_offset;
            }
        }
        TerminatorKind::Return
        | TerminatorKind::Unreachable
        | TerminatorKind::Resume
        | TerminatorKind::Abort => {}
        TerminatorKind::InlineAsm { .. } => return false,
    }
    true
}

/// C14 (assert mining, dynamic form): inline `callee`'s monomorphic body into `body` at
/// `source`, with every validity statement converted into a filter on the nondeterministic
/// inputs:
/// - `kani::assert(cond, msg)` calls (Kani's macro overrides have already rewritten user
///   asserts/panics into these) become `kani::assume(cond)`;
/// - `hint::assert_unchecked(cond)` calls (UB-hint contracts, e.g. deranged's
///   `new_unchecked`) become `kani::assume(cond)`;
/// - raw panic-entry calls become `assume(false); unreachable`;
/// - MIR `Assert` terminators (overflow checks) become `assume(cond == expected)`.
///
/// Calls *within* the inlined body whose callees themselves contain such validity statements
/// (e.g. time's `Time::__from_hms_nanos_unchecked` calling deranged's `new_unchecked`) are
/// recursively inlined, up to [INLINE_MAX_DEPTH] levels and [INLINE_MAX_BLOCKS] blocks per
/// callee; other calls are kept as plain calls.
///
/// `arg_locals` must hold fully-initialized constructor arguments. Returns the local holding
/// the constructed value, or None (caller falls back to a plain call) if the outer callee
/// body contains unsupported constructs. (A bail-out mid-way leaves only unused locals
/// behind, which is harmless.)
const INLINE_MAX_DEPTH: usize = 3;
const INLINE_MAX_BLOCKS: usize = 32;

#[allow(clippy::too_many_arguments)]
fn inline_with_assumed_panics(
    tcx: TyCtxt,
    kani_assume: FnDef,
    kani_assert: FnDef,
    body: &mut MutableBody,
    source: &mut SourceInstruction,
    callee: Instance,
    arg_locals: &[Local],
    ret_ty: Ty,
) -> Option<Local> {
    let callee_body = callee.body()?;
    let span = source.span(body.blocks());
    let ret_lcl = body.new_local(ret_ty, span, Mutability::Mut);

    // All blocks from `block_offset` onward are planned into `planned`; slots are allocated
    // (possibly ahead of being filled) so that nested inlining can interleave with the outer
    // walk without breaking target indices.
    let continuation = body.blocks().len();
    let block_offset = continuation + 1;
    let assume_inst = Instance::resolve(kani_assume, &GenericArgs(vec![])).unwrap();

    struct Ctx<'tcx, 'a> {
        tcx: TyCtxt<'tcx>,
        kani_assert: FnDef,
        assume_inst: Instance,
        body: &'a mut MutableBody,
        planned: Vec<Option<BasicBlock>>,
        block_offset: usize,
        span: rustc_public::ty::Span,
    }

    impl Ctx<'_, '_> {
        fn alloc(&mut self, n: usize) -> usize {
            let base = self.block_offset + self.planned.len();
            self.planned.extend(std::iter::repeat_with(|| None).take(n));
            base
        }

        fn set(&mut self, idx: usize, bb: BasicBlock) {
            self.planned[idx - self.block_offset] = Some(bb);
        }

        fn assume_call_terminator(
            &mut self,
            cond: Operand,
            target: BasicBlockIdx,
        ) -> TerminatorKind {
            let func_lcl = self.body.new_local(self.assume_inst.ty(), self.span, Mutability::Not);
            let unit_lcl = self.body.new_local(Ty::new_tuple(&[]), self.span, Mutability::Mut);
            TerminatorKind::Call {
                func: Operand::Copy(Place::from(func_lcl)),
                args: vec![cond],
                destination: Place::from(unit_lcl),
                target: Some(target),
                unwind: UnwindAction::Terminate,
            }
        }

        /// Does `fn_body` directly contain a validity statement worth mining?
        fn worth_inlining(&self, fn_body: &Body) -> bool {
            fn_body.blocks.iter().any(|bb| match &bb.terminator.kind {
                TerminatorKind::Assert { .. } => true,
                TerminatorKind::Call { func, .. } => {
                    match func.ty(fn_body.locals()).map(|t| t.kind()) {
                        Ok(TyKind::RigidTy(RigidTy::FnDef(def, _))) => {
                            def == self.kani_assert
                                || is_assert_unchecked_def(def)
                                || is_panic_def(self.tcx, def)
                        }
                        _ => false,
                    }
                }
                _ => false,
            })
        }

        /// Plan `fn_body` (of `n` blocks) into slots `base..base+n`, remapping via
        /// `local_map`, converting validity statements, recursively inlining qualifying
        /// callees. Returns false to bail out (unsupported construct at depth 0; at deeper
        /// levels callers pre-check with `worth_inlining` and blocks are conservative).
        fn plan_body(
            &mut self,
            fn_body: &Body,
            local_map: &[Local],
            base: usize,
            ret_target: BasicBlockIdx,
            depth: usize,
        ) -> bool {
            for (i, callee_bb) in fn_body.blocks.iter().enumerate() {
                let mut bb = callee_bb.clone();
                if !remap_block(&mut bb, local_map, base) {
                    return false;
                }
                match &mut bb.terminator.kind {
                    TerminatorKind::Return => {
                        bb.terminator.kind = TerminatorKind::Goto { target: ret_target };
                    }
                    TerminatorKind::Resume | TerminatorKind::Abort => {
                        bb.terminator.kind = TerminatorKind::Unreachable;
                    }
                    TerminatorKind::Assert { cond, expected, target, .. } => {
                        let (cond, expected, target) = (cond.clone(), *expected, *target);
                        let cond_lcl =
                            self.body.new_local(Ty::bool_ty(), self.span, Mutability::Mut);
                        let rv = if expected {
                            Rvalue::Use(cond, WithRetag::No)
                        } else {
                            Rvalue::UnaryOp(UnOp::Not, cond)
                        };
                        bb.statements.push(Statement {
                            kind: StatementKind::Assign(Place::from(cond_lcl), rv),
                            source_info: synthetic_source_info(self.span),
                        });
                        bb.terminator.kind = self
                            .assume_call_terminator(Operand::Move(Place::from(cond_lcl)), target);
                    }
                    TerminatorKind::Call { func, args, destination, target, .. } => {
                        let fn_def = match func.ty(self.body.locals()).map(|t| t.kind()) {
                            Ok(TyKind::RigidTy(RigidTy::FnDef(def, fn_args))) => {
                                Some((def, fn_args))
                            }
                            _ => None,
                        };
                        if let Some((def, _)) = &fn_def
                            && (*def == self.kani_assert || is_assert_unchecked_def(*def))
                            && let Some(cond) = args.first()
                        {
                            // kani::assert(cond, msg) / assert_unchecked(cond) -> assume(cond)
                            let cond = cond.clone();
                            let target = target.expect("assert has a return target");
                            bb.terminator.kind = self.assume_call_terminator(cond, target);
                        } else if let Some((def, _)) = &fn_def
                            && is_panic_def(self.tcx, *def)
                        {
                            // panic -> assume(false); unreachable
                            let unreach = self.alloc(1);
                            self.set(
                                unreach,
                                BasicBlock {
                                    statements: vec![],
                                    terminator: Terminator {
                                        kind: TerminatorKind::Unreachable,
                                        source_info: synthetic_source_info(self.span),
                                    },
                                },
                            );
                            let false_op = Operand::Constant(ConstOperand {
                                span: self.span,
                                user_ty: None,
                                const_: MirConst::from_bool(false),
                            });
                            bb.terminator.kind = self.assume_call_terminator(false_op, unreach);
                        } else if depth < INLINE_MAX_DEPTH
                            && let Some((def, fn_args)) = &fn_def
                            && let Ok(inst) = Instance::resolve(*def, fn_args)
                            && let Some(inner_body) = inst.body()
                            && inner_body.blocks.len() <= INLINE_MAX_BLOCKS
                            && self.worth_inlining(&inner_body)
                        {
                            // Recursively inline: materialize args into fresh locals,
                            // stitch the return value into the call's destination.
                            let target = target.expect("inlined callee has a return target");
                            let inner_ret_ty = inner_body.locals()[0].ty;
                            let inner_ret_lcl =
                                self.body.new_local(inner_ret_ty, self.span, Mutability::Mut);
                            let mut inner_map = vec![inner_ret_lcl];
                            for (arg_op, decl) in args.iter().zip(inner_body.arg_locals().iter()) {
                                let a = self.body.new_local(decl.ty, self.span, Mutability::Mut);
                                bb.statements.push(Statement {
                                    kind: StatementKind::Assign(
                                        Place::from(a),
                                        Rvalue::Use(arg_op.clone(), WithRetag::No),
                                    ),
                                    source_info: synthetic_source_info(self.span),
                                });
                                inner_map.push(a);
                            }
                            for decl in inner_body.locals().iter().skip(1 + args.len()) {
                                inner_map.push(self.body.new_local(
                                    decl.ty,
                                    self.span,
                                    Mutability::Mut,
                                ));
                            }
                            let stitch = self.alloc(1);
                            let inner_base = self.alloc(inner_body.blocks.len());
                            self.set(
                                stitch,
                                BasicBlock {
                                    statements: vec![Statement {
                                        kind: StatementKind::Assign(
                                            destination.clone(),
                                            Rvalue::Use(
                                                Operand::Move(Place::from(inner_ret_lcl)),
                                                WithRetag::No,
                                            ),
                                        ),
                                        source_info: synthetic_source_info(self.span),
                                    }],
                                    terminator: Terminator {
                                        kind: TerminatorKind::Goto { target },
                                        source_info: synthetic_source_info(self.span),
                                    },
                                },
                            );
                            if self.plan_body(
                                &inner_body,
                                &inner_map,
                                inner_base,
                                stitch,
                                depth + 1,
                            ) {
                                bb.terminator.kind = TerminatorKind::Goto { target: inner_base };
                            } else {
                                // Nested bail-out: keep the plain call; fill the reserved
                                // slots with unreachable stubs (never targeted).
                                for j in 0..inner_body.blocks.len() {
                                    if self.planned[inner_base + j - self.block_offset].is_none() {
                                        self.set(
                                            inner_base + j,
                                            BasicBlock {
                                                statements: vec![],
                                                terminator: Terminator {
                                                    kind: TerminatorKind::Unreachable,
                                                    source_info: synthetic_source_info(self.span),
                                                },
                                            },
                                        );
                                    }
                                }
                            }
                        }
                        // else: keep the plain (already remapped) call.
                    }
                    _ => {}
                }
                self.set(base + i, bb);
            }
            true
        }
    }

    // Map callee locals: _0 -> ret_lcl, _1..=argc -> arg_locals, rest -> fresh.
    let mut local_map: Vec<Local> = Vec::with_capacity(callee_body.locals().len());
    local_map.push(ret_lcl);
    let argc = callee_body.arg_locals().len();
    assert_eq!(argc, arg_locals.len());
    local_map.extend_from_slice(arg_locals);
    for decl in callee_body.locals().iter().skip(1 + argc) {
        local_map.push(body.new_local(decl.ty, span, Mutability::Mut));
    }

    let mut ctx = Ctx { tcx, kani_assert, assume_inst, body, planned: vec![], block_offset, span };
    let outer_base = ctx.alloc(callee_body.blocks.len());
    if !ctx.plan_body(&callee_body, &local_map, outer_base, continuation, 0) {
        return None;
    }
    let planned = ctx.planned;

    // Commit: split the caller and append all planned blocks at their precomputed indices.
    let placeholder = Terminator {
        kind: TerminatorKind::Goto { target: outer_base },
        source_info: synthetic_source_info(span),
    };
    let (_goto_bb, actual_continuation) = body.split_with_terminator(source, placeholder);
    assert_eq!(actual_continuation, continuation);
    for bb in planned {
        body.push_raw_bb(bb.expect("all planned slots must be filled"));
    }
    Some(ret_lcl)
}

/// Whether `def` is a panic entry point.
fn is_panic_def(tcx: TyCtxt, def: FnDef) -> bool {
    let def_id = rustc_public::rustc_internal::internal(tcx, def.def_id());
    Some(def_id) == tcx.lang_items().panic_fn()
        || Some(def_id) == tcx.lang_items().panic_fmt()
        || Some(def_id) == tcx.lang_items().begin_panic_fn()
        || def.name().starts_with("core::panicking::")
}

/// Whether `def` is `core`/`std`'s `hint::assert_unchecked` (a UB-hint contract whose single
/// argument is a validity condition). Matched by exact path rather than substring, so an
/// unrelated user function whose name merely contains "assert_unchecked" is not misclassified
/// (which would drop its call and, if its first argument were not a `bool`, emit ill-typed MIR).
fn is_assert_unchecked_def(def: FnDef) -> bool {
    matches!(def.name().as_str(), "core::hint::assert_unchecked" | "std::hint::assert_unchecked")
}

/// Materialize a [MinedExpr] over the value in `value_local` (of the mined type) as MIR,
/// returning the local holding the expression's result. Total by construction: the AST
/// contains only field reads, constants, and pure operators.
fn build_mined_expr(
    body: &mut MutableBody,
    source: &mut SourceInstruction,
    value_local: Local,
    expr: &MinedExpr,
) -> Local {
    let span = source.span(body.blocks());
    match expr {
        MinedExpr::Field(path) => {
            let (_, leaf_ty) = *path.last().unwrap();
            let place = Place {
                local: value_local,
                projection: path.iter().map(|(idx, ty)| ProjectionElem::Field(*idx, *ty)).collect(),
            };
            let lcl = body.new_local(leaf_ty, span, Mutability::Not);
            body.assign_to(
                Place::from(lcl),
                Rvalue::Use(Operand::Copy(place), WithRetag::No),
                source,
                InsertPosition::Before,
            );
            lcl
        }
        MinedExpr::DowncastField(variant, path) => {
            // Reads the variant's field regardless of the actual discriminant; consumers
            // guard the enclosing conjunct with `discriminant != variant || ...`, so the
            // read value is irrelevant on other variants (the read itself is byte-level
            // and harmless to CBMC).
            let (_, leaf_ty) = *path.last().unwrap();
            let mut projection =
                vec![ProjectionElem::Downcast(rustc_public::ty::VariantIdx::to_val(*variant))];
            projection.extend(path.iter().map(|(idx, ty)| ProjectionElem::Field(*idx, *ty)));
            let place = Place { local: value_local, projection };
            let lcl = body.new_local(leaf_ty, span, Mutability::Not);
            body.assign_to(
                Place::from(lcl),
                Rvalue::Use(Operand::Copy(place), WithRetag::No),
                source,
                InsertPosition::Before,
            );
            lcl
        }
        MinedExpr::Const(_, c) => {
            let ty = c.const_.ty();
            let lcl = body.new_local(ty, span, Mutability::Not);
            body.assign_to(
                Place::from(lcl),
                Rvalue::Use(Operand::Constant(c.clone()), WithRetag::No),
                source,
                InsertPosition::Before,
            );
            lcl
        }
        MinedExpr::BinOp(op, a, b) => {
            let la = build_mined_expr(body, source, value_local, a);
            let lb = build_mined_expr(body, source, value_local, b);
            let ty = expr.ty().unwrap();
            let lcl = body.new_local(ty, span, Mutability::Not);
            body.assign_to(
                Place::from(lcl),
                Rvalue::BinaryOp(
                    *op,
                    Operand::Copy(Place::from(la)),
                    Operand::Copy(Place::from(lb)),
                ),
                source,
                InsertPosition::Before,
            );
            lcl
        }
        MinedExpr::UnOp(op, a) => {
            let la = build_mined_expr(body, source, value_local, a);
            let ty = expr.ty().unwrap();
            let lcl = body.new_local(ty, span, Mutability::Not);
            body.assign_to(
                Place::from(lcl),
                Rvalue::UnaryOp(*op, Operand::Copy(Place::from(la))),
                source,
                InsertPosition::Before,
            );
            lcl
        }
    }
}

/// The enum variant a conjunct is guarded by, if any: mining produces conjuncts whose
/// downcast field reads all target the assert's match arm, so a single variant governs the
/// whole expression.
fn conjunct_guard_variant(expr: &MinedExpr) -> Option<usize> {
    match expr {
        MinedExpr::Field(_) | MinedExpr::Const(_, _) => None,
        MinedExpr::DowncastField(v, _) => Some(*v),
        MinedExpr::BinOp(_, a, b) => {
            conjunct_guard_variant(a).or_else(|| conjunct_guard_variant(b))
        }
        MinedExpr::UnOp(_, a) => conjunct_guard_variant(a),
    }
}

/// Build the final condition local for a conjunct over `value_local`: the raw expression
/// for struct conjuncts, or `discriminant(value) != variant || expr` for variant-guarded
/// (enum) conjuncts, making the claim vacuously true on other variants.
fn build_guarded_conjunct(
    tcx: TyCtxt,
    body: &mut MutableBody,
    source: &mut SourceInstruction,
    value_local: Local,
    value_ty: Ty,
    expr: &MinedExpr,
) -> Option<Local> {
    let raw = build_mined_expr(body, source, value_local, expr);
    let Some(variant) = conjunct_guard_variant(expr) else { return Some(raw) };
    let span = source.span(body.blocks());
    // discriminant(value)
    let discr_ty = value_ty.kind().discriminant_ty()?;
    let discr_lcl = body.new_local(discr_ty, span, Mutability::Not);
    body.assign_to(
        Place::from(discr_lcl),
        Rvalue::Discriminant(Place::from(value_local)),
        source,
        InsertPosition::Before,
    );
    // Transmute to a same-width uint for the comparison (as assume_scalar_niche does).
    let niche_bits = crate::kani_middle::scalar_width_bits(tcx, discr_ty)?;
    // Bail (skip this conjunct) rather than fall back to the *unguarded* `raw` expression: for a
    // variant-guarded (enum) conjunct, assuming/checking `raw` without the discriminant guard
    // would apply it to *all* variants, excluding valid values of the other variants (unsound in
    // the assume direction). These arms are unreachable for real discriminant types, but making
    // the skip explicit keeps the "never emit an unguarded enum conjunct" intent correct by
    // construction.
    let uint_ty = match niche_bits {
        8 => UintTy::U8,
        16 => UintTy::U16,
        32 => UintTy::U32,
        64 => UintTy::U64,
        128 => UintTy::U128,
        _ => return None,
    };
    let raw_ty = Ty::from_rigid_kind(RigidTy::Uint(uint_ty));
    let discr_uint = body.new_local(raw_ty, span, Mutability::Not);
    body.assign_to(
        Place::from(discr_uint),
        Rvalue::Cast(CastKind::Transmute, Operand::Copy(Place::from(discr_lcl)), raw_ty),
        source,
        InsertPosition::Before,
    );
    let TyKind::RigidTy(RigidTy::Adt(adt_def, _)) = value_ty.kind() else { return None };
    let discr_val =
        adt_def.discriminant_for_variant(rustc_public::ty::VariantIdx::to_val(variant)).val;
    let mask = if niche_bits == 128 { u128::MAX } else { (1u128 << niche_bits) - 1 };
    let variant_const = Operand::Constant(ConstOperand {
        span,
        user_ty: None,
        const_: MirConst::try_from_uint(discr_val & mask, uint_ty).ok()?,
    });
    let bool_ty = Ty::bool_ty();
    let ne_lcl = body.new_local(bool_ty, span, Mutability::Not);
    body.assign_to(
        Place::from(ne_lcl),
        Rvalue::BinaryOp(BinOp::Ne, Operand::Copy(Place::from(discr_uint)), variant_const),
        source,
        InsertPosition::Before,
    );
    let cond_lcl = body.new_local(bool_ty, span, Mutability::Not);
    body.assign_to(
        Place::from(cond_lcl),
        Rvalue::BinaryOp(
            BinOp::BitOr,
            Operand::Move(Place::from(ne_lcl)),
            Operand::Move(Place::from(raw)),
        ),
        source,
        InsertPosition::Before,
    );
    Some(cond_lcl)
}

/// Emit `kani::assume(<conjunct>)` for each mined conjunct of `ty` over the value in
/// `value_local`. Mined conjuncts are the type's own assertions (a necessary condition of
/// the values the crate's code accepts), so assuming them filters generated values the same
/// way constructor-assert mining does, at lower formula cost than constructor inlining.
///
/// Soundness caveat: the "asserted in >= MIN_ASSERTING_METHODS methods" filter is a heuristic,
/// not a proof of type-invariance; a shared *precondition* that is not a universal invariant
/// could be assumed here and exclude otherwise-valid values (a potential missed bug). Only used
/// under the opt-in, under-approximating `(ctor)` contract; tracked in
/// <https://github.com/model-checking/kani/issues/4763>.
fn assume_mined_invariants(
    tcx: TyCtxt,
    kani_assume: FnDef,
    body: &mut MutableBody,
    source: &mut SourceInstruction,
    value_local: Local,
    value_ty: Ty,
    conjuncts: &[MinedConjunct],
) {
    let assume_inst = Instance::resolve(kani_assume, &GenericArgs(vec![])).unwrap();
    for conjunct in conjuncts {
        let Some(cond) =
            build_guarded_conjunct(tcx, body, source, value_local, value_ty, &conjunct.expr)
        else {
            continue;
        };
        let span = source.span(body.blocks());
        let unit_lcl = body.new_local(Ty::new_tuple(&[]), span, Mutability::Not);
        body.insert_call(
            &assume_inst,
            source,
            InsertPosition::Before,
            vec![Operand::Move(Place::from(cond))],
            Place::from(unit_lcl),
        );
    }
}

/// Emit `kani::assert(<conjunct>, msg)` for each mined conjunct of `ty` over the value in
/// `value_local` (`--check-invariants`): checks that a function's return value satisfies
/// the type's own assertions. Reported with a distinct message so users can recognize the
/// property class (the mined predicate is heuristic; a failure means the returned value
/// would trip the type's own assertions when used).
#[allow(clippy::too_many_arguments)]
fn check_mined_invariants(
    tcx: TyCtxt,
    kani_assert: FnDef,
    body: &mut MutableBody,
    source: &mut SourceInstruction,
    value_local: Local,
    ty: Ty,
    // For payloads peeled out of Option/Result returns: (wrapper local, ok variant idx,
    // wrapper ty). Each check is guarded with `wrapper discriminant != ok || conjunct`, so
    // None/Err returns pass vacuously (the payload read is a harmless byte-level read).
    wrapper: Option<(Local, usize, Ty)>,
    conjuncts: &[MinedConjunct],
) {
    let assert_inst = Instance::resolve(kani_assert, &GenericArgs(vec![])).unwrap();
    let wrapper_present = wrapper.is_some();
    // Compute the wrapper guard (discriminant != ok) once.
    let wrapper_ne: Option<Local> = wrapper.and_then(|(wlcl, ok_idx, wty)| {
        let span = source.span(body.blocks());
        let discr_ty = wty.kind().discriminant_ty()?;
        let discr_lcl = body.new_local(discr_ty, span, Mutability::Not);
        body.assign_to(
            Place::from(discr_lcl),
            Rvalue::Discriminant(Place::from(wlcl)),
            source,
            InsertPosition::Before,
        );
        let bits = crate::kani_middle::scalar_width_bits(tcx, discr_ty)?;
        let uint_ty = match bits {
            8 => UintTy::U8,
            16 => UintTy::U16,
            32 => UintTy::U32,
            64 => UintTy::U64,
            128 => UintTy::U128,
            _ => return None,
        };
        let raw_ty = Ty::from_rigid_kind(RigidTy::Uint(uint_ty));
        let discr_uint = body.new_local(raw_ty, span, Mutability::Not);
        body.assign_to(
            Place::from(discr_uint),
            Rvalue::Cast(CastKind::Transmute, Operand::Copy(Place::from(discr_lcl)), raw_ty),
            source,
            InsertPosition::Before,
        );
        let TyKind::RigidTy(RigidTy::Adt(adt_def, _)) = wty.kind() else { return None };
        let discr_val =
            adt_def.discriminant_for_variant(rustc_public::ty::VariantIdx::to_val(ok_idx)).val;
        let mask = if bits == 128 { u128::MAX } else { (1u128 << bits) - 1 };
        let c = Operand::Constant(ConstOperand {
            span,
            user_ty: None,
            const_: MirConst::try_from_uint(discr_val & mask, uint_ty).ok()?,
        });
        let ne = body.new_local(Ty::bool_ty(), span, Mutability::Not);
        body.assign_to(
            Place::from(ne),
            Rvalue::BinaryOp(BinOp::Ne, Operand::Copy(Place::from(discr_uint)), c),
            source,
            InsertPosition::Before,
        );
        Some(ne)
    });
    // If a wrapper (Option/Result) was supplied but its discriminant guard could not be built,
    // skip: emitting the check on the downcast payload without the `discriminant != ok` guard
    // would assert over garbage bytes for None/Err returns (a spurious failure). Unreachable for
    // real Option/Result, but keep the "never check an unguarded payload" intent explicit.
    if wrapper_present && wrapper_ne.is_none() {
        return;
    }
    for conjunct in conjuncts {
        let Some(mut cond) =
            build_guarded_conjunct(tcx, body, source, value_local, ty, &conjunct.expr)
        else {
            continue;
        };
        if let Some(ne) = wrapper_ne {
            let span = source.span(body.blocks());
            let ored = body.new_local(Ty::bool_ty(), span, Mutability::Not);
            body.assign_to(
                Place::from(ored),
                Rvalue::BinaryOp(
                    BinOp::BitOr,
                    Operand::Copy(Place::from(ne)),
                    Operand::Move(Place::from(cond)),
                ),
                source,
                InsertPosition::Before,
            );
            cond = ored;
        }
        let span = source.span(body.blocks());
        let msg = format!(
            "mined invariant of `{ty}` violated by return value (asserted in {})",
            conjunct.asserted_in.join(", ")
        );
        let msg_op = body.new_str_operand(&msg, span);
        let unit_lcl = body.new_local(Ty::new_tuple(&[]), span, Mutability::Not);
        body.insert_call(
            &assert_inst,
            source,
            InsertPosition::Before,
            vec![Operand::Move(Place::from(cond)), msg_op],
            Place::from(unit_lcl),
        );
    }
}

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
/// If `ty` has a scalar layout with a restricted valid range (a layout niche), emit
/// `kani::assume(<raw bits of the value in place_local> in valid_range)`.
///
/// Values outside the niche are language-level invalid -- rustc packs enum variants into the
/// invalid bit patterns, so such a value is as invalid as a `bool` holding 3. Nondeterministic
/// value generation must therefore never produce them: e.g. std's `NonZero` niches, or
/// `core::time::Duration`'s `Nanoseconds` field (`rustc_layout_scalar_valid_range` types),
/// whose compiler-derived generation would otherwise produce invalid values and raise false
/// alarms in every harness generating the type.
///
/// The assumption is sound by construction -- it assumes a *necessary* condition of
/// language-level validity, so every valid value stays in the explored set -- and therefore
/// needs no flag or reporting caveat.
fn assume_scalar_niche(
    tcx: TyCtxt,
    kani_assume: FnDef,
    body: &mut MutableBody,
    source: &mut SourceInstruction,
    place_local: Local,
    ty: Ty,
) {
    let Some(niche) = scalar_niche(tcx, ty) else { return };
    let span = source.span(body.blocks());
    let uint_ty = match niche.bits {
        8 => UintTy::U8,
        16 => UintTy::U16,
        32 => UintTy::U32,
        64 => UintTy::U64,
        128 => UintTy::U128,
        _ => return,
    };
    let raw_ty = Ty::from_rigid_kind(RigidTy::Uint(uint_ty));
    // let raw: uN = transmute(value);
    let raw_lcl = body.new_local(raw_ty, span, Mutability::Not);
    body.assign_to(
        Place::from(raw_lcl),
        Rvalue::Cast(CastKind::Transmute, Operand::Copy(Place::from(place_local)), raw_ty),
        source,
        InsertPosition::Before,
    );
    let uint_const = |v: u128| {
        Operand::Constant(ConstOperand {
            span,
            user_ty: None,
            const_: MirConst::try_from_uint(v, uint_ty).unwrap(),
        })
    };
    let bool_ty = Ty::bool_ty();
    let ge_lcl = body.new_local(bool_ty, span, Mutability::Not);
    body.assign_to(
        Place::from(ge_lcl),
        Rvalue::BinaryOp(BinOp::Ge, Operand::Copy(Place::from(raw_lcl)), uint_const(niche.start)),
        source,
        InsertPosition::Before,
    );
    let le_lcl = body.new_local(bool_ty, span, Mutability::Not);
    body.assign_to(
        Place::from(le_lcl),
        Rvalue::BinaryOp(BinOp::Le, Operand::Copy(Place::from(raw_lcl)), uint_const(niche.end)),
        source,
        InsertPosition::Before,
    );
    // Contiguous range (start <= end): raw >= start && raw <= end.
    // Wrapping range (end < start, e.g. NonZero's 1..=0): raw >= start || raw <= end.
    let combine = if niche.start <= niche.end { BinOp::BitAnd } else { BinOp::BitOr };
    let cond_lcl = body.new_local(bool_ty, span, Mutability::Not);
    body.assign_to(
        Place::from(cond_lcl),
        Rvalue::BinaryOp(
            combine,
            Operand::Move(Place::from(ge_lcl)),
            Operand::Move(Place::from(le_lcl)),
        ),
        source,
        InsertPosition::Before,
    );
    let assume_inst = Instance::resolve(kani_assume, &GenericArgs(vec![])).unwrap();
    let unit_lcl = body.new_local(Ty::new_tuple(&[]), span, Mutability::Not);
    body.insert_call(
        &assume_inst,
        source,
        InsertPosition::Before,
        vec![Operand::Move(Place::from(cond_lcl))],
        Place::from(unit_lcl),
    );
}

/// The (optional, alloc-requiring) unbounded generation models, resolved per argument type.
#[derive(Debug, Clone, Copy, Default)]
pub struct UnboundedModels {
    slice_ref: Option<FnDef>,
    slice_mut: Option<FnDef>,
    vec: Option<FnDef>,
}

impl UnboundedModels {
    pub fn from_kani_functions(kani_fns: &std::collections::HashMap<KaniFunction, FnDef>) -> Self {
        UnboundedModels {
            slice_ref: kani_fns.get(&KaniModel::AnySliceRefUnbounded.into()).copied(),
            slice_mut: kani_fns.get(&KaniModel::AnySliceMutUnbounded.into()).copied(),
            vec: kani_fns.get(&KaniModel::AnyVecUnbounded.into()).copied(),
        }
    }

    /// The model instance generating `ty` unbounded, if `ty` qualifies.
    fn instance_for(&self, tcx: TyCtxt, ty: Ty) -> Option<Instance> {
        let (def, elem) = match ty.kind() {
            TyKind::RigidTy(RigidTy::Ref(_, inner, mutability)) => match inner.kind() {
                TyKind::RigidTy(RigidTy::Slice(elem))
                    if crate::kani_middle::slice_elem_unbounded_ok(tcx, elem) =>
                {
                    let def =
                        if mutability == Mutability::Not { self.slice_ref } else { self.slice_mut };
                    (def?, elem)
                }
                _ => return None,
            },
            _ => {
                let elem = crate::kani_middle::vec_elem_ty(ty)?;
                if !crate::kani_middle::slice_elem_unbounded_ok(tcx, elem) {
                    return None;
                }
                (self.vec?, elem)
            }
        };
        let instance =
            Instance::resolve(def, &GenericArgs(vec![GenericArgKind::Type(elem)])).ok()?;
        // Only use the model if its return type matches `ty` exactly (mirrors
        // `smart_pointer_model_instance`): guards against generating an ill-typed value if the
        // model's signature ever skews from the argument type (e.g. a `Vec<T, A>` with a
        // non-`Global` allocator that slipped past `vec_elem_ty`).
        let ret_ty = instance.ty().kind().fn_sig()?.skip_binder().output();
        (ret_ty == ty).then_some(instance)
    }
}

#[allow(clippy::too_many_arguments)]
fn call_kani_any_for_ty(
    tcx: TyCtxt,
    models: AnyModels,
    body: &mut MutableBody,
    ty: Ty,
    mutability: Mutability,
    source: &mut SourceInstruction,
    invariant_cache: &mut FxHashMap<Ty, bool>,
    mined_cache: &mut FxHashMap<Ty, Vec<MinedConjunct>>,
) -> Local {
    // Unbounded generation for slices (&[T]/&mut [T]) and Vec<T> of primitive
    // integer/float elements: fresh allocations of nondeterministic size, so results hold
    // for all lengths (mirrors the eligibility decision in automatic_harness_partition).
    if let Some(model_inst) = models.unbounded_models.instance_for(tcx, ty) {
        let lcl = body.new_local(ty, source.span(body.blocks()), mutability);
        body.insert_call(&model_inst, source, InsertPosition::Before, vec![], Place::from(lcl));
        return lcl;
    }
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
                    mined_cache,
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
            mined_cache,
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
            mined_cache,
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

        // Constrain the value to the type's layout niche, if any. This only reads `lcl`, so the
        // invariant assumption below can still move out of it.
        assume_scalar_niche(tcx, models.kani_assume, body, source, lcl, ty);

        // Under --constructor-args (the heuristic-filter umbrella), assume the type's mined
        // invariant conjuncts (its own methods' assertions) for the generated value. Reads
        // `lcl` via Copy, so the safety-invariant move below is unaffected.
        if models.constructor_args && matches!(ty.kind(), TyKind::RigidTy(RigidTy::Adt(..))) {
            let conjuncts = mined_cache
                .entry(ty)
                .or_insert_with(|| mine_self_assert_conjuncts(tcx, ty, models.kani_assert))
                .clone();
            if !conjuncts.is_empty() {
                assume_mined_invariants(tcx, models.kani_assume, body, source, lcl, ty, &conjuncts);
            }
        }

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
        mined_cache: &mut FxHashMap<Ty, Vec<MinedConjunct>>,
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
                mined_cache,
            );
            field_locals.push(lcl);
        }

        // Insert a basic block that constructs the variant from each of the nondet fields, then returns it
        body.insert_terminator(
            source,
            InsertPosition::Before,
            Terminator {
                kind: TerminatorKind::Return,
                source_info: synthetic_source_info(source.span(body.blocks())),
            },
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

    /// Overwrite the default `kani::any()` implementation `body` for a struct with private
    /// fields by inlining an assert-guarded representation constructor with nondeterministic
    /// arguments and panic paths converted into assumptions
    /// (c.f. [find_unchecked_constructor][crate::kani_middle::find_unchecked_constructor]).
    /// Returns None if the constructor body contains constructs the inliner does not support.
    ///
    /// Soundness caveat: if the constructor is unsatisfiable for `ty` (every argument trips an
    /// assertion), the generated body assumes `false` on all paths and the harness becomes
    /// vacuous, reporting Success without checking anything. Not yet detected; tracked in
    /// <https://github.com/model-checking/kani/issues/4757>.
    fn generate_unchecked_ctor_body(
        &self,
        tcx: TyCtxt,
        ctor: Instance,
        ty: Ty,
        body: Body,
    ) -> Option<Body> {
        let mut new_body = MutableBody::from(body);
        new_body.clear_body(TerminatorKind::Unreachable);
        let mut source = SourceInstruction::Terminator { bb: 0 };
        let ctor_sig = ctor.ty().kind().fn_sig().unwrap().skip_binder();
        let mut invariant_cache = FxHashMap::default();
        let mut mined_cache = FxHashMap::default();
        let arg_locals: Vec<Local> = ctor_sig
            .inputs()
            .iter()
            .map(|input_ty| {
                call_kani_any_for_ty(
                    tcx,
                    self.models,
                    &mut new_body,
                    *input_ty,
                    Mutability::Not,
                    &mut source,
                    &mut invariant_cache,
                    &mut mined_cache,
                )
            })
            .collect();
        let ret_lcl = inline_with_assumed_panics(
            tcx,
            self.models.kani_assume,
            self.models.kani_assert,
            &mut new_body,
            &mut source,
            ctor,
            &arg_locals,
            ty,
        )?;
        // RETURN_LOCAL = move ret; return
        new_body.assign_to(
            Place::from(0),
            Rvalue::Use(Operand::Move(Place::from(ret_lcl)), WithRetag::No),
            &mut source,
            InsertPosition::Before,
        );
        let span = source.span(new_body.blocks());
        new_body.insert_terminator(
            &mut source,
            InsertPosition::Before,
            Terminator { kind: TerminatorKind::Return, source_info: synthetic_source_info(span) },
        );
        Some(new_body.into())
    }

    /// Overwrite the default `kani::any()` implementation `body` for a struct with private
    /// fields by calling a public constructor with nondeterministic arguments
    /// (`--constructor-args`). The returned body is equivalent to:
    /// ```ignore
    /// // ctor returning Self:
    /// Ty::ctor(kani::any(), ..)
    /// // ctor returning Option<Self> (Result<Self, E> analogously):
    /// match Ty::ctor(kani::any(), ..) {
    ///     Some(v) => v,
    ///     None => { kani::assume(false); unreachable!() }
    /// }
    /// ```
    fn generate_ctor_body(
        &self,
        tcx: TyCtxt,
        ctor: Instance,
        shape: CtorReturn,
        ty: Ty,
        body: Body,
    ) -> Body {
        let mut new_body = MutableBody::from(body);
        new_body.clear_body(TerminatorKind::Unreachable);
        let mut source = SourceInstruction::Terminator { bb: 0 };

        let ctor_sig = ctor.ty().kind().fn_sig().unwrap().skip_binder();

        // Generate a nondeterministic value for every constructor argument.
        let mut invariant_cache = FxHashMap::default();
        let mut mined_cache = FxHashMap::default();
        let arg_ops: Vec<Operand> = ctor_sig
            .inputs()
            .iter()
            .map(|input_ty| {
                let lcl = call_kani_any_for_ty(
                    tcx,
                    self.models,
                    &mut new_body,
                    *input_ty,
                    Mutability::Not,
                    &mut source,
                    &mut invariant_cache,
                    &mut mined_cache,
                );
                Operand::Move(Place::from(lcl))
            })
            .collect();

        if shape == CtorReturn::Direct {
            // RETURN_LOCAL = ctor(args); return
            new_body.insert_call(
                &ctor,
                &mut source,
                InsertPosition::Before,
                arg_ops,
                Place::from(0),
            );
            let ret_span = source.span(new_body.blocks());
            new_body.insert_terminator(
                &mut source,
                InsertPosition::Before,
                Terminator {
                    kind: TerminatorKind::Return,
                    source_info: synthetic_source_info(ret_span),
                },
            );
            return new_body.into();
        }

        // Option<Self> / Result<Self, E>: call, switch on the discriminant, assume success.
        let ret_ty = ctor_sig.output();
        let TyKind::RigidTy(RigidTy::Adt(..)) = ret_ty.kind() else {
            unreachable!("constructor return shape guaranteed by find_arbitrary_constructor")
        };
        // Some = variant 1 of Option; Ok = variant 0 of Result. Both have discriminant
        // values equal to their variant indices.
        let ok_idx = match shape {
            CtorReturn::OptionOf => 1usize,
            CtorReturn::ResultOf => 0usize,
            CtorReturn::Direct => unreachable!(),
        };
        // `VariantDef::idx` is no longer publicly accessible, so reconstruct it from the
        // enumeration order, as `generate_enum_body` does.
        let ok_variant_idx = VariantIdx::to_val(ok_idx);

        let span = source.span(new_body.blocks());
        let ret_lcl = new_body.new_local(ret_ty, span, Mutability::Not);
        new_body.insert_call(
            &ctor,
            &mut source,
            InsertPosition::Before,
            arg_ops,
            Place::from(ret_lcl),
        );

        // Read the discriminant.
        let discr_ty = ret_ty.kind().discriminant_ty().unwrap();
        let discr_lcl = new_body.new_local(discr_ty, span, Mutability::Not);
        new_body.assign_to(
            Place::from(discr_lcl),
            Rvalue::Discriminant(Place::from(ret_lcl)),
            &mut source,
            InsertPosition::Before,
        );

        // Placeholder for the SwitchInt terminator.
        let span = source.span(new_body.blocks());
        new_body.insert_terminator(
            &mut source,
            InsertPosition::Before,
            Terminator {
                kind: TerminatorKind::Unreachable,
                source_info: synthetic_source_info(span),
            },
        );
        let switch_instr = SourceInstruction::Terminator { bb: source.bb() - 1 };

        // Failure branch: kani::assume(false); unreachable.
        let assume_inst = Instance::resolve(self.models.kani_assume, &GenericArgs(vec![])).unwrap();
        let false_op = Operand::Constant(ConstOperand {
            span,
            user_ty: None,
            const_: MirConst::from_bool(false),
        });
        let unit_lcl = new_body.new_local(Ty::new_tuple(&[]), span, Mutability::Not);
        new_body.insert_call(
            &assume_inst,
            &mut source,
            InsertPosition::Before,
            vec![false_op],
            Place::from(unit_lcl),
        );
        new_body.insert_terminator(
            &mut source,
            InsertPosition::Before,
            Terminator {
                kind: TerminatorKind::Unreachable,
                source_info: synthetic_source_info(span),
            },
        );
        // insert_call + terminator added two blocks; the failure branch starts at the first.
        let bad_bb = source.bb() - 2;

        // Success branch: RETURN_LOCAL = move (ret as OkVariant).0; return.
        let payload_place = Place {
            local: ret_lcl,
            projection: vec![
                ProjectionElem::Downcast(ok_variant_idx),
                ProjectionElem::Field(0, ty),
            ],
        };
        new_body.insert_terminator(
            &mut source,
            InsertPosition::Before,
            Terminator { kind: TerminatorKind::Return, source_info: synthetic_source_info(span) },
        );
        let ok_bb = source.bb() - 1;
        let mut assign_instr = SourceInstruction::Terminator { bb: ok_bb };
        new_body.assign_to(
            Place::from(0),
            Rvalue::Use(Operand::Move(payload_place), WithRetag::No),
            &mut assign_instr,
            InsertPosition::Before,
        );

        let switch = Terminator {
            kind: TerminatorKind::SwitchInt {
                discr: Operand::Copy(Place::from(discr_lcl)),
                targets: SwitchTargets::new(vec![(ok_idx as u128, ok_bb)], bad_bb),
            },
            source_info: synthetic_source_info(span),
        };
        new_body.replace_terminator(&switch_instr, switch);

        new_body.into()
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
        let mut mined_cache = FxHashMap::default();

        // Generate a nondet u128 to switch on
        let discr_lcl = call_kani_any_for_ty(
            tcx,
            self.models,
            &mut new_body,
            Ty::from_rigid_kind(RigidTy::Uint(UintTy::U128)),
            Mutability::Not,
            &mut source,
            &mut invariant_cache,
            &mut mined_cache,
        );

        // Insert a placeholder for the SwitchInt terminator
        let span = source.span(new_body.blocks());
        new_body.insert_terminator(
            &mut source,
            InsertPosition::Before,
            Terminator {
                kind: TerminatorKind::Unreachable,
                source_info: synthetic_source_info(span),
            },
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
                &mut mined_cache,
            );
            branches.push((idx as u128, target_bb));
        }

        let otherwise = branches.pop().unwrap().1;
        let match_term = Terminator {
            kind: TerminatorKind::SwitchInt {
                discr: Operand::Copy(Place::from(discr_lcl)),
                targets: SwitchTargets::new(branches, otherwise),
            },
            source_info: synthetic_source_info(source.span(new_body.blocks())),
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
        let mut mined_cache = FxHashMap::default();

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
            &mut mined_cache,
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
    /// Whether --check-invariants is enabled: check values returned by verified functions
    /// against the type's mined invariant conjuncts.
    check_invariants: bool,
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
        let check_invariants = query_db.args().autoharness_check_invariants;
        Self {
            models: AnyModels::new(query_db),
            kani_check_debug_fmt,
            kani_check_display_fmt,
            init_contracts_hook,
            reset_clause_depth,
            kani_autoharness_intrinsic,
            check_invariants,
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
        let mut mined_cache: FxHashMap<Ty, Vec<MinedConjunct>> = FxHashMap::default();

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
                &mut mined_cache,
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
                    &mut mined_cache,
                )
            })
            .collect::<Vec<_>>();

        let func_to_verify_ret = fn_to_verify_body.ret_local();
        let ret_lcl = harness_body.new_local(
            func_to_verify_ret.ty,
            source.span(harness_body.blocks()),
            func_to_verify_ret.mutability,
        );
        let ret_place = Place::from(ret_lcl);

        // Call `fn_to_verify` on the nondeterministic arguments generated above.
        harness_body.insert_call(
            &fn_to_verify,
            &mut source,
            InsertPosition::Before,
            arg_locals.iter().map(|lcl| Operand::Copy(Place::from(*lcl))).collect::<Vec<_>>(),
            ret_place,
        );

        // Under --check-invariants, check the return value against the type's mined
        // invariants. Direct T and &T returns are handled, along with the payloads of
        // Option<T>/Result<T, E> (None/Err pass vacuously via a discriminant guard).
        if self.check_invariants {
            let ret_ty = func_to_verify_ret.ty;
            // Peel the return type down to a checkable ADT value: direct T, &T, and the
            // payloads of Option<T>/Result<T, E>. For the latter two, the payload is read
            // via a downcast (harmless byte-level read on the other variant) and every
            // conjunct is guarded with `discriminant != Some/Ok || conjunct`, making the
            // check vacuously true for None/Err returns.
            let mut wrapper_guard: Option<(usize, Ty)> = None; // (ok variant idx, wrapper ty)
            let (check_ty, check_lcl) = match ret_ty.kind() {
                TyKind::RigidTy(RigidTy::Ref(_, inner, _))
                    if matches!(inner.kind(), TyKind::RigidTy(RigidTy::Adt(..))) =>
                {
                    // Deref into a temp of the pointee type.
                    let span = source.span(harness_body.blocks());
                    let tmp = harness_body.new_local(inner, span, Mutability::Not);
                    harness_body.assign_to(
                        Place::from(tmp),
                        Rvalue::Use(
                            Operand::Copy(Place {
                                local: ret_lcl,
                                projection: vec![ProjectionElem::Deref],
                            }),
                            WithRetag::No,
                        ),
                        &mut source,
                        InsertPosition::Before,
                    );
                    (inner, Some(tmp))
                }
                TyKind::RigidTy(RigidTy::Adt(d, ref wargs))
                    if matches!(
                        d.name().as_str(),
                        "std::option::Option"
                            | "core::option::Option"
                            | "std::result::Result"
                            | "core::result::Result"
                    ) =>
                {
                    let ok_idx = if d.name().contains("Option") { 1 } else { 0 };
                    let payload = wargs.0.iter().find_map(|a| match a {
                        GenericArgKind::Type(t) => Some(*t),
                        _ => None,
                    });
                    match payload {
                        Some(pt) if matches!(pt.kind(), TyKind::RigidTy(RigidTy::Adt(..))) => {
                            let span = source.span(harness_body.blocks());
                            let tmp = harness_body.new_local(pt, span, Mutability::Not);
                            harness_body.assign_to(
                                Place::from(tmp),
                                Rvalue::Use(
                                    Operand::Copy(Place {
                                        local: ret_lcl,
                                        projection: vec![
                                            ProjectionElem::Downcast(
                                                rustc_public::ty::VariantIdx::to_val(ok_idx),
                                            ),
                                            ProjectionElem::Field(0, pt),
                                        ],
                                    }),
                                    WithRetag::No,
                                ),
                                &mut source,
                                InsertPosition::Before,
                            );
                            wrapper_guard = Some((ok_idx, ret_ty));
                            (pt, Some(tmp))
                        }
                        _ => (ret_ty, None),
                    }
                }
                TyKind::RigidTy(RigidTy::Adt(..)) => (ret_ty, Some(ret_lcl)),
                _ => (ret_ty, None),
            };
            if let Some(lcl) = check_lcl {
                let conjuncts = mined_cache
                    .entry(check_ty)
                    .or_insert_with(|| {
                        mine_self_assert_conjuncts(tcx, check_ty, self.models.kani_assert)
                    })
                    .clone();
                if !conjuncts.is_empty() {
                    check_mined_invariants(
                        tcx,
                        self.models.kani_assert,
                        &mut harness_body,
                        &mut source,
                        lcl,
                        check_ty,
                        wrapper_guard.map(|(ok, wty)| (ret_lcl, ok, wty)),
                        &conjuncts,
                    );
                }
            }
        }

        (true, harness_body.into())
    }
}
