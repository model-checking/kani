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
use crate::kani_middle::transform::body::{InsertPosition, MutableBody, SourceInstruction};
use crate::kani_middle::transform::{TransformPass, TransformationType};
use crate::kani_middle::{
    CtorReturn, adt_has_private_field_check, find_arbitrary_constructor, implements_arbitrary,
    scalar_niche,
};
use crate::kani_queries::QueryDb;
use rustc_data_structures::fx::FxHashMap;
use rustc_middle::ty::TyCtxt;
use rustc_public::CrateDef;
use rustc_public::mir::mono::Instance;
use rustc_public::mir::{
    AggregateKind, BasicBlock, BasicBlockIdx, BinOp, Body, BorrowKind, CastKind, ConstOperand,
    Local, MutBorrowKind, Mutability, NonDivergingIntrinsic, Operand, Place, ProjectionElem,
    Rvalue, Statement, StatementKind, SwitchTargets, Terminator, TerminatorKind, UnOp,
    UnwindAction,
};
use rustc_public::ty::{
    AdtDef, AdtKind, FnDef, GenericArgKind, GenericArgs, MirConst, RigidTy, Ty, TyKind, UintTy,
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
    /// The FnDef of KaniHook::Assume (used for layout-niche assumptions and constructor
    /// success).
    kani_assume: FnDef,
    /// The FnDef of KaniHook::Assert (rewritten into assumptions when inlining
    /// assert-guarded constructors).
    kani_assert: FnDef,
    /// Whether --constructor-args is enabled: generate values of private-field types through
    /// their public constructors instead of raw field synthesis.
    constructor_args: bool,
    /// The (optional) unbounded generation models.
    unbounded_models: UnboundedModels,
}

impl AutomaticArbitraryPass {
    pub fn new(_unit: &CodegenUnit, query_db: &QueryDb) -> Self {
        let kani_fns = query_db.kani_functions();
        let kani_any = *kani_fns.get(&KaniModel::Any.into()).unwrap();
        let kani_assume = *kani_fns.get(&KaniHook::Assume.into()).unwrap();
        let kani_assert = *kani_fns.get(&KaniHook::Assert.into()).unwrap();
        let constructor_args = query_db.args().autoharness_constructor_args;
        let unbounded_models = UnboundedModels::from_kani_functions(kani_fns);
        Self { kani_any, kani_assume, kani_assert, constructor_args, unbounded_models }
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
                    self.kani_any,
                    &mut FxHashMap::default(),
                ) && let Some(new_body) =
                    self.generate_unchecked_ctor_body(tcx, ctor, *ty, body.clone())
                {
                    debug!(?ty, ctor=?ctor.name(), "generate_unchecked_ctor_body");
                    return (true, new_body);
                }
                if let Some((ctor, shape)) =
                    find_arbitrary_constructor(tcx, *ty, self.kani_any, &mut FxHashMap::default())
                {
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

/// Insert a call to kani::any::<ty>() in `body`; return the local storing the result.
/// Panics if `ty` does not implement Arbitrary.
/// Remap all locals and block targets of an inlined basic block. Returns false (bail out)
/// when the block contains a construct the remapper does not support; the caller then falls
/// back to non-inlined generation. The whitelist covers everything rustc emits for
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
                    Rvalue::Use(op) | Rvalue::Repeat(op, _) | Rvalue::Cast(_, op, _) => {
                        remap_operand(op)
                    }
                    Rvalue::BinaryOp(_, a, b) | Rvalue::CheckedBinaryOp(_, a, b) => {
                        remap_operand(a);
                        remap_operand(b);
                    }
                    Rvalue::UnaryOp(_, op) => remap_operand(op),
                    Rvalue::Ref(_, _, p)
                    | Rvalue::AddressOf(_, p)
                    | Rvalue::CopyForDeref(p)
                    | Rvalue::Discriminant(p)
                    | Rvalue::Len(p) => remap_place(p),
                    Rvalue::Aggregate(_, ops) => ops.iter_mut().for_each(remap_operand),
                    Rvalue::ShallowInitBox(op, _) => remap_operand(op),
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
            | StatementKind::Retag(..)
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
        TerminatorKind::Call { func, args, destination, target, .. } => {
            remap_operand(func);
            args.iter_mut().for_each(remap_operand);
            remap_place(destination);
            if let Some(t) = target {
                *t += block_offset;
            }
        }
        TerminatorKind::Assert { cond, target, .. } => {
            remap_operand(cond);
            *target += block_offset;
        }
        TerminatorKind::Drop { place, target, .. } => {
            remap_place(place);
            *target += block_offset;
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
                                || def.name().contains("assert_unchecked")
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
                            Rvalue::Use(cond)
                        } else {
                            Rvalue::UnaryOp(UnOp::Not, cond)
                        };
                        bb.statements.push(Statement {
                            kind: StatementKind::Assign(Place::from(cond_lcl), rv),
                            span: self.span,
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
                            && (*def == self.kani_assert || def.name().contains("assert_unchecked"))
                        {
                            // kani::assert(cond, msg) / assert_unchecked(cond) -> assume(cond)
                            let cond = args[0].clone();
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
                                        span: self.span,
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
                                        Rvalue::Use(arg_op.clone()),
                                    ),
                                    span: self.span,
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
                                            Rvalue::Use(Operand::Move(Place::from(inner_ret_lcl))),
                                        ),
                                        span: self.span,
                                    }],
                                    terminator: Terminator {
                                        kind: TerminatorKind::Goto { target },
                                        span: self.span,
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
                                                    span: self.span,
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
    let placeholder = Terminator { kind: TerminatorKind::Goto { target: outer_base }, span };
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
                Rvalue::Use(Operand::Copy(place)),
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
                Rvalue::Use(Operand::Copy(place)),
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
                Rvalue::Use(Operand::Constant(c.clone())),
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
    let uint_ty = match niche_bits {
        8 => UintTy::U8,
        16 => UintTy::U16,
        32 => UintTy::U32,
        64 => UintTy::U64,
        128 => UintTy::U128,
        _ => return Some(raw),
    };
    let raw_ty = Ty::from_rigid_kind(RigidTy::Uint(uint_ty));
    let discr_uint = body.new_local(raw_ty, span, Mutability::Not);
    body.assign_to(
        Place::from(discr_uint),
        Rvalue::Cast(CastKind::Transmute, Operand::Copy(Place::from(discr_lcl)), raw_ty),
        source,
        InsertPosition::Before,
    );
    let TyKind::RigidTy(RigidTy::Adt(adt_def, _)) = value_ty.kind() else { return Some(raw) };
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

/// If `ty` has a scalar layout with a restricted valid range (a niche), append
/// `kani::assume(<raw bits of the value in place_local> in valid_range)`.
/// Values outside the niche are language-level invalid (rustc packs enum variants into the
/// invalid patterns), so nondeterministic-value generation must never produce them: e.g.
/// std's `NonZero` niches, or `core::time::Duration`'s `Nanoseconds` field
/// (`rustc_layout_scalar_valid_range` types), whose compiler-derived generation would
/// otherwise produce invalid values and false alarms in every harness generating the type.
/// The assumption is sound by construction and requires no reporting caveat.
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
#[derive(Debug, Clone, Default)]
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
        Instance::resolve(def, &GenericArgs(vec![GenericArgKind::Type(elem)])).ok()
    }
}

fn call_kani_any_for_ty(
    tcx: TyCtxt,
    kani_any: FnDef,
    kani_assume: FnDef,
    kani_assert: FnDef,
    constructor_args: bool,
    mined_cache: &mut FxHashMap<Ty, Vec<MinedConjunct>>,
    unbounded_models: &UnboundedModels,
    body: &mut MutableBody,
    ty: Ty,
    mutability: Mutability,
    source: &mut SourceInstruction,
) -> Local {
    // Unbounded generation for slices (&[T]/&mut [T]) and Vec<T> of primitive
    // integer/float elements: fresh allocations of nondeterministic size, so results hold
    // for all lengths (mirrors the eligibility decision in automatic_harness_partition).
    if let Some(model_inst) = unbounded_models.instance_for(tcx, ty) {
        let lcl = body.new_local(ty, source.span(body.blocks()), mutability);
        body.insert_call(&model_inst, source, InsertPosition::Before, vec![], Place::from(lcl));
        return lcl;
    }
    if let TyKind::RigidTy(RigidTy::Ref(region, inner_ty, inner_mutability)) = ty.kind() {
        let inner_lcl = call_kani_any_for_ty(
            tcx,
            kani_any,
            kani_assume,
            kani_assert,
            constructor_args,
            mined_cache,
            unbounded_models,
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
        let kani_any_inst =
            Instance::resolve(kani_any, &GenericArgs(vec![GenericArgKind::Type(ty)]))
                .unwrap_or_else(|_| panic!("expected a ty that implements Arbitrary, got {ty}"));
        let lcl = body.new_local(ty, source.span(body.blocks()), mutability);
        body.insert_call(&kani_any_inst, source, InsertPosition::Before, vec![], Place::from(lcl));
        // Constrain the value to the type's layout niche, if any.
        assume_scalar_niche(tcx, kani_assume, body, source, lcl, ty);

        // Under --constructor-args (the heuristic-filter umbrella), assume the type's
        // mined invariant conjuncts (its own methods' assertions) for the generated value.
        if constructor_args {
            if matches!(ty.kind(), TyKind::RigidTy(RigidTy::Adt(..))) {
                let conjuncts = mined_cache
                    .entry(ty)
                    .or_insert_with(|| mine_self_assert_conjuncts(tcx, ty, kani_assert));
                if !conjuncts.is_empty() {
                    let conjuncts = conjuncts.clone();
                    assume_mined_invariants(tcx, kani_assume, body, source, lcl, ty, &conjuncts);
                }
            }
        }
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
        mined_cache: &mut FxHashMap<Ty, Vec<MinedConjunct>>,
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
                self.kani_assume,
                self.kani_assert,
                self.constructor_args,
                mined_cache,
                &self.unbounded_models,
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

    /// Overwrite the default `kani::any()` implementation `body` for a struct with private
    /// fields by inlining an assert-guarded representation constructor with nondeterministic
    /// arguments and panic paths converted into assumptions
    /// (c.f. [find_unchecked_constructor][crate::kani_middle::find_unchecked_constructor]).
    /// Returns None if the constructor body contains constructs the inliner does not support.
    fn generate_unchecked_ctor_body(
        &self,
        tcx: TyCtxt,
        ctor: Instance,
        ty: Ty,
        body: Body,
    ) -> Option<Body> {
        let mut new_body = MutableBody::from(body);
        new_body.clear_body(TerminatorKind::Unreachable);
        let mut mined_cache: FxHashMap<Ty, Vec<MinedConjunct>> = FxHashMap::default();
        let mut source = SourceInstruction::Terminator { bb: 0 };
        let ctor_sig = ctor.ty().kind().fn_sig().unwrap().skip_binder();
        let arg_locals: Vec<Local> = ctor_sig
            .inputs()
            .iter()
            .map(|input_ty| {
                call_kani_any_for_ty(
                    tcx,
                    self.kani_any,
                    self.kani_assume,
                    self.kani_assert,
                    self.constructor_args,
                    &mut mined_cache,
                    &self.unbounded_models,
                    &mut new_body,
                    *input_ty,
                    Mutability::Not,
                    &mut source,
                )
            })
            .collect();
        let ret_lcl = inline_with_assumed_panics(
            tcx,
            self.kani_assume,
            self.kani_assert,
            &mut new_body,
            &mut source,
            ctor,
            &arg_locals,
            ty,
        )?;
        // RETURN_LOCAL = move ret; return
        new_body.assign_to(
            Place::from(0),
            Rvalue::Use(Operand::Move(Place::from(ret_lcl))),
            &mut source,
            InsertPosition::Before,
        );
        let span = source.span(new_body.blocks());
        new_body.insert_terminator(
            &mut source,
            InsertPosition::Before,
            Terminator { kind: TerminatorKind::Return, span },
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
        let mut mined_cache: FxHashMap<Ty, Vec<MinedConjunct>> = FxHashMap::default();
        let mut source = SourceInstruction::Terminator { bb: 0 };

        let ctor_sig = ctor.ty().kind().fn_sig().unwrap().skip_binder();

        // Generate a nondeterministic value for every constructor argument.
        let arg_ops: Vec<Operand> = ctor_sig
            .inputs()
            .iter()
            .map(|input_ty| {
                let lcl = call_kani_any_for_ty(
                    tcx,
                    self.kani_any,
                    self.kani_assume,
                    self.kani_assert,
                    self.constructor_args,
                    &mut mined_cache,
                    &self.unbounded_models,
                    &mut new_body,
                    *input_ty,
                    Mutability::Not,
                    &mut source,
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
                Terminator { kind: TerminatorKind::Return, span: ret_span },
            );
            return new_body.into();
        }

        // Option<Self> / Result<Self, E>: call, switch on the discriminant, assume success.
        let ret_ty = ctor_sig.output();
        let TyKind::RigidTy(RigidTy::Adt(wrap_def, _)) = ret_ty.kind() else {
            unreachable!("constructor return shape guaranteed by find_arbitrary_constructor")
        };
        // Some = variant 1 of Option; Ok = variant 0 of Result. Both have discriminant
        // values equal to their variant indices.
        let ok_idx = match shape {
            CtorReturn::OptionOf => 1usize,
            CtorReturn::ResultOf => 0usize,
            CtorReturn::Direct => unreachable!(),
        };
        let ok_variant = wrap_def.variants()[ok_idx];

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
            Terminator { kind: TerminatorKind::Unreachable, span },
        );
        let switch_instr = SourceInstruction::Terminator { bb: source.bb() - 1 };

        // Failure branch: kani::assume(false); unreachable.
        let assume_inst = Instance::resolve(self.kani_assume, &GenericArgs(vec![])).unwrap();
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
            Terminator { kind: TerminatorKind::Unreachable, span },
        );
        // insert_call + terminator added two blocks; the failure branch starts at the first.
        let bad_bb = source.bb() - 2;

        // Success branch: RETURN_LOCAL = move (ret as OkVariant).0; return.
        let payload_place = Place {
            local: ret_lcl,
            projection: vec![
                ProjectionElem::Downcast(ok_variant.idx),
                ProjectionElem::Field(0, ty),
            ],
        };
        new_body.insert_terminator(
            &mut source,
            InsertPosition::Before,
            Terminator { kind: TerminatorKind::Return, span },
        );
        let ok_bb = source.bb() - 1;
        let mut assign_instr = SourceInstruction::Terminator { bb: ok_bb };
        new_body.assign_to(
            Place::from(0),
            Rvalue::Use(Operand::Move(payload_place)),
            &mut assign_instr,
            InsertPosition::Before,
        );

        let switch = Terminator {
            kind: TerminatorKind::SwitchInt {
                discr: Operand::Copy(Place::from(discr_lcl)),
                targets: SwitchTargets::new(
                    vec![(ok_variant.idx.to_index() as u128, ok_bb)],
                    bad_bb,
                ),
            },
            span,
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
        let mut mined_cache: FxHashMap<Ty, Vec<MinedConjunct>> = FxHashMap::default();
        let mut source = SourceInstruction::Terminator { bb: 0 };

        // Generate a nondet u128 to switch on
        let discr_lcl = call_kani_any_for_ty(
            tcx,
            self.kani_any,
            self.kani_assume,
            self.kani_assert,
            self.constructor_args,
            &mut mined_cache,
            &self.unbounded_models,
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
                &mut mined_cache,
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
        let mut mined_cache: FxHashMap<Ty, Vec<MinedConjunct>> = FxHashMap::default();
        let mut source = SourceInstruction::Terminator { bb: 0 };

        let variant = def.variants()[0];
        self.call_kani_any_for_variant(
            tcx,
            &mut mined_cache,
            def,
            &args,
            &mut new_body,
            &mut source,
            variant,
        );

        new_body.into()
    }
}
/// Transform the dummy body of an automatic_harness Kani intrinsic to be a proof harness for a given function.
#[derive(Debug, Clone)]
pub struct AutomaticHarnessPass {
    kani_assert: FnDef,
    constructor_args: bool,
    check_invariants: bool,
    /// The FnDef of KaniHook::Assume (used for layout-niche assumptions).
    kani_assume: FnDef,
    kani_any: FnDef,
    init_contracts_hook: Instance,
    kani_autoharness_intrinsic: FnDef,
    unbounded_models: UnboundedModels,
}

impl AutomaticHarnessPass {
    pub fn new(query_db: &QueryDb) -> Self {
        let kani_fns = query_db.kani_functions();
        let kani_assume = *kani_fns.get(&KaniHook::Assume.into()).unwrap();
        let kani_autoharness_intrinsic =
            *kani_fns.get(&KaniIntrinsic::AutomaticHarness.into()).unwrap();
        let kani_any = *kani_fns.get(&KaniModel::Any.into()).unwrap();
        let kani_assert = *kani_fns.get(&KaniHook::Assert.into()).unwrap();
        let constructor_args = query_db.args().autoharness_constructor_args;
        let check_invariants = query_db.args().autoharness_check_invariants;
        let unbounded_models = UnboundedModels::from_kani_functions(kani_fns);
        let init_contracts_hook = *kani_fns.get(&KaniHook::InitContracts.into()).unwrap();
        let init_contracts_hook =
            Instance::resolve(init_contracts_hook, &GenericArgs(vec![])).unwrap();
        Self {
            kani_assume,
            kani_any,
            kani_assert,
            constructor_args,
            check_invariants,
            unbounded_models,
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
                    self.kani_assume,
                    self.kani_assert,
                    self.constructor_args,
                    &mut mined_cache,
                    &self.unbounded_models,
                    &mut harness_body,
                    local_decl.ty,
                    local_decl.mutability,
                    &mut source,
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
        // invariants. Direct T and &T returns are handled (Option/Result peeling is a
        // logged follow-up).
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
                        Rvalue::Use(Operand::Copy(Place {
                            local: ret_lcl,
                            projection: vec![ProjectionElem::Deref],
                        })),
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
                                Rvalue::Use(Operand::Copy(Place {
                                    local: ret_lcl,
                                    projection: vec![
                                        ProjectionElem::Downcast(
                                            rustc_public::ty::VariantIdx::to_val(ok_idx),
                                        ),
                                        ProjectionElem::Field(0, pt),
                                    ],
                                })),
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
                    .or_insert_with(|| mine_self_assert_conjuncts(tcx, check_ty, self.kani_assert))
                    .clone();
                if !conjuncts.is_empty() {
                    check_mined_invariants(
                        tcx,
                        self.kani_assert,
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
