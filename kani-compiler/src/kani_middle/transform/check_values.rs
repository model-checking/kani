// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
//! Implement a transformation pass that instrument the code to detect possible UB due to
//! the generation of an invalid value.
//!
//! This pass highly depend on Rust type layouts. For more details, see:
//! <https://doc.rust-lang.org/reference/type-layout.html>
//!
//! For that, we traverse the function body and look for unsafe operations that may generate
//! invalid values. For each operation found, we add checks to ensure the value is valid.
//!
//! Note: There is some redundancy in the checks that could be optimized. Example:
//!   1. We could merge the invalid values by the offset.
//!   2. We could avoid checking places that have been checked before.
use crate::args::ExtraChecks;
use crate::kani_middle::attributes::attrs_by_path;
use crate::kani_middle::kani_functions::{KaniFunction, KaniIntrinsic, KaniModel};
use crate::kani_middle::transform::body::{
    CheckType, InsertPosition, MutableBody, SourceInstruction, synthetic_source_info,
};
use crate::kani_middle::transform::{TransformPass, TransformationType};
use crate::kani_queries::QueryDb;
use rustc_middle::ty::TyCtxt;
use rustc_public::CrateDef;
use rustc_public::CrateDefType;
use rustc_public::abi::{
    FieldsShape, Scalar, TagEncoding, ValueRepr, VariantsShape, WrappingRange,
};
use rustc_public::mir::mono::Instance;
use rustc_public::mir::visit::{Location, PlaceContext, PlaceRef};
use rustc_public::mir::{
    AggregateKind, BasicBlock, BasicBlockIdx, BinOp, Body, BorrowKind, CastKind, ConstOperand,
    FieldIdx, Local, LocalDecl, MirVisitor, Mutability, NonDivergingIntrinsic, Operand, Place,
    ProjectionElem, RawPtrKind, Rvalue, Statement, StatementKind, SwitchTargets, Terminator,
    TerminatorKind, UnOp, UnwindAction, WithRetag,
};
use rustc_public::rustc_internal;
use rustc_public::target::{MachineInfo, MachineSize};
use rustc_public::ty::{AdtKind, GenericArgs, RigidTy, Span, Ty, TyKind, UintTy, VariantDef};
use rustc_public::ty::{FnDef, GenericArgKind, MirConst};
use rustc_public_bridge::IndexedVal;
use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use strum_macros::AsRefStr;
use tracing::{debug, trace};

/// Instrument the code with checks for invalid values.
#[derive(Debug, Clone)]
pub struct ValidValuePass {
    pub safety_check_type: CheckType,
    pub unsupported_check_type: CheckType,
    /// Kani's functions, for the model of untyped writes (`kani_core::kani_valid_value!`).
    pub kani_functions: HashMap<KaniFunction, FnDef>,
}

impl TransformPass for ValidValuePass {
    fn transformation_type() -> TransformationType
    where
        Self: Sized,
    {
        TransformationType::Instrumentation
    }

    fn is_enabled(&self, query_db: &QueryDb) -> bool
    where
        Self: Sized,
    {
        let args = query_db.args();
        args.ub_check.contains(&ExtraChecks::Validity)
    }

    /// Transform the function body by inserting checks one-by-one.
    /// For every unsafe dereference or a transmute operation, we check all values are valid.
    fn transform(&mut self, tcx: TyCtxt, body: Body, instance: Instance) -> (bool, Body) {
        trace!(function=?instance.name(), "transform");
        let harness = is_harness(instance, tcx);
        // Kani's own functions are not instrumented, nor are the instrumentation models of the
        // validity and initialization checks, which have helpers without markers: the model of
        // untyped writes reads its state through a reference, and checking that read would call
        // the model again.
        if !harness
            && (KaniFunction::try_from(instance).is_ok() || is_instrumentation_model(instance))
        {
            return (false, body);
        }
        let mut new_body = MutableBody::from(body);
        let orig_len = new_body.blocks().len();
        let address_taken = address_taken_locals(&new_body);
        // The checks split blocks, and the instructions after an instrumented one end up in a new
        // block at the end, so the number of blocks is read on every iteration.
        let mut bb_idx = 0;
        while bb_idx < new_body.blocks().len() {
            let Some(candidate) = CheckValueVisitor::find_next(
                tcx,
                &new_body,
                bb_idx,
                bb_idx >= orig_len,
                &address_taken,
            ) else {
                bb_idx += 1;
                continue;
            };
            self.build_check(&mut new_body, candidate);
            bb_idx += 1;
        }
        if harness {
            self.initialize_untyped_write_state(&mut new_body);
        }
        (harness || orig_len != new_body.blocks().len(), new_body.into())
    }
}

impl ValidValuePass {
    fn build_check(&self, body: &mut MutableBody, instruction: UnsafeInstruction) {
        debug!(?instruction, "build_check");
        let mut source = instruction.source;
        for operation in instruction.operations {
            match operation {
                SourceOp::BytesValidity { ranges, target_ty, rvalue } => {
                    let value = body.insert_assignment(rvalue, &mut source, InsertPosition::Before);
                    let rvalue_ptr = Rvalue::AddressOf(RawPtrKind::Const, Place::from(value));
                    for range in ranges {
                        let result = build_limits(body, &range, rvalue_ptr.clone(), &mut source);
                        let msg =
                            format!("Undefined Behavior: Invalid value of type `{target_ty}`",);
                        body.insert_check(
                            &self.safety_check_type,
                            &mut source,
                            InsertPosition::Before,
                            Some(result),
                            &msg,
                        );
                    }
                }
                SourceOp::DerefValidity { pointee_ty, rvalue, ranges } => {
                    for range in ranges {
                        let result = build_limits(body, &range, rvalue.clone(), &mut source);
                        let msg =
                            format!("Undefined Behavior: Invalid value of type `{pointee_ty}`",);
                        body.insert_check(
                            &self.safety_check_type,
                            &mut source,
                            InsertPosition::Before,
                            Some(result),
                            &msg,
                        );
                    }
                }
                SourceOp::UnsupportedCheck { check, ty } => {
                    let reason = format!(
                        "Kani currently doesn't support checking validity of `{check}` for `{ty}`",
                    );
                    self.unsupported_check(body, &mut source, &reason);
                }
                SourceOp::SetUntyped { ptr, count, elem_size, untyped } => {
                    let span = source.span(body.blocks());
                    let ptr = body.insert_assignment(ptr, &mut source, InsertPosition::Before);
                    let pointee = pointee_ty(body.locals()[ptr].ty);
                    let count =
                        count.unwrap_or_else(|| body.new_uint_operand(1, UintTy::Usize, span));
                    let elem_size = body.new_uint_operand(elem_size as u128, UintTy::Usize, span);
                    let untyped = bool_operand(untyped, span);
                    self.call_model(
                        body,
                        &mut source,
                        KaniModel::SetUntypedWrite,
                        pointee,
                        vec![Operand::Move(Place::from(ptr)), count, elem_size, untyped],
                        Ty::new_tuple(&[]),
                    );
                }
                SourceOp::UntypedLoad { ptr, dest, ty, ranges } => {
                    let span = source.span(body.blocks());
                    let ptr = body.insert_assignment(
                        Rvalue::Use(ptr, WithRetag::No),
                        &mut source,
                        InsertPosition::Before,
                    );
                    let size = ty.layout().unwrap().shape().size.bytes() as u128;
                    let size = body.new_uint_operand(size, UintTy::Usize, span);
                    let untyped = self.call_model(
                        body,
                        &mut source,
                        KaniModel::IsUntypedWrite,
                        ty,
                        vec![Operand::Copy(Place::from(ptr)), size],
                        Ty::bool_ty(),
                    );
                    // Continue after the call, where `dest` holds the loaded value.
                    let dest_ptr = body.insert_assignment(
                        Rvalue::AddressOf(RawPtrKind::Const, dest),
                        &mut source,
                        InsertPosition::After,
                    );
                    // Insert the checks after that assignment, before the new block's `goto`.
                    source = SourceInstruction::Terminator { bb: source.bb() };
                    self.check_untyped_value(body, &mut source, dest_ptr, untyped, ty, ranges);
                }
                SourceOp::UntypedRead { place, ty, ranges } => {
                    let span = source.span(body.blocks());
                    let ptr = body.insert_assignment(
                        Rvalue::AddressOf(RawPtrKind::Const, place),
                        &mut source,
                        InsertPosition::Before,
                    );
                    let size = ty.layout().unwrap().shape().size.bytes() as u128;
                    let size = body.new_uint_operand(size, UintTy::Usize, span);
                    let untyped = self.call_model(
                        body,
                        &mut source,
                        KaniModel::IsUntypedWrite,
                        ty,
                        vec![Operand::Copy(Place::from(ptr)), size],
                        Ty::bool_ty(),
                    );
                    self.check_untyped_value(body, &mut source, ptr, untyped, ty, ranges);
                }
            }
        }
    }

    /// Check that the `ty` at `ptr` is valid if `untyped` holds, or report `ty` as unsupported if
    /// Kani can't express its validity (`ranges` is `Err`).
    fn check_untyped_value(
        &self,
        body: &mut MutableBody,
        source: &mut SourceInstruction,
        ptr: Local,
        untyped: Local,
        ty: Ty,
        ranges: Result<Vec<ValidValueReq>, String>,
    ) {
        let typed = body.insert_assignment(
            Rvalue::UnaryOp(UnOp::Not, Operand::Copy(Place::from(untyped))),
            source,
            InsertPosition::Before,
        );
        match ranges {
            Ok(ranges) => {
                for range in ranges {
                    let valid = build_limits(
                        body,
                        &range,
                        Rvalue::Use(Operand::Copy(Place::from(ptr)), WithRetag::No),
                        source,
                    );
                    let result = body.insert_binary_op(
                        BinOp::BitOr,
                        Operand::Copy(Place::from(typed)),
                        Operand::Copy(Place::from(valid)),
                        source,
                        InsertPosition::Before,
                    );
                    let msg = format!("Undefined Behavior: Invalid value of type `{ty}`");
                    body.insert_check(
                        &self.safety_check_type,
                        source,
                        InsertPosition::Before,
                        Some(result),
                        &msg,
                    );
                }
            }
            Err(_) => {
                let reason = format!(
                    "Kani currently doesn't support checking validity of `read of bytes \
                                 written without a type check` for `{ty}`"
                );
                self.unsupported_check_if(body, source, untyped, &reason);
            }
        }
    }

    /// Insert the unsupported check `reason`, reached only if `cond` holds: split the block at
    /// `source` on `cond`, into a block with the check, which then continues where `source` is.
    fn unsupported_check_if(
        &self,
        body: &mut MutableBody,
        source: &mut SourceInstruction,
        cond: Local,
        reason: &str,
    ) {
        let CheckType::UnsupportedCheck(check_fn) = &self.unsupported_check_type else {
            unreachable!("the unsupported check type is an unsupported check")
        };
        let span = source.span(body.blocks());
        let msg = body.new_str_operand(reason, span);
        let func = Operand::Copy(Place::from(body.new_local(check_fn.ty(), span, Mutability::Not)));
        let unit = body.new_local(Ty::new_tuple(&[]), span, Mutability::Not);
        // The switch's targets are filled in once the blocks exist.
        let switch = Terminator {
            kind: TerminatorKind::Goto { target: 0 },
            source_info: synthetic_source_info(span),
        };
        let (switch_bb, rest_bb) = body.split_with_terminator(source, switch);
        let check_bb = body.push_raw_bb(BasicBlock {
            statements: vec![],
            terminator: Terminator {
                kind: TerminatorKind::Call {
                    func,
                    args: vec![msg],
                    destination: Place::from(unit),
                    target: Some(rest_bb),
                    unwind: UnwindAction::Terminate,
                },
                source_info: synthetic_source_info(span),
            },
        });
        body.replace_terminator(
            &SourceInstruction::Terminator { bb: switch_bb },
            Terminator {
                kind: TerminatorKind::SwitchInt {
                    discr: Operand::Copy(Place::from(cond)),
                    targets: SwitchTargets::new(vec![(0, rest_bb)], check_bb),
                },
                source_info: synthetic_source_info(span),
            },
        );
    }

    /// Insert a call to the model `model::<ty>(args)` and return the local with its result.
    fn call_model(
        &self,
        body: &mut MutableBody,
        source: &mut SourceInstruction,
        model: KaniModel,
        ty: Ty,
        args: Vec<Operand>,
        ret_ty: Ty,
    ) -> Local {
        let span = source.span(body.blocks());
        let def = self.kani_functions[&model.into()];
        let instance =
            Instance::resolve(def, &GenericArgs(vec![GenericArgKind::Type(ty)])).unwrap();
        let ret = body.new_local(ret_ty, span, Mutability::Not);
        body.insert_call(&instance, source, InsertPosition::Before, args, Place::from(ret));
        ret
    }

    /// Choose the byte whose untyped writes the checks track, where the harness starts.
    fn initialize_untyped_write_state(&self, body: &mut MutableBody) {
        let mut source = if body.blocks()[0].statements.is_empty() {
            SourceInstruction::Terminator { bb: 0 }
        } else {
            SourceInstruction::Statement { idx: 0, bb: 0 }
        };
        let span = source.span(body.blocks());
        let def = self.kani_functions[&KaniModel::InitializeUntypedWriteState.into()];
        let instance = Instance::resolve(def, &GenericArgs(vec![])).unwrap();
        let ret = body.new_local(Ty::new_tuple(&[]), span, Mutability::Not);
        body.insert_call(&instance, &mut source, InsertPosition::Before, vec![], Place::from(ret));
    }

    fn unsupported_check(
        &self,
        body: &mut MutableBody,
        source: &mut SourceInstruction,
        reason: &str,
    ) {
        body.insert_check(
            &self.unsupported_check_type,
            source,
            InsertPosition::Before,
            None,
            reason,
        );
    }
}

fn move_local(local: Local) -> Operand {
    Operand::Move(Place::from(local))
}

fn uint_ty(bytes: usize) -> UintTy {
    match bytes {
        1 => UintTy::U8,
        2 => UintTy::U16,
        4 => UintTy::U32,
        8 => UintTy::U64,
        16 => UintTy::U128,
        _ => unreachable!("Unexpected size: {bytes}"),
    }
}

/// Represent a requirement for the value stored in the given offset.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct ValidValueReq {
    /// Offset in bytes.
    offset: usize,
    /// Size of this requirement.
    size: MachineSize,
    /// The range restriction is represented by a Scalar.
    valid_range: ValidityRange,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
enum ValidityRange {
    /// The value validity fits in a single value range.
    /// This includes cases where the full range is covered.
    Single(WrappingRange),
    /// The validity includes more than one value range.
    /// Currently, this is only the case for `char`, which has two ranges.
    /// If more cases come up, we could turn this into a vector instead.
    Multiple([WrappingRange; 2]),
}

// TODO: Optimize checks by merging requirements whenever possible.
// There are a few cases that would need to be cover:
// 1- Ranges intersection is the same as one of the ranges (or both).
// 2- Ranges intersection is a new valid range.
// 3- Ranges intersection is a combination of two new ranges.
// 4- Intersection is empty.
impl ValidValueReq {
    /// Only a type with `ValueRepr::Scalar` and `ValueRepr::ScalarPair` can be directly assigned an
    /// invalid value directly.
    ///
    /// It's not possible to define a `rustc_layout_scalar_valid_range_*` to any other structure.
    /// Note that this annotation only applies to the first scalar in the layout.
    pub fn try_from_ty(tcx: TyCtxt, machine_info: &MachineInfo, ty: Ty) -> Option<ValidValueReq> {
        // The `char` special case below is skipped for pattern types (whose
        // stable `Ty::kind()` may panic); `ty_validity_per_offset` rejects
        // pattern types over `char`, so it cannot be needed here.
        if !is_pattern_type(tcx, ty) && ty.kind().is_char() {
            Some(ValidValueReq {
                offset: 0,
                size: MachineSize::from_bits(size_of::<char>() * 8),
                valid_range: ValidityRange::Multiple([
                    WrappingRange { start: 0, end: 0xD7FF },
                    WrappingRange { start: 0xE000, end: char::MAX.into() },
                ]),
            })
        } else {
            let shape = ty.layout().unwrap().shape();
            match shape.value_repr {
                ValueRepr::Scalar(Scalar::Initialized { value, valid_range })
                | ValueRepr::ScalarPair { a: Scalar::Initialized { value, valid_range }, .. } => {
                    Some(ValidValueReq {
                        offset: 0,
                        size: value.size(machine_info),
                        valid_range: ValidityRange::Single(valid_range),
                    })
                }
                ValueRepr::Scalar(_)
                | ValueRepr::ScalarPair { .. }
                | ValueRepr::Vector { .. }
                | ValueRepr::ScalableVector { .. }
                | ValueRepr::Aggregate { .. } => None,
            }
        }
    }

    /// Check if range is full.
    pub fn is_full(&self) -> bool {
        if let ValidityRange::Single(valid_range) = self.valid_range {
            valid_range.is_full(self.size).unwrap()
        } else {
            false
        }
    }

    /// Check if this range contains `other` range.
    ///
    /// I.e., `scalar_2` ⊆ `scalar_1`
    pub fn contains(&self, other: &ValidValueReq) -> bool {
        assert_eq!(self.size, other.size);
        match (&self.valid_range, &other.valid_range) {
            (ValidityRange::Single(this_range), ValidityRange::Single(other_range)) => {
                range_contains(this_range, other_range, self.size)
            }
            (ValidityRange::Multiple(this_ranges), ValidityRange::Single(other_range)) => {
                range_contains(&this_ranges[0], other_range, self.size)
                    || range_contains(&this_ranges[1], other_range, self.size)
            }
            (ValidityRange::Single(this_range), ValidityRange::Multiple(other_ranges)) => {
                range_contains(this_range, &other_ranges[0], self.size)
                    && range_contains(this_range, &other_ranges[1], self.size)
            }
            (ValidityRange::Multiple(this_ranges), ValidityRange::Multiple(other_ranges)) => {
                let contains = (range_contains(&this_ranges[0], &other_ranges[0], self.size)
                    || range_contains(&this_ranges[1], &other_ranges[0], self.size))
                    && (range_contains(&this_ranges[0], &other_ranges[1], self.size)
                        || range_contains(&this_ranges[1], &other_ranges[1], self.size));
                // Multiple today only cover `char` case.
                debug_assert!(
                    contains,
                    "Expected validity of `char` for Multiple ranges. Found: {self:?}, {other:?}"
                );
                contains
            }
        }
    }
}

/// Check if range `r1` contains range `r2`.
///
/// I.e., `r2` ⊆ `r1`
fn range_contains(r1: &WrappingRange, r2: &WrappingRange, sz: MachineSize) -> bool {
    match (r1.wraps_around(), r2.wraps_around()) {
        (true, true) | (false, false) => r1.start <= r2.start && r1.end >= r2.end,
        (true, false) => r1.start <= r2.start || r1.end >= r2.end,
        (false, true) => r1.is_full(sz).unwrap(),
    }
}

#[derive(AsRefStr, Clone, Debug)]
enum SourceOp {
    /// Validity checks are done on a byte level when the Rvalue can generate invalid value.
    ///
    /// This variant tracks a location that is valid for its current type, but it may not be
    /// valid for the given location in target type. This happens for:
    ///  - Transmute
    ///  - Field assignment
    ///  - Aggregate assignment
    ///  - Union Access
    ///
    /// Each range is a pair of offset and scalar that represents the valid values.
    /// Note that the same offset may have multiple ranges that may require being joined.
    BytesValidity { target_ty: Ty, rvalue: Rvalue, ranges: Vec<ValidValueReq> },

    /// Similar to BytesValidity, but it stores any dereference that may be unsafe.
    ///
    /// This can happen for:
    ///  - Raw pointer dereference
    DerefValidity { pointee_ty: Ty, rvalue: Rvalue, ranges: Vec<ValidValueReq> },

    /// Represents a range check Kani currently does not support.
    ///
    /// This will translate into an assertion failure with an unsupported message.
    /// There are many corner cases with the usage of #[rustc_layout_scalar_valid_range_*]
    /// attribute. Such as valid ranges that do not intersect or enumeration with variants
    /// with niche.
    ///
    /// Supporting all cases require significant work, and it is unlikely to exist in real world
    /// code. To be on the sound side, we just emit an unsupported check, and users will need to
    /// disable the check in person, and create a feature request for their case.
    ///
    /// TODO: Consider replacing the assertion(false) by an unsupported operation that emits a
    /// compilation warning.
    UnsupportedCheck { check: String, ty: Ty },

    /// Record that the `count` (one if `None`) values of `elem_size` bytes at the pointer `ptr`
    /// evaluates to were
    /// written without a type check (`untyped`), or by a typed store of a type with validity
    /// requirements (not `untyped`).
    ///
    /// Writing bytes that are invalid for a type is not undefined behavior: reading them as that
    /// type is. So untyped writes are only recorded, and the reads that cover them are checked.
    SetUntyped { ptr: Rvalue, count: Option<Operand>, elem_size: u64, untyped: bool },

    /// A typed read of `place`, of type `ty`, from memory that may have been written untyped:
    /// check its value if one of its bytes was. `ranges` is `Err` if Kani can't express the
    /// validity of `ty`, which is then reported as unsupported, but only if the read covers bytes
    /// written untyped.
    UntypedRead { place: Place, ty: Ty, ranges: Result<Vec<ValidValueReq>, String> },
    /// The call this instruction is, an unaligned load of a `ty` at the pointer `ptr` into
    /// `dest`: check the value it loads, after the call, if one of the bytes at `ptr` was written
    /// untyped. The bytes at `ptr` can't be read as `ty` in place, as they may be misaligned.
    UntypedLoad { ptr: Operand, dest: Place, ty: Ty, ranges: Result<Vec<ValidValueReq>, String> },
}

/// The unsafe instructions that may generate invalid values.
/// We need to instrument all operations to ensure the instruction is safe.
#[derive(Clone, Debug)]
struct UnsafeInstruction {
    /// The instruction that depends on the potentially invalid value.
    source: SourceInstruction,
    /// The unsafe operations that may cause an invalid value in this instruction.
    operations: Vec<SourceOp>,
}

/// How a place is used.
#[derive(Clone, Copy, PartialEq, Eq)]
enum PlaceAccessKind {
    /// Keep the existing checks for operations not yet classified here.
    Conservative,
    Drop,
    MutableBorrow,
    RawAddress,
    Read,
    SharedBorrow,
    Store,
}

/// Extract any source that may potentially trigger UB due to the generation of an invalid value.
///
/// Generating an invalid value requires an unsafe operation, however, in MIR, it
/// may just be represented as a regular assignment.
///
/// Thus, we have to instrument every assignment to an object that has niche and that the source
/// is an object of a different source, e.g.:
///   - Aggregate assignment
///   - Transmute
///   - MemCopy
///   - Cast
struct CheckValueVisitor<'a, 'b> {
    access: PlaceAccessKind,
    tcx: TyCtxt<'b>,
    locals: &'a [LocalDecl],
    /// Whether we should skip the next instruction, since it might've been instrumented already.
    /// When we instrument an instruction, we partition the basic block, and the instruction that
    /// may trigger UB becomes the first instruction of the basic block, which we need to skip
    /// later.
    skip_next: bool,
    /// The instruction being visited at a given point.
    current: SourceInstruction,
    /// The target instruction that should be verified.
    pub target: Option<UnsafeInstruction>,
    /// The basic block being visited.
    bb: BasicBlockIdx,
    /// Machine information needed to calculate Niche.
    machine: MachineInfo,
    /// Locals whose address the function takes: a pointer may write them untyped.
    address_taken: &'a HashSet<Local>,
}

impl<'a, 'b> CheckValueVisitor<'a, 'b> {
    fn find_next(
        tcx: TyCtxt<'b>,
        body: &'a MutableBody,
        bb: BasicBlockIdx,
        skip_first: bool,
        address_taken: &'a HashSet<Local>,
    ) -> Option<UnsafeInstruction> {
        let mut visitor = CheckValueVisitor {
            access: PlaceAccessKind::Conservative,
            tcx,
            locals: body.locals(),
            skip_next: skip_first,
            current: SourceInstruction::Statement { idx: 0, bb },
            target: None,
            bb,
            machine: MachineInfo::target(),
            address_taken,
        };
        visitor.visit_basic_block(&body.blocks()[bb]);
        visitor.target
    }

    fn push_target(&mut self, op: SourceOp) {
        let target = self
            .target
            .get_or_insert_with(|| UnsafeInstruction { source: self.current, operations: vec![] });
        target.operations.push(op);
    }

    fn with_access(&mut self, access: PlaceAccessKind, visit: impl FnOnce(&mut Self)) {
        let previous = self.access;
        self.access = access;
        visit(self);
        self.access = previous;
    }

    /// Whether the memory of `place` may have been written untyped: it is reached through a
    /// pointer, or it is a local whose address is taken.
    fn may_be_untyped(&self, place: &Place) -> bool {
        place.projection.iter().any(|elem| matches!(elem, ProjectionElem::Deref))
            || self.address_taken.contains(&place.local)
    }

    /// Whether `place` is a value a closure captured, read through its environment (`env.field`
    /// or `(*env).field`). These reads are not checked: a closure is only called through a borrow
    /// or a move of the closure value, and that read of the closure is checked where the closure
    /// lives if its bytes may have been written untyped. Reads through a captured reference, into
    /// other memory, are still checked. Quantifier predicates, which can't contain checks, read
    /// their captures this way.
    fn is_closure_capture(&self, place: &Place) -> bool {
        let is_closure = |ty: Ty| {
            matches!(
                ty.kind(),
                TyKind::RigidTy(RigidTy::Closure(..) | RigidTy::CoroutineClosure(..))
            )
        };
        let env_ty = self.locals[place.local].ty;
        match place.projection.as_slice() {
            [ProjectionElem::Field(..)] => is_closure(env_ty),
            [ProjectionElem::Deref, ProjectionElem::Field(..)] => match env_ty.kind() {
                TyKind::RigidTy(RigidTy::Ref(_, pointee, _)) => is_closure(pointee),
                _ => false,
            },
            _ => false,
        }
    }

    /// Whether `place` dereferences a raw pointer.
    fn derefs_raw_pointer(&self, place: &Place) -> bool {
        place.projection.iter().enumerate().any(|(idx, elem)| {
            matches!(elem, ProjectionElem::Deref)
                && PlaceRef { local: place.local, projection: &place.projection[..idx] }
                    .ty(self.locals)
                    .is_ok_and(|ty| ty.kind().is_raw_ptr())
        })
    }

    /// A typed read of `place`: check it if it reads bytes written untyped.
    fn check_untyped_read(&mut self, place: &Place) {
        if !self.may_be_untyped(place) || self.is_closure_capture(place) {
            return;
        }
        let Ok(ty) = place.ty(self.locals) else { return };
        if !ty.layout().is_ok_and(|layout| layout.shape().is_sized()) {
            return;
        }
        match ty_validity_per_offset(self.tcx, &self.machine, ty, 0) {
            Ok(ranges) if ranges.is_empty() => {}
            ranges => self.push_target(SourceOp::UntypedRead { place: place.clone(), ty, ranges }),
        }
    }

    /// A store to `place`. A store of a restricted type directly into a local writes a valid value
    /// of the local's type. A store through a pointer is recorded as untyped: if the pointee type
    /// has no validity requirements, its bytes may be invalid for the memory's type; if it does,
    /// the pointer may alias memory of another restricted type (a `*mut NonZeroU8` cast from a
    /// `*mut bool`), and the reads that cover the bytes check them. A restricted type whose
    /// validity can't be expressed clears the mark instead, so that its later reads don't report
    /// an unsupported check.
    fn record_store(&mut self, place: &Place) {
        if !self.may_be_untyped(place) {
            return;
        }
        let Ok(ty) = place.ty(self.locals) else { return };
        let Ok(layout) = ty.layout() else { return };
        let shape = layout.shape();
        if !shape.is_sized() || shape.size.bytes() == 0 {
            return;
        }
        let through_pointer =
            place.projection.iter().any(|elem| matches!(elem, ProjectionElem::Deref));
        let validity = ty_validity_per_offset(self.tcx, &self.machine, ty, 0);
        let unrestricted = matches!(&validity, Ok(ranges) if ranges.is_empty());
        if unrestricted && !through_pointer {
            // A typed store of an unrestricted type into a local: nothing reads it as a
            // restricted type without a cast, and reads through casts are checked.
            return;
        }
        let untyped = through_pointer && validity.is_ok();
        self.push_target(SourceOp::SetUntyped {
            ptr: Rvalue::AddressOf(RawPtrKind::Const, place.clone()),
            count: None,
            elem_size: shape.size.bytes() as u64,
            untyped,
        });
    }

    /// A write of `count` values at the pointer `ptr` (of type `*mut T`) that doesn't check types.
    fn record_untyped_write(&mut self, ptr: &Operand, count: Option<Operand>) {
        let ptr_ty = ptr.ty(self.locals).unwrap();
        let elem_ty = pointee_ty(ptr_ty);
        let Ok(layout) = elem_ty.layout() else { return };
        self.push_target(SourceOp::SetUntyped {
            ptr: Rvalue::Use(ptr.clone(), WithRetag::No),
            count,
            elem_size: layout.shape().size.bytes() as u64,
            untyped: true,
        });
    }

    /// Narrow checks only for an optional leading dereference and ordinary struct/tuple fields.
    /// Other projections retain the existing conservative checks.
    fn supports_precise_access(&self, place: &Place) -> bool {
        let mut ty = self.locals[place.local].ty;
        for (idx, elem) in place.projection.iter().enumerate() {
            match elem {
                ProjectionElem::Deref if idx == 0 => {}
                ProjectionElem::Field(_, field_ty) => {
                    // Avoid stable kind conversion for pattern parents.
                    if is_pattern_type(self.tcx, ty) {
                        return false;
                    }
                    let ordinary_field = match ty.kind() {
                        TyKind::RigidTy(RigidTy::Adt(def, _)) => def.kind() == AdtKind::Struct,
                        TyKind::RigidTy(RigidTy::Tuple(_)) => true,
                        _ => false,
                    };
                    if !ordinary_field {
                        return false;
                    }
                    // Pattern fields are not unions; avoid converting their stable kind.
                    if !is_pattern_type(self.tcx, *field_ty) && field_ty.kind().is_union() {
                        return false;
                    }
                }
                ProjectionElem::Deref
                | ProjectionElem::Downcast(_)
                | ProjectionElem::OpaqueCast(_)
                | ProjectionElem::Index(_)
                | ProjectionElem::ConstantIndex { .. }
                | ProjectionElem::Subslice { .. } => return false,
            }
            ty = elem.ty(ty).unwrap();
        }
        true
    }

    /// Form an address, rather than loading an aggregate to extract the field being checked.
    fn check_place_value(&mut self, place: Place) {
        let ty = place.ty(self.locals).unwrap();
        match ty_validity_per_offset(self.tcx, &self.machine, ty, 0) {
            Ok(ranges) if !ranges.is_empty() => {
                self.push_target(SourceOp::DerefValidity {
                    pointee_ty: ty,
                    rvalue: Rvalue::AddressOf(RawPtrKind::Const, place),
                    ranges,
                });
            }
            Err(_) => self.push_target(SourceOp::UnsupportedCheck {
                check: "raw pointer dereference".to_string(),
                ty,
            }),
            _ => {}
        }
    }

    /// Called only for places accepted by `supports_precise_access`.
    ///
    /// For `**ptr = value` or `&raw const **ptr`, rustc emits a separate load of the intermediate
    /// pointer before the final store/address operation. `visit_operand` (or `CopyForDeref`)
    /// classifies that load as Read, independently of the final operation's access kind.
    fn check_place_access(&mut self, place: &Place) {
        let through_raw_pointer = matches!(place.projection.first(), Some(ProjectionElem::Deref))
            && self.locals[place.local].ty.kind().is_raw_ptr();
        if through_raw_pointer {
            match self.access {
                PlaceAccessKind::Drop
                | PlaceAccessKind::MutableBorrow
                | PlaceAccessKind::Read
                | PlaceAccessKind::SharedBorrow => self.check_place_value(place.clone()),
                PlaceAccessKind::RawAddress | PlaceAccessKind::Store => {}
                PlaceAccessKind::Conservative => unreachable!(),
            }
        }
    }
}

impl MirVisitor for CheckValueVisitor<'_, '_> {
    fn visit_statement(&mut self, stmt: &Statement, location: Location) {
        if self.skip_next {
            self.skip_next = false;
        } else if self.target.is_none() {
            // Leave it as an exhaustive match to be notified when a new kind is added.
            match &stmt.kind {
                StatementKind::Intrinsic(NonDivergingIntrinsic::CopyNonOverlapping(copy)) => {
                    // A copy writes bytes, not values: the destination's bytes are written
                    // untyped, and the reads that cover them are checked.
                    self.record_untyped_write(&copy.dst, Some(copy.count.clone()));
                }
                StatementKind::Assign(place, rvalue) => {
                    // First check rvalue.
                    // The destination is a store; rvalues and operands override this context.
                    self.with_access(PlaceAccessKind::Store, |visitor| {
                        visitor.super_statement(stmt, location);
                    });
                    // Then check the destination place.
                    let ranges = assignment_check_points(
                        self.tcx,
                        &self.machine,
                        self.locals,
                        place,
                        rvalue.ty(self.locals).unwrap(),
                    );
                    if !ranges.is_empty() {
                        self.push_target(SourceOp::BytesValidity {
                            target_ty: self.locals[place.local].ty,
                            rvalue: rvalue.clone(),
                            ranges,
                        });
                    }
                    self.record_store(place);
                }
                StatementKind::FakeRead(_, _)
                | StatementKind::SetDiscriminant { .. }
                | StatementKind::StorageLive(_)
                | StatementKind::StorageDead(_)
                | StatementKind::PlaceMention(_)
                | StatementKind::AscribeUserType { .. }
                | StatementKind::Coverage(_)
                | StatementKind::ConstEvalCounter
                | StatementKind::Intrinsic(NonDivergingIntrinsic::Assume(_))
                | StatementKind::Nop => self.super_statement(stmt, location),
            }
        }

        let SourceInstruction::Statement { idx, bb } = self.current else { unreachable!() };
        self.current = SourceInstruction::Statement { idx: idx + 1, bb };
    }
    fn visit_terminator(&mut self, term: &Terminator, location: Location) {
        if !(self.skip_next || self.target.is_some()) {
            self.current = SourceInstruction::Terminator { bb: self.bb };
            // Leave it as an exhaustive match to be notified when a new kind is added.
            match &term.kind {
                TerminatorKind::Call { func, args, destination, .. } => {
                    // Note: For transmute, both Src and Dst must be valid type.
                    // In this case, we need to save the Dst, and invoke super_terminator.
                    // The return destination is a store; argument operands are reads.
                    self.with_access(PlaceAccessKind::Store, |visitor| {
                        visitor.super_terminator(term, location);
                    });
                    match intrinsic_name(self.locals, func).as_deref() {
                        // Intrinsics that write bytes, not values: record the untyped write.
                        // <https://doc.rust-lang.org/stable/core/intrinsics/fn.write_bytes.html>
                        Some("write_bytes") | Some("volatile_set_memory") => {
                            self.record_untyped_write(&args[0], Some(args[2].clone()));
                        }
                        Some("copy") => self.record_untyped_write(&args[1], Some(args[2].clone())),
                        Some("volatile_copy_memory")
                        | Some("volatile_copy_nonoverlapping_memory") => {
                            self.record_untyped_write(&args[0], Some(args[2].clone()));
                        }
                        Some("typed_swap_nonoverlapping") => {
                            self.record_untyped_write(&args[0], None);
                            self.record_untyped_write(&args[1], None);
                        }
                        // Volatile accesses read and write through the pointer without a MIR
                        // place: treat them as a typed read and a store through the pointer.
                        Some("volatile_load") => {
                            if let Some(place) = deref_place(&args[0]) {
                                self.check_untyped_read(&place);
                            }
                        }
                        Some("unaligned_volatile_load") => {
                            let ty = destination.ty(self.locals).unwrap();
                            match ty_validity_per_offset(self.tcx, &self.machine, ty, 0) {
                                Ok(ranges) if ranges.is_empty() => {}
                                ranges => self.push_target(SourceOp::UntypedLoad {
                                    ptr: args[0].clone(),
                                    dest: destination.clone(),
                                    ty,
                                    ranges,
                                }),
                            }
                        }
                        Some("volatile_store") => {
                            if let Some(place) = deref_place(&args[0]) {
                                self.record_store(&place);
                            }
                        }
                        // The pointer may be misaligned, so it's not a place: record the bytes
                        // it writes as untyped, as for any store through a pointer.
                        Some("unaligned_volatile_store") => {
                            self.record_untyped_write(&args[0], None);
                        }
                        // Atomic operations other than loads and fences write an integer or a
                        // pointer, types without validity requirements, through the pointer.
                        Some(name)
                            if name.starts_with("atomic_")
                                && !matches!(
                                    name,
                                    "atomic_load" | "atomic_fence" | "atomic_singlethreadfence"
                                ) =>
                        {
                            self.record_untyped_write(&args[0], None);
                        }
                        Some("transmute") | Some("transmute_copy") => {
                            unreachable!("Should've been lowered")
                        }
                        _ => {}
                    }
                }
                TerminatorKind::Drop { .. } => {
                    self.with_access(PlaceAccessKind::Drop, |visitor| {
                        visitor.super_terminator(term, location);
                    });
                }
                TerminatorKind::Goto { .. }
                | TerminatorKind::SwitchInt { .. }
                | TerminatorKind::Resume
                | TerminatorKind::Abort
                | TerminatorKind::Return
                | TerminatorKind::Unreachable
                | TerminatorKind::Assert { .. }
                | TerminatorKind::InlineAsm { .. } => self.super_terminator(term, location),
            }
        }
    }

    fn visit_place(&mut self, place: &Place, ptx: PlaceContext, location: Location) {
        if self.access != PlaceAccessKind::Conservative && self.supports_precise_access(place) {
            self.check_place_access(place);
            self.super_place(place, ptx, location);
            return;
        }
        for (idx, elem) in place.projection.iter().enumerate() {
            let place_ref = PlaceRef { local: place.local, projection: &place.projection[..idx] };
            match elem {
                ProjectionElem::Deref => {
                    let ptr_ty = place_ref.ty(self.locals).unwrap();
                    if ptr_ty.kind().is_raw_ptr() {
                        let target_ty = elem.ty(ptr_ty).unwrap();
                        let validity =
                            ty_validity_per_offset(self.tcx, &self.machine, target_ty, 0);
                        match validity {
                            Ok(ranges) if !ranges.is_empty() => {
                                self.push_target(SourceOp::DerefValidity {
                                    pointee_ty: target_ty,
                                    rvalue: Rvalue::Use(
                                        Operand::Copy(Place {
                                            local: place_ref.local,
                                            projection: place_ref.projection.to_vec(),
                                        })
                                        .clone(),
                                        WithRetag::No,
                                    ),
                                    ranges,
                                })
                            }
                            Err(_msg) => self.push_target(SourceOp::UnsupportedCheck {
                                check: "raw pointer dereference".to_string(),
                                ty: target_ty,
                            }),
                            _ => {}
                        }
                    }
                }
                ProjectionElem::Field(idx, target_ty) => {
                    // Pattern fields are not unions; avoid converting their stable kind.
                    if !is_pattern_type(self.tcx, *target_ty)
                        && target_ty.kind().is_union()
                        && (!ptx.is_mutating() || place.projection.len() > idx + 1)
                    {
                        let validity =
                            ty_validity_per_offset(self.tcx, &self.machine, *target_ty, 0);
                        match validity {
                            Ok(ranges) if !ranges.is_empty() => {
                                self.push_target(SourceOp::BytesValidity {
                                    target_ty: *target_ty,
                                    rvalue: Rvalue::Use(
                                        Operand::Copy(Place {
                                            local: place_ref.local,
                                            projection: place_ref.projection.to_vec(),
                                        }),
                                        WithRetag::No,
                                    ),
                                    ranges,
                                })
                            }
                            Err(_msg) => self.push_target(SourceOp::UnsupportedCheck {
                                check: "union access".to_string(),
                                ty: *target_ty,
                            }),
                            _ => {}
                        }
                    }
                }
                ProjectionElem::Downcast(_) => {}
                ProjectionElem::OpaqueCast(_) => {}
                ProjectionElem::Index(_)
                | ProjectionElem::ConstantIndex { .. }
                | ProjectionElem::Subslice { .. } => { /* safe */ }
            }
        }
        self.super_place(place, ptx, location)
    }

    fn visit_operand(&mut self, operand: &Operand, location: Location) {
        if let Operand::Copy(place) | Operand::Move(place) = operand {
            self.check_untyped_read(place);
        }
        self.with_access(PlaceAccessKind::Read, |visitor| {
            visitor.super_operand(operand, location);
        });
    }

    fn visit_rvalue(&mut self, rvalue: &Rvalue, location: Location) {
        match rvalue {
            Rvalue::Cast(kind, op, dest_ty) => match kind {
                CastKind::PtrToPtr => {
                    // For mutable raw pointer, if the type we are casting to is less restrictive
                    // than the original type, writing to the pointer could generate UB if the
                    // value is ever read again using the original pointer.
                    let TyKind::RigidTy(RigidTy::RawPtr(dest_pointee_ty, Mutability::Mut)) =
                        dest_ty.kind()
                    else {
                        // We only care about *mut T as *mut U
                        return;
                    };
                    if dest_pointee_ty.kind().is_unit() {
                        // Ignore cast to *mut () since nothing can be written to it.
                        // This is a common pattern
                        return;
                    }

                    let src_ty = op.ty(self.locals).unwrap();
                    debug!(?src_ty, ?dest_ty, "visit_rvalue mutcast");
                    let TyKind::RigidTy(RigidTy::RawPtr(src_pointee_ty, _)) = src_ty.kind() else {
                        unreachable!()
                    };

                    // Writes through a pointer to a type without validity requirements are
                    // recorded as untyped (see `record_store`), and the reads that cover them
                    // are checked.
                    if matches!(
                        ty_validity_per_offset(self.tcx, &self.machine, dest_pointee_ty, 0),
                        Ok(ranges) if ranges.is_empty()
                    ) {
                        return;
                    }

                    if src_pointee_ty.kind().is_unit() {
                        // We cannot track what was the initial type. Thus, fail.
                        self.push_target(SourceOp::UnsupportedCheck {
                            check: "mutable cast".to_string(),
                            ty: src_ty,
                        });
                        return;
                    }

                    if let Ok(src_validity) =
                        ty_validity_per_offset(self.tcx, &self.machine, src_pointee_ty, 0)
                    {
                        if !src_validity.is_empty() {
                            if let Ok(dest_validity) =
                                ty_validity_per_offset(self.tcx, &self.machine, dest_pointee_ty, 0)
                            {
                                // A pointer to the element of an array (e.g. `*mut [T; N]` as
                                // `*mut T`) reaches, by pointer arithmetic, only slots that hold
                                // a `T`.
                                if dest_validity != src_validity
                                    && !is_array_element(self.tcx, src_pointee_ty, dest_pointee_ty)
                                {
                                    self.push_target(SourceOp::UnsupportedCheck {
                                        check: "mutable cast".to_string(),
                                        ty: src_ty,
                                    })
                                }
                            } else {
                                self.push_target(SourceOp::UnsupportedCheck {
                                    check: "mutable cast".to_string(),
                                    ty: *dest_ty,
                                })
                            }
                        }
                    } else {
                        self.push_target(SourceOp::UnsupportedCheck {
                            check: "mutable cast".to_string(),
                            ty: src_ty,
                        })
                    }
                }
                CastKind::Transmute | CastKind::BoxDerefTransmute | CastKind::Subtype => {
                    debug!(?dest_ty, "transmute");
                    // For transmute, we care about the destination type only.
                    // This could be optimized to only add a check if the requirements of the
                    // destination type are stricter than the source.
                    if let Ok(dest_validity) =
                        ty_validity_per_offset(self.tcx, &self.machine, *dest_ty, 0)
                    {
                        trace!(?dest_validity, "transmute");
                        if !dest_validity.is_empty() {
                            self.push_target(SourceOp::BytesValidity {
                                target_ty: *dest_ty,
                                rvalue: rvalue.clone(),
                                ranges: dest_validity,
                            })
                        }
                    } else {
                        self.push_target(SourceOp::UnsupportedCheck {
                            check: "transmute".to_string(),
                            ty: *dest_ty,
                        })
                    }
                }
                CastKind::PointerExposeAddress
                | CastKind::PointerWithExposedProvenance
                | CastKind::PointerCoercion(_)
                | CastKind::IntToInt
                | CastKind::FloatToInt
                | CastKind::FloatToFloat
                | CastKind::IntToFloat
                | CastKind::FnPtrToPtr => {}
            },
            Rvalue::Aggregate(kind, operands) => match kind {
                // If the aggregated structure has invalid value, this could generate invalid value.
                // But only if the operands don't have the exact same restrictions.
                // This happens today with the usage of `rustc_layout_scalar_valid_range_*`
                // attributes.
                // In this case, only the value of the first member in memory can be restricted,
                // thus, we only need to check the operand used to assign to the first in memory
                // field.
                AggregateKind::Adt(def, _variant, args, _, _) => {
                    if def.kind() == AdtKind::Struct {
                        let dest_ty = Ty::from_rigid_kind(RigidTy::Adt(*def, args.clone()));
                        if let Some(req) =
                            ValidValueReq::try_from_ty(self.tcx, &self.machine, dest_ty)
                            && !req.is_full()
                        {
                            let dest_layout = dest_ty.layout().unwrap().shape();
                            let first_op =
                                first_aggregate_operand(dest_ty, &dest_layout.fields, operands);
                            let first_ty = first_op.ty(self.locals).unwrap();
                            // Rvalue must have same Abi layout except for range.
                            if !req.contains(
                                &ValidValueReq::try_from_ty(self.tcx, &self.machine, first_ty)
                                    .unwrap(),
                            ) {
                                self.push_target(SourceOp::BytesValidity {
                                    target_ty: dest_ty,
                                    rvalue: Rvalue::Use(first_op, WithRetag::No),
                                    ranges: vec![req],
                                })
                            }
                        }
                    }
                }
                // Only aggregate value.
                AggregateKind::Array(_)
                | AggregateKind::Closure(_, _)
                | AggregateKind::Coroutine(_, _)
                | AggregateKind::CoroutineClosure(_, _)
                | AggregateKind::RawPtr(_, _)
                | AggregateKind::Tuple => {}
            },
            // Reading a discriminant, or a place to dereference it, is a typed read.
            // So is forming a reference: Kani requires the target of a reference to be valid, as
            // it does for `&*ptr`. Dropping a value borrows it, and its drop glue reads it.
            Rvalue::CopyForDeref(place) | Rvalue::Discriminant(place) => {
                self.check_untyped_read(place);
            }
            // A borrow through a raw pointer is checked unconditionally (`DerefValidity`).
            Rvalue::Ref(_, _, place) if !self.derefs_raw_pointer(place) => {
                self.check_untyped_read(place);
            }
            Rvalue::Ref(..)
            | Rvalue::AddressOf(_, _)
            | Rvalue::BinaryOp(_, _, _)
            | Rvalue::CheckedBinaryOp(_, _, _)
            | Rvalue::Len(_)
            | Rvalue::Reborrow(..)
            | Rvalue::Repeat(_, _)
            | Rvalue::ThreadLocalRef(_)
            | Rvalue::UnaryOp(_, _)
            | Rvalue::Use(..) => {}
        }
        let access = match rvalue {
            Rvalue::AddressOf(_, _) => PlaceAccessKind::RawAddress,
            Rvalue::Ref(_, BorrowKind::Mut { .. }, _) => PlaceAccessKind::MutableBorrow,
            Rvalue::Ref(_, BorrowKind::Shared, _) => PlaceAccessKind::SharedBorrow,
            Rvalue::CopyForDeref(_) => PlaceAccessKind::Read,
            // Unclassified rvalues must not inherit Store from the assignment.
            _ => PlaceAccessKind::Conservative,
        };
        self.with_access(access, |visitor| visitor.super_rvalue(rvalue, location));
    }
}

/// Gets the operand that corresponds to the assignment of the first sized field in memory.
///
/// The first field of a structure is the only one that can have extra value restrictions imposed
/// by `rustc_layout_scalar_valid_range_*` attributes.
///
/// Note: This requires at least one operand to be sized and there's a 1:1 match between operands
/// and field types.
fn first_aggregate_operand(dest_ty: Ty, dest_shape: &FieldsShape, operands: &[Operand]) -> Operand {
    let Some(first) = first_sized_field_idx(dest_ty, dest_shape) else { unreachable!() };
    operands[first].clone()
}

/// Index of the first non_1zst fields in memory order.
fn first_sized_field_idx(ty: Ty, shape: &FieldsShape) -> Option<FieldIdx> {
    if let TyKind::RigidTy(RigidTy::Adt(adt_def, args)) = ty.kind()
        && adt_def.kind() == AdtKind::Struct
    {
        let offset_order = shape.fields_by_offset_order();
        let fields = adt_def.variants_iter().next().unwrap().fields();
        offset_order
            .into_iter()
            .find(|idx| !fields[*idx].ty_with_args(&args).layout().unwrap().shape().is_1zst())
    } else {
        None
    }
}

/// An assignment to a field with invalid values is unsafe, and it may trigger UB if
/// the assigned value is invalid.
///
/// This can only happen to the first in memory sized field of a struct, and only if the field
/// type invalid range is a valid value for the rvalue type.
fn assignment_check_points(
    tcx: TyCtxt,
    machine_info: &MachineInfo,
    locals: &[LocalDecl],
    place: &Place,
    rvalue_ty: Ty,
) -> Vec<ValidValueReq> {
    let mut ty = locals[place.local].ty;
    let Some(rvalue_range) = ValidValueReq::try_from_ty(tcx, machine_info, rvalue_ty) else {
        // Rvalue Abi must be Scalar / ScalarPair since destination must be Scalar / ScalarPair.
        return vec![];
    };
    let mut invalid_ranges = vec![];
    for proj in &place.projection {
        match proj {
            ProjectionElem::Field(field_idx, field_ty) => {
                let shape = ty.layout().unwrap().shape();
                if first_sized_field_idx(ty, &shape.fields) == Some(*field_idx)
                    && let Some(dest_valid) = ValidValueReq::try_from_ty(tcx, machine_info, ty)
                    && !dest_valid.is_full()
                    && dest_valid.size == rvalue_range.size
                {
                    if !dest_valid.contains(&rvalue_range) {
                        invalid_ranges.push(dest_valid)
                    }
                } else {
                    // Invalidate collected ranges so far since we are no longer in the path of
                    // the first element.
                    invalid_ranges.clear();
                }
                ty = *field_ty;
            }
            ProjectionElem::Deref
            | ProjectionElem::Index(_)
            | ProjectionElem::ConstantIndex { .. }
            | ProjectionElem::Subslice { .. }
            | ProjectionElem::Downcast(_)
            | ProjectionElem::OpaqueCast(_) => ty = proj.ty(ty).unwrap(),
        };
    }
    invalid_ranges
}

/// Retrieve the name of the intrinsic if this operand is an intrinsic.
///
/// Intrinsics can only be invoked directly, so we can safely ignore other operand types.
fn intrinsic_name(locals: &[LocalDecl], func: &Operand) -> Option<String> {
    let ty = func.ty(locals).unwrap();
    let TyKind::RigidTy(RigidTy::FnDef(def, args)) = ty.kind() else { return None };
    Instance::resolve(def, &args).unwrap().intrinsic_name()
}

/// Instrument MIR to check the value pointed by `rvalue_ptr` satisfies requirement `req`.
///
/// The MIR will do something equivalent to:
/// ```rust
///     let ptr = rvalue_ptr.byte_offset(req.offset);
///     let typed_ptr = ptr as *const Unsigned<req.size>; // Some unsigned type with length req.size
///     let value = unsafe { *typed_ptr };
///     req.valid_range.contains(value)
/// ```
pub fn build_limits(
    body: &mut MutableBody,
    req: &ValidValueReq,
    rvalue_ptr: Rvalue,
    source: &mut SourceInstruction,
) -> Local {
    let span = source.span(body.blocks());
    debug!(?req, ?rvalue_ptr, ?span, "build_limits");
    let primitive_ty = uint_ty(req.size.bytes());
    let orig_ptr = if req.offset != 0 {
        let start_ptr =
            move_local(body.insert_assignment(rvalue_ptr, source, InsertPosition::Before));
        let byte_ptr = move_local(body.insert_ptr_cast(
            start_ptr,
            Ty::unsigned_ty(UintTy::U8),
            Mutability::Not,
            source,
            InsertPosition::Before,
        ));
        let offset_const = body.new_uint_operand(req.offset as _, UintTy::Usize, span);
        let offset = move_local(body.insert_assignment(
            Rvalue::Use(offset_const, WithRetag::No),
            source,
            InsertPosition::Before,
        ));
        move_local(body.insert_binary_op(
            BinOp::Offset,
            byte_ptr,
            offset,
            source,
            InsertPosition::Before,
        ))
    } else {
        move_local(body.insert_assignment(rvalue_ptr, source, InsertPosition::Before))
    };
    let value_ptr = body.insert_ptr_cast(
        orig_ptr,
        Ty::unsigned_ty(primitive_ty),
        Mutability::Not,
        source,
        InsertPosition::Before,
    );
    let value = Operand::Copy(Place { local: value_ptr, projection: vec![ProjectionElem::Deref] });
    match &req.valid_range {
        ValidityRange::Single(range) => {
            build_single_limit(body, range, source, span, primitive_ty, value)
        }
        ValidityRange::Multiple([range1, range2]) => {
            // Build `let valid = range1.contains(value) || range2.contains(value);
            let cond1 = build_single_limit(body, range1, source, span, primitive_ty, value.clone());
            let cond2 = build_single_limit(body, range2, source, span, primitive_ty, value);
            body.insert_binary_op(
                BinOp::BitOr,
                move_local(cond1),
                move_local(cond2),
                source,
                InsertPosition::Before,
            )
        }
    }
}

fn build_single_limit(
    body: &mut MutableBody,
    range: &WrappingRange,
    source: &mut SourceInstruction,
    span: Span,
    primitive_ty: UintTy,
    value: Operand,
) -> Local {
    let start_const = body.new_uint_operand(range.start, primitive_ty, span);
    let end_const = body.new_uint_operand(range.end, primitive_ty, span);
    let start_result = body.insert_binary_op(
        BinOp::Ge,
        value.clone(),
        start_const,
        source,
        InsertPosition::Before,
    );
    let end_result =
        body.insert_binary_op(BinOp::Le, value, end_const, source, InsertPosition::Before);
    if range.wraps_around() {
        // valid >= start || valid <= end
        body.insert_binary_op(
            BinOp::BitOr,
            move_local(start_result),
            move_local(end_result),
            source,
            InsertPosition::Before,
        )
    } else {
        // valid >= start && valid <= end
        body.insert_binary_op(
            BinOp::BitAnd,
            move_local(start_result),
            move_local(end_result),
            source,
            InsertPosition::Before,
        )
    }
}

/// Traverse the type and find all invalid values and their location in memory.
///
/// A pattern type (e.g. `NonNull`) may use a `PatternKind` (such as `NotNull`)
/// that `rustc_public` cannot convert to its stable representation yet, so
/// calling the stable `Ty::kind()` on it panics. Detect such types via the
/// internal representation instead, where inspecting the kind is always safe.
fn is_pattern_type(tcx: TyCtxt, ty: Ty) -> bool {
    matches!(rustc_internal::internal(tcx, ty).kind(), rustc_middle::ty::TyKind::Pat(..))
}

/// Whether all validity requirements of the fields of `variant` (the untagged variant of a
/// scalar niche-encoded enum) are single ranges, which the enum's own scalar `valid_range` then
/// subsumes. Returns `false` if any requirement is not a single range (e.g. for `char`) or cannot
/// be computed.
fn niche_payload_is_single_range(
    tcx: TyCtxt,
    machine_info: &MachineInfo,
    variant: &VariantDef,
    args: &GenericArgs,
) -> bool {
    variant.fields().iter().all(|field| {
        ty_validity_per_offset(tcx, machine_info, field.ty_with_args(args), 0).is_ok_and(|reqs| {
            reqs.iter().all(|req| matches!(req.valid_range, ValidityRange::Single(_)))
        })
    })
}

/// Not all values are currently supported. For those not supported, we return Error.
pub fn ty_validity_per_offset(
    tcx: TyCtxt,
    machine_info: &MachineInfo,
    ty: Ty,
    current_offset: usize,
) -> Result<Vec<ValidValueReq>, String> {
    let layout = ty.layout().unwrap().shape();
    let ty_req = || {
        if let Some(mut req) = ValidValueReq::try_from_ty(tcx, machine_info, ty)
            && !req.is_full()
        {
            req.offset = current_offset;
            vec![req]
        } else {
            vec![]
        }
    };
    // A pattern type's validity is fully captured by the valid_range of its
    // layout's first scalar (see `try_from_ty`): integer bases add no
    // requirements of their own, and the `NotNull` constraint is part of the
    // range. This holds for a `Scalar` base and for a `ScalarPair` base (a
    // pattern over a wide pointer, whose first scalar is the data pointer).
    // Patterns over `char` are rejected below. Handle it here without
    // inspecting the stable kind, which would panic for kinds `rustc_public`
    // cannot yet convert (e.g. `PatternKind::NotNull` for `NonNull`).
    if let rustc_middle::ty::TyKind::Pat(base_ty, _) = rustc_internal::internal(tcx, ty).kind() {
        // `char`'s validity (two intervals around the surrogate gap) exceeds
        // what a single scalar range can express, and the `char` special case
        // in `try_from_ty` is not applied to pattern types; reject instead of
        // under-checking.
        if base_ty.is_char() {
            return Err("Unsupported pattern type over `char`".to_string());
        }
        // A pattern over a wide pointer (e.g. `NonNull<[u8]>`'s `*const [u8] is !null`, which
        // every allocation reaches since nightly-2026-03-21) has a `ScalarPair` ABI whose
        // first scalar is the data pointer, so `try_from_ty` checks it like `&[u8]`. Any other
        // ABI is reported as unsupported rather than asserted on, so `-Z valid-value-checks`
        // never ICEs.
        if !matches!(layout.value_repr, ValueRepr::Scalar(..) | ValueRepr::ScalarPair { .. }) {
            return Err(format!("Unsupported pattern type over a non-scalar type: {ty}"));
        }
        return Ok(ty_req());
    }
    match layout.fields {
        FieldsShape::Primitive => Ok(ty_req()),
        FieldsShape::Array { stride, count } if count > 0 => {
            let TyKind::RigidTy(RigidTy::Array(elem_ty, _)) = ty.kind() else { unreachable!() };
            let elem_validity = ty_validity_per_offset(tcx, machine_info, elem_ty, current_offset)?;
            let mut result = vec![];
            if !elem_validity.is_empty() {
                for idx in 0..count {
                    let idx: usize = idx.try_into().unwrap();
                    let elem_offset = idx * stride.bytes();
                    let mut next_validity = elem_validity
                        .iter()
                        .cloned()
                        .map(|mut req| {
                            req.offset += elem_offset;
                            req
                        })
                        .collect::<Vec<_>>();
                    result.append(&mut next_validity)
                }
            }
            Ok(result)
        }
        FieldsShape::Arbitrary { ref offsets } => {
            match ty.kind().rigid().unwrap_or_else(|| panic!("unexpected type: {ty:?}")) {
                RigidTy::Adt(def, args) => {
                    match def.kind() {
                        AdtKind::Enum => {
                            // Support basic enumeration forms
                            let ty_variants = def.variants();
                            match layout.variants {
                                VariantsShape::Empty => Ok(vec![]),
                                VariantsShape::Single { index } => {
                                    // Only one variant is reachable. This behaves like a struct.
                                    let fields = ty_variants[index.to_index()].fields();
                                    let mut fields_validity = vec![];
                                    for idx in layout.fields.fields_by_offset_order() {
                                        let field_offset = offsets[idx].bytes();
                                        let field_ty = fields[idx].ty_with_args(args);
                                        fields_validity.append(&mut ty_validity_per_offset(
                                            tcx,
                                            machine_info,
                                            field_ty,
                                            field_offset + current_offset,
                                        )?);
                                    }
                                    Ok(fields_validity)
                                }
                                VariantsShape::Multiple {
                                    tag_encoding: TagEncoding::Niche { untagged_variant, .. },
                                    ref variants,
                                    ..
                                } if matches!(layout.value_repr, ValueRepr::Scalar(_))
                                    && variants.iter().enumerate().all(|(index, variant)| {
                                        index == untagged_variant.to_index()
                                            || variant.offsets.is_empty()
                                    })
                                    && niche_payload_is_single_range(
                                        tcx,
                                        machine_info,
                                        &ty_variants[untagged_variant.to_index()],
                                        args,
                                    ) =>
                                {
                                    // A niche-optimized enum like `Option<NonNull<T>>`,
                                    // `Option<&T>` or `Option<NonZero<u32>>`: its layout is a single
                                    // scalar, every variant except the untagged one is fieldless and
                                    // thus encoded purely by niche values of that scalar, and the
                                    // untagged variant's payload is that same scalar. The enum's own
                                    // scalar `valid_range` is the payload's valid range widened by
                                    // the niche values, so it describes exactly the enum's valid
                                    // values, provided the payload's validity is itself a single
                                    // range (checked above; e.g. `char`'s is not).
                                    Ok(ty_req())
                                }
                                VariantsShape::Multiple {
                                    tag_encoding: TagEncoding::Niche { .. },
                                    ..
                                } => {
                                    Err(format!("Unsupported Enum `{}` check", def.trimmed_name()))?
                                }
                                VariantsShape::Multiple { variants, .. } => {
                                    let enum_validity = ty_req();
                                    let mut fields_validity = vec![];
                                    for (index, variant) in variants.iter().enumerate() {
                                        let fields = ty_variants[index].fields();
                                        for field_idx in variant.fields_by_offset_order() {
                                            let field_offset = variant.offsets[field_idx].bytes();
                                            let field_ty = fields[field_idx].ty_with_args(args);
                                            fields_validity.append(&mut ty_validity_per_offset(
                                                tcx,
                                                machine_info,
                                                field_ty,
                                                field_offset + current_offset,
                                            )?);
                                        }
                                    }
                                    if fields_validity.is_empty() {
                                        Ok(enum_validity)
                                    } else {
                                        Err(format!(
                                            "Unsupported Enum `{}` check",
                                            def.trimmed_name()
                                        ))
                                    }
                                }
                            }
                        }
                        AdtKind::Union => unreachable!(),
                        AdtKind::Struct => {
                            // If the struct range has niche add that.
                            let mut struct_validity = ty_req();
                            let fields = def.variants_iter().next().unwrap().fields();
                            for idx in layout.fields.fields_by_offset_order() {
                                let field_offset = offsets[idx].bytes();
                                let field_ty = fields[idx].ty_with_args(args);
                                for req in ty_validity_per_offset(
                                    tcx,
                                    machine_info,
                                    field_ty,
                                    field_offset + current_offset,
                                )? {
                                    // A struct whose layout is a single scalar (e.g. the
                                    // `#[repr(packed)]` `Unaligned<bool>`) has the requirement of
                                    // its scalar field already; repeating it changes nothing, but
                                    // would make the struct's validity compare unequal to the
                                    // field's.
                                    if !struct_validity.contains(&req) {
                                        struct_validity.push(req);
                                    }
                                }
                            }
                            Ok(struct_validity)
                        }
                    }
                }
                // Pattern types are fully handled before the match on the
                // layout's field shape, so this arm can never be reached.
                RigidTy::Pat(..) => {
                    unreachable!("pattern types are handled before the field-shape match")
                }
                RigidTy::Tuple(tys) => {
                    let mut tuple_validity = vec![];
                    for idx in layout.fields.fields_by_offset_order() {
                        let field_offset = offsets[idx].bytes();
                        let field_ty = tys[idx];
                        tuple_validity.append(&mut ty_validity_per_offset(
                            tcx,
                            machine_info,
                            field_ty,
                            field_offset + current_offset,
                        )?);
                    }
                    Ok(tuple_validity)
                }
                // A closure is laid out like a tuple of its captures.
                RigidTy::Closure(_, _) => {
                    let rustc_middle::ty::TyKind::Closure(_, args) =
                        rustc_internal::internal(tcx, ty).kind()
                    else {
                        unreachable!("Expected a closure: {ty:?}")
                    };
                    let upvars: Vec<Ty> =
                        args.as_closure().upvar_tys().iter().map(rustc_internal::stable).collect();
                    let mut closure_validity = vec![];
                    for idx in layout.fields.fields_by_offset_order() {
                        closure_validity.append(&mut ty_validity_per_offset(
                            tcx,
                            machine_info,
                            upvars[idx],
                            offsets[idx].bytes() + current_offset,
                        )?);
                    }
                    Ok(closure_validity)
                }
                RigidTy::Bool
                | RigidTy::Char
                | RigidTy::Int(_)
                | RigidTy::Uint(_)
                | RigidTy::Float(_)
                | RigidTy::Never => {
                    unreachable!("Expected primitive layout for {ty:?}")
                }
                RigidTy::Str | RigidTy::Slice(_) | RigidTy::Array(_, _) => {
                    unreachable!("Expected array layout for {ty:?}")
                }
                RigidTy::RawPtr(_, _) | RigidTy::Ref(_, _, _) => {
                    // Fat pointer has arbitrary shape.
                    Ok(ty_req())
                }
                RigidTy::FnDef(_, _)
                | RigidTy::FnPtr(_)
                | RigidTy::Coroutine(_, _)
                | RigidTy::CoroutineClosure(_, _)
                | RigidTy::CoroutineWitness(_, _)
                | RigidTy::Foreign(_)
                | RigidTy::Dynamic(_, _) => Err(format!("Unsupported {ty:?}")),
            }
        }
        FieldsShape::Union(_) | FieldsShape::Array { .. } => {
            /* Anything is valid */
            Ok(vec![])
        }
    }
}

/// The place `*ptr`, if the pointer operand is a place.
fn deref_place(ptr: &Operand) -> Option<Place> {
    let (Operand::Copy(place) | Operand::Move(place)) = ptr else { return None };
    let mut projection = place.projection.clone();
    projection.push(ProjectionElem::Deref);
    Some(Place { local: place.local, projection })
}

/// Whether `elem_ty` is the element type of the array `ty`, or of an array nested in it.
fn is_array_element(tcx: TyCtxt, ty: Ty, elem_ty: Ty) -> bool {
    let mut ty = ty;
    while !is_pattern_type(tcx, ty)
        && let TyKind::RigidTy(RigidTy::Array(inner, _)) = ty.kind()
    {
        if inner == elem_ty {
            return true;
        }
        ty = inner;
    }
    false
}

/// The pointee of the raw pointer or reference type `ty`.
fn pointee_ty(ty: Ty) -> Ty {
    match ty.kind() {
        TyKind::RigidTy(RigidTy::RawPtr(pointee, _))
        | TyKind::RigidTy(RigidTy::Ref(_, pointee, _)) => pointee,
        _ => unreachable!("expected a pointer, found `{ty}`"),
    }
}

fn bool_operand(value: bool, span: Span) -> Operand {
    Operand::Constant(ConstOperand { span, user_ty: None, const_: MirConst::from_bool(value) })
}

/// The locals whose address the body takes, by a reference or a raw borrow of any place in them.
fn address_taken_locals(body: &MutableBody) -> HashSet<Local> {
    body.blocks()
        .iter()
        .flat_map(|block| &block.statements)
        .filter_map(|stmt| match &stmt.kind {
            StatementKind::Assign(_, Rvalue::Ref(_, _, place) | Rvalue::AddressOf(_, place))
                if !place.projection.iter().any(|elem| matches!(elem, ProjectionElem::Deref)) =>
            {
                Some(place.local)
            }
            _ => None,
        })
        .collect()
}

/// Whether `instance` is a harness: a proof, a contract's proof, or an automatic harness.
fn is_harness(instance: Instance, tcx: TyCtxt) -> bool {
    if KaniFunction::try_from(instance)
        == Ok(KaniFunction::Intrinsic(KaniIntrinsic::AutomaticHarness))
    {
        return true;
    }
    let harness_identifiers = [
        vec![
            rustc_span::symbol::Symbol::intern("kanitool"),
            rustc_span::symbol::Symbol::intern("proof_for_contract"),
        ],
        vec![
            rustc_span::symbol::Symbol::intern("kanitool"),
            rustc_span::symbol::Symbol::intern("proof"),
        ],
    ];
    harness_identifiers.iter().any(|attr_path| {
        attrs_by_path(tcx, rustc_internal::internal(tcx, instance.def.def_id()), attr_path)
            .next()
            .is_some()
    })
}

/// Whether `instance` belongs to the state Kani's checks keep (`kani_core::kani_mem_init!` and
/// `kani_core::kani_valid_value!`), which is in the `kani` crate, or in `core` for the standard
/// library.
fn is_instrumentation_model(instance: Instance) -> bool {
    let name = instance.name();
    name.contains("kani::mem_init::") || name.contains("kani::valid_value::")
}
