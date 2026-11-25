// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
//! Module containing multiple transformation passes that instrument the code to detect possible UB
//! due to the accesses to uninitialized memory.

use crate::kani_middle::nonnull_pointee;
use crate::kani_middle::transform::body::{
    CheckType, InsertPosition, MutableBody, SourceInstruction, synthetic_source_info,
};
use relevant_instruction::{InitRelevantInstruction, MemoryInitOp};
use rustc_public::{
    mir::{
        AggregateKind, BasicBlock, Body, ConstOperand, Mutability, Operand, Place, Rvalue,
        Statement, StatementKind, Terminator, TerminatorKind, UnwindAction, mono::Instance,
    },
    ty::{FnDef, GenericArgKind, GenericArgs, MirConst, RigidTy, Ty, TyConst, TyKind, UintTy},
};
use std::collections::HashMap;

use crate::kani_middle::kani_functions::{KaniFunction, KaniModel};
pub use delayed_ub::DelayedUbPass;
pub use ptr_uninit::UninitPass;
use rustc_public::mir::WithRetag;
pub use ty_layout::{PointeeInfo, PointeeLayout};

mod delayed_ub;
mod ptr_uninit;
mod relevant_instruction;
mod ty_layout;

/// Trait that the instrumentation target providers must implement to work with the instrumenter.
pub trait TargetFinder {
    fn find_all(self, body: &MutableBody) -> Vec<InitRelevantInstruction>;
}

const KANI_IS_PTR_INITIALIZED: KaniFunction = KaniFunction::Model(KaniModel::IsPtrInitialized);
const KANI_SET_PTR_INITIALIZED: KaniFunction = KaniFunction::Model(KaniModel::SetPtrInitialized);
const KANI_IS_SLICE_CHUNK_PTR_INITIALIZED: KaniFunction =
    KaniFunction::Model(KaniModel::IsSliceChunkPtrInitialized);
const KANI_SET_SLICE_CHUNK_PTR_INITIALIZED: KaniFunction =
    KaniFunction::Model(KaniModel::SetSliceChunkPtrInitialized);
const KANI_IS_SLICE_PTR_INITIALIZED: KaniFunction =
    KaniFunction::Model(KaniModel::IsSlicePtrInitialized);
const KANI_SET_SLICE_PTR_INITIALIZED: KaniFunction =
    KaniFunction::Model(KaniModel::SetSlicePtrInitialized);
const KANI_SET_PTR_INITIALIZED_KEEP: KaniFunction =
    KaniFunction::Model(KaniModel::SetPtrInitializedKeep);
const KANI_SET_SLICE_CHUNK_PTR_INITIALIZED_KEEP: KaniFunction =
    KaniFunction::Model(KaniModel::SetSliceChunkPtrInitializedKeep);
const KANI_SET_SLICE_PTR_INITIALIZED_KEEP: KaniFunction =
    KaniFunction::Model(KaniModel::SetSlicePtrInitializedKeep);
const KANI_IS_STR_PTR_INITIALIZED: KaniFunction =
    KaniFunction::Model(KaniModel::IsStrPtrInitialized);
const KANI_SET_STR_PTR_INITIALIZED: KaniFunction =
    KaniFunction::Model(KaniModel::SetStrPtrInitialized);
const KANI_COPY_INIT_STATE: KaniFunction = KaniFunction::Model(KaniModel::CopyInitState);
const KANI_COPY_INIT_STATE_SINGLE: KaniFunction =
    KaniFunction::Model(KaniModel::CopyInitStateSingle);
const KANI_COPY_INIT_STATE_MASKED: KaniFunction =
    KaniFunction::Model(KaniModel::CopyInitStateMasked);
const KANI_LOAD_ARGUMENT: KaniFunction = KaniFunction::Model(KaniModel::LoadArgument);
const KANI_STORE_ARGUMENT: KaniFunction = KaniFunction::Model(KaniModel::StoreArgument);
// The contract-clause-depth functions access the `CONTRACT_CLAUSE_DEPTH`
// static. Instrumenting that access would inject memory-initialization shadow
// writes that CBMC's contract write-set check then flags as illegal side
// effects when a clause is evaluated during contract verification (e.g. a
// `requires(can_dereference(..))` under `-Z uninit-checks`). The counter is
// verifier-internal and always reset at harness entry, so skipping these is
// sound, mirroring the mem-init functions above.
const KANI_ENTER_CONTRACT_CLAUSE: KaniFunction =
    KaniFunction::Model(KaniModel::EnterContractClause);
const KANI_EXIT_CONTRACT_CLAUSE: KaniFunction = KaniFunction::Model(KaniModel::ExitContractClause);
const KANI_IN_CONTRACT_CLAUSE: KaniFunction = KaniFunction::Model(KaniModel::InContractClause);
const KANI_RESET_CONTRACT_CLAUSE_DEPTH: KaniFunction =
    KaniFunction::Model(KaniModel::ResetContractClauseDepth);
// The panic-permission functions access verifier-internal statics, like the
// contract-clause-depth functions above.
const KANI_SET_PANICS_ONLY_IF: KaniFunction = KaniFunction::Model(KaniModel::SetPanicsOnlyIf);
const KANI_SET_PANICS_IF: KaniFunction = KaniFunction::Model(KaniModel::SetPanicsIf);
const KANI_RESTORE_PANIC_PERMISSION: KaniFunction =
    KaniFunction::Model(KaniModel::RestorePanicPermission);

// Function bodies of those functions will not be instrumented as not to cause infinite recursion.
const SKIPPED_ITEMS: &[KaniFunction] = &[
    KANI_IS_PTR_INITIALIZED,
    KANI_SET_PTR_INITIALIZED,
    KANI_IS_SLICE_CHUNK_PTR_INITIALIZED,
    KANI_SET_SLICE_CHUNK_PTR_INITIALIZED,
    KANI_IS_SLICE_PTR_INITIALIZED,
    KANI_SET_SLICE_PTR_INITIALIZED,
    KANI_SET_PTR_INITIALIZED_KEEP,
    KANI_SET_SLICE_CHUNK_PTR_INITIALIZED_KEEP,
    KANI_SET_SLICE_PTR_INITIALIZED_KEEP,
    KANI_IS_STR_PTR_INITIALIZED,
    KANI_SET_STR_PTR_INITIALIZED,
    KANI_COPY_INIT_STATE,
    KANI_COPY_INIT_STATE_SINGLE,
    KANI_COPY_INIT_STATE_MASKED,
    KANI_LOAD_ARGUMENT,
    KANI_STORE_ARGUMENT,
    KANI_ENTER_CONTRACT_CLAUSE,
    KANI_EXIT_CONTRACT_CLAUSE,
    KANI_IN_CONTRACT_CLAUSE,
    KANI_RESET_CONTRACT_CLAUSE_DEPTH,
    KANI_SET_PANICS_ONLY_IF,
    KANI_SET_PANICS_IF,
    KANI_RESTORE_PANIC_PERMISSION,
    // Initializes the shadow memory and the argument buffer, which hold no user data.
    KaniFunction::Model(KaniModel::InitializeMemoryInitializationState),
];

/// Instruments the code with checks for uninitialized memory, agnostic to the source of targets.
pub struct UninitInstrumenter<'a> {
    safety_check_type: CheckType,
    unsupported_check_type: CheckType,
    /// Used to cache FnDef lookups of injected memory initialization functions.
    mem_init_fn_cache: &'a mut HashMap<KaniFunction, FnDef>,
}

impl<'a> UninitInstrumenter<'a> {
    /// Create the instrumenter and run it with the given parameters.
    pub(crate) fn run(
        body: Body,
        instance: Instance,
        safety_check_type: CheckType,
        unsupported_check_type: CheckType,
        mem_init_fn_cache: &'a mut HashMap<KaniFunction, FnDef>,
        target_finder: impl TargetFinder,
    ) -> (bool, Body) {
        let mut instrumenter =
            Self { safety_check_type, unsupported_check_type, mem_init_fn_cache };
        let body = MutableBody::from(body);
        let (changed, new_body) = instrumenter.instrument(body, instance, target_finder);
        (changed, new_body.into())
    }

    /// Instrument a body with memory initialization checks, the visitor that generates
    /// instrumentation targets must be provided via a TF type parameter.
    fn instrument(
        &mut self,
        mut body: MutableBody,
        instance: Instance,
        target_finder: impl TargetFinder,
    ) -> (bool, MutableBody) {
        // Need to break infinite recursion when memory initialization checks are inserted, so the
        // internal functions responsible for memory initialization are skipped.
        if KaniFunction::try_from(instance).map(|f| SKIPPED_ITEMS.contains(&f)).unwrap_or(false) {
            return (false, body);
        }

        let orig_len = body.blocks().len();
        for instruction in target_finder.find_all(&body).into_iter().rev() {
            let source = instruction.source;
            self.build_check_for_instruction(&mut body, instruction, source);
        }
        (orig_len != body.blocks().len(), body)
    }

    /// Inject memory initialization checks for each operation in an instruction.
    fn build_check_for_instruction(
        &mut self,
        body: &mut MutableBody,
        instruction: InitRelevantInstruction,
        mut source: SourceInstruction,
    ) {
        for operation in instruction.before_instruction {
            self.build_check_for_operation(body, &mut source, operation);
        }
        for operation in instruction.after_instruction {
            self.build_check_for_operation(body, &mut source, operation);
        }
    }

    /// Inject memory initialization check for an operation.
    fn build_check_for_operation(
        &mut self,
        body: &mut MutableBody,
        source: &mut SourceInstruction,
        operation: MemoryInitOp,
    ) {
        if let MemoryInitOp::Unsupported { reason } = &operation {
            self.inject_unsupported_check(body, source, operation.position(), reason);
            return;
        } else if let MemoryInitOp::TriviallyUnsafe { reason } = &operation {
            self.inject_safety_check(body, source, operation.position(), reason);
            return;
        };

        let pointee_info = {
            // Sanity check: since CBMC memory object primitives only accept pointers, need to
            // ensure the correct type.
            let ptr_operand_ty = operation.operand_ty(body);
            let pointee_ty =
                if let TyKind::RigidTy(RigidTy::RawPtr(pointee_ty, _)) = ptr_operand_ty.kind() {
                    pointee_ty
                } else if let Some(pointee_ty) = nonnull_pointee(ptr_operand_ty) {
                    // The allocation shims hand us a `NonNull<u8>`, which holds the same address as the
                    // `*mut u8` they used to take.
                    pointee_ty
                } else {
                    unreachable!(
                        "Should only build checks for raw pointers, `{ptr_operand_ty}` encountered."
                    )
                };
            // Calculate pointee layout for byte-by-byte memory initialization checks.
            if let MemoryInitOp::CheckMasked { mask, .. } | MemoryInitOp::SetMasked { mask, .. } =
                &operation
            {
                PointeeInfo::with_mask(pointee_ty, mask.clone())
            } else {
                match PointeeInfo::from_ty(pointee_ty) {
                    Ok(type_info) => type_info,
                    Err(reason) => {
                        let reason = format!(
                            "Kani currently doesn't support checking memory initialization for pointers to `{pointee_ty}. {reason}",
                        );
                        self.inject_unsupported_check(body, source, operation.position(), &reason);
                        return;
                    }
                }
            }
        };

        match &operation {
            MemoryInitOp::CheckSliceChunk { .. }
            | MemoryInitOp::Check { .. }
            | MemoryInitOp::CheckAs { .. }
            | MemoryInitOp::CheckMasked { .. }
            | MemoryInitOp::CheckRef { .. } => {
                self.build_get_and_check(body, source, operation, pointee_info)
            }
            MemoryInitOp::SetSliceChunk { .. }
            | MemoryInitOp::Set { .. }
            | MemoryInitOp::SetRef { .. }
            | MemoryInitOp::SetVtable { .. }
            | MemoryInitOp::SetAs { .. }
            | MemoryInitOp::SetMasked { .. }
            | MemoryInitOp::CreateUnion { .. } => {
                self.build_set(body, source, operation, pointee_info)
            }
            MemoryInitOp::Copy { .. } => self.build_copy(body, source, operation, pointee_info),
            MemoryInitOp::AssignUnion { .. } => {
                self.build_assign_union(body, source, operation, pointee_info)
            }
            MemoryInitOp::StoreArgument { .. }
            | MemoryInitOp::LoadArgument { .. }
            | MemoryInitOp::LoadReturnValue { .. } => {
                self.build_argument_operation(body, source, operation, pointee_info)
            }
            MemoryInitOp::Unsupported { .. } | MemoryInitOp::TriviallyUnsafe { .. } => {
                unreachable!()
            }
        };
    }

    /// Inject a load from memory initialization state and an assertion that all non-padding bytes
    /// are initialized.
    fn build_get_and_check(
        &mut self,
        body: &mut MutableBody,
        source: &mut SourceInstruction,
        operation: MemoryInitOp,
        pointee_info: PointeeInfo,
    ) {
        let ret_place = Place {
            local: body.new_local(Ty::bool_ty(), source.span(body.blocks()), Mutability::Not),
            projection: vec![],
        };
        // Instead of injecting the instrumentation immediately, collect it into a list of
        // statements and a terminator to construct a basic block and inject it at the end.
        let mut statements = vec![];
        let ptr_operand = operation.mk_operand(body, &mut statements, source);
        let terminator = match pointee_info.layout() {
            PointeeLayout::Sized { layout, .. } => {
                let layout_operand = mk_layout_operand(body, &mut statements, source, layout);
                // Depending on whether accessing the known number of elements in the slice, need to
                // pass is as an argument.
                let (diagnostic, args) = match &operation {
                    MemoryInitOp::Check { .. }
                    | MemoryInitOp::CheckAs { .. }
                    | MemoryInitOp::CheckMasked { .. }
                    | MemoryInitOp::CheckRef { .. } => {
                        let diagnostic = KANI_IS_PTR_INITIALIZED;
                        let args = vec![ptr_operand.clone(), layout_operand];
                        (diagnostic, args)
                    }
                    MemoryInitOp::CheckSliceChunk { .. } => {
                        let diagnostic = KANI_IS_SLICE_CHUNK_PTR_INITIALIZED;
                        let args =
                            vec![ptr_operand.clone(), layout_operand, operation.expect_count()];
                        (diagnostic, args)
                    }
                    _ => unreachable!(),
                };
                let is_ptr_initialized_instance = resolve_mem_init_fn(
                    get_mem_init_fn_def(diagnostic, self.mem_init_fn_cache),
                    layout.len(),
                    *pointee_info.ty(),
                );
                Terminator {
                    kind: TerminatorKind::Call {
                        func: Operand::Copy(Place::from(body.new_local(
                            is_ptr_initialized_instance.ty(),
                            source.span(body.blocks()),
                            Mutability::Not,
                        ))),
                        args,
                        destination: ret_place.clone(),
                        target: Some(0), // The current value does not matter, since it will be overwritten in add_bb.
                        unwind: UnwindAction::Terminate,
                    },
                    source_info: synthetic_source_info(source.span(body.blocks())),
                }
            }
            PointeeLayout::Slice { element_layout, .. } => {
                // Since `str`` is a separate type, need to differentiate between [T] and str.
                let (slicee_ty, diagnostic) = match pointee_info.ty().kind() {
                    TyKind::RigidTy(RigidTy::Slice(slicee_ty)) => {
                        (slicee_ty, KANI_IS_SLICE_PTR_INITIALIZED)
                    }
                    TyKind::RigidTy(RigidTy::Str) => {
                        (Ty::unsigned_ty(UintTy::U8), KANI_IS_STR_PTR_INITIALIZED)
                    }
                    _ => unreachable!(),
                };
                let is_ptr_initialized_instance = resolve_mem_init_fn(
                    get_mem_init_fn_def(diagnostic, self.mem_init_fn_cache),
                    element_layout.len(),
                    slicee_ty,
                );
                let layout_operand =
                    mk_layout_operand(body, &mut statements, source, element_layout);
                Terminator {
                    kind: TerminatorKind::Call {
                        func: Operand::Copy(Place::from(body.new_local(
                            is_ptr_initialized_instance.ty(),
                            source.span(body.blocks()),
                            Mutability::Not,
                        ))),
                        args: vec![ptr_operand.clone(), layout_operand],
                        destination: ret_place.clone(),
                        target: Some(0), // The current value does not matter, since it will be overwritten in add_bb.
                        unwind: UnwindAction::Terminate,
                    },
                    source_info: synthetic_source_info(source.span(body.blocks())),
                }
            }
            PointeeLayout::TraitObject => {
                // The layout of the value behind the pointer, and so which of its bytes must be
                // initialized, is only known from the vtable at run time. Places `*ptr` of a trait
                // object type are not checked (see `CheckUninitVisitor::visit_place`), so this is
                // a check that a reference to a trait object points to a valid value, e.g. for a
                // transmute from a raw pointer to a reference: report it as unsupported.
                let reason = format!(
                    "Kani does not support checking the initialization of the value behind a `{}`.",
                    operation.operand_ty(body)
                );
                self.inject_unsupported_check(body, source, operation.position(), &reason);
                return;
            }
            PointeeLayout::Union { .. } => {
                // Reading or copying a whole union does not require any of its bytes to be
                // initialized: a union may hold uninitialized bytes (`MaybeUninit::uninit()`).
                // Reading one of its fields is checked with the field's layout (`CheckRef`).
                return;
            }
        };

        // Construct the basic block and insert it into the body.
        body.insert_bb(BasicBlock { statements, terminator }, source, operation.position());

        // Since the check involves a terminator, we cannot add it to the previously constructed
        // basic block. Instead, we insert the check after the basic block.
        let operand_ty = match &operation {
            MemoryInitOp::Check { operand }
            | MemoryInitOp::CheckSliceChunk { operand, .. }
            | MemoryInitOp::CheckRef { operand } => operand.ty(body.locals()).unwrap(),
            MemoryInitOp::CheckAs { .. } | MemoryInitOp::CheckMasked { .. } => {
                operation.operand_ty(body)
            }
            _ => unreachable!(),
        };
        body.insert_check(
            &self.safety_check_type,
            source,
            operation.position(),
            Some(ret_place.local),
            &format!(
                "Undefined Behavior: Reading from an uninitialized pointer of type `{operand_ty}`"
            ),
        )
    }

    /// Inject a store into memory initialization state to initialize or deinitialize all
    /// non-padding bytes.
    fn build_set(
        &mut self,
        body: &mut MutableBody,
        source: &mut SourceInstruction,
        operation: MemoryInitOp,
        pointee_info: PointeeInfo,
    ) {
        let ret_place = Place {
            local: body.new_local(Ty::new_tuple(&[]), source.span(body.blocks()), Mutability::Not),
            projection: vec![],
        };

        // Instead of injecting the instrumentation immediately, collect it into a list of
        // statements and a terminator to construct a basic block and inject it at the end.
        let mut statements = vec![];
        let ptr_operand = operation.mk_operand(body, &mut statements, source);
        let value = operation.expect_value();
        let terminator = match pointee_info.layout() {
            PointeeLayout::Sized { layout, union_mask } => {
                let layout_operand = mk_layout_operand(body, &mut statements, source, layout);
                // Marking the pointee initialized leaves the bytes of nested unions unchanged:
                // their state does not follow from the type (see `UnionMask`). A write of a whole
                // value first gives them the source's state (see `UninitVisitor`).
                let keep = value && !union_mask.is_empty();
                let keep_operand =
                    keep.then(|| mk_layout_operand(body, &mut statements, source, union_mask));
                let value_operand = Operand::Constant(ConstOperand {
                    span: source.span(body.blocks()),
                    user_ty: None,
                    const_: MirConst::from_bool(value),
                });
                // Depending on whether writing to the known number of elements in the slice, need to
                // pass is as an argument.
                let (diagnostic, args) = match &operation {
                    MemoryInitOp::Set { .. }
                    | MemoryInitOp::SetRef { .. }
                    | MemoryInitOp::SetVtable { .. }
                    | MemoryInitOp::SetAs { .. }
                    | MemoryInitOp::SetMasked { .. } => match keep_operand {
                        Some(keep_operand) => (
                            KANI_SET_PTR_INITIALIZED_KEEP,
                            vec![ptr_operand, layout_operand, keep_operand, value_operand],
                        ),
                        None => (
                            KANI_SET_PTR_INITIALIZED,
                            vec![ptr_operand, layout_operand, value_operand],
                        ),
                    },
                    MemoryInitOp::SetSliceChunk { .. } => match keep_operand {
                        Some(keep_operand) => (
                            KANI_SET_SLICE_CHUNK_PTR_INITIALIZED_KEEP,
                            vec![
                                ptr_operand,
                                layout_operand,
                                keep_operand,
                                operation.expect_count(),
                                value_operand,
                            ],
                        ),
                        None => (
                            KANI_SET_SLICE_CHUNK_PTR_INITIALIZED,
                            vec![
                                ptr_operand,
                                layout_operand,
                                operation.expect_count(),
                                value_operand,
                            ],
                        ),
                    },
                    _ => unreachable!(),
                };
                let set_ptr_initialized_instance = resolve_mem_init_fn(
                    get_mem_init_fn_def(diagnostic, self.mem_init_fn_cache),
                    layout.len(),
                    *pointee_info.ty(),
                );
                Terminator {
                    kind: TerminatorKind::Call {
                        func: Operand::Copy(Place::from(body.new_local(
                            set_ptr_initialized_instance.ty(),
                            source.span(body.blocks()),
                            Mutability::Not,
                        ))),
                        args,
                        destination: ret_place.clone(),
                        target: Some(0), // this will be overriden in add_bb
                        unwind: UnwindAction::Terminate,
                    },
                    source_info: synthetic_source_info(source.span(body.blocks())),
                }
            }
            PointeeLayout::Slice { element_layout, element_union_mask } => {
                let keep = value && !element_union_mask.is_empty();
                // Since `str`` is a separate type, need to differentiate between [T] and str.
                let (slicee_ty, diagnostic) = match pointee_info.ty().kind() {
                    TyKind::RigidTy(RigidTy::Slice(slicee_ty)) if keep => {
                        (slicee_ty, KANI_SET_SLICE_PTR_INITIALIZED_KEEP)
                    }
                    TyKind::RigidTy(RigidTy::Slice(slicee_ty)) => {
                        (slicee_ty, KANI_SET_SLICE_PTR_INITIALIZED)
                    }
                    TyKind::RigidTy(RigidTy::Str) => {
                        (Ty::unsigned_ty(UintTy::U8), KANI_SET_STR_PTR_INITIALIZED)
                    }
                    _ => unreachable!(),
                };
                let set_ptr_initialized_instance = resolve_mem_init_fn(
                    get_mem_init_fn_def(diagnostic, self.mem_init_fn_cache),
                    element_layout.len(),
                    slicee_ty,
                );
                let layout_operand =
                    mk_layout_operand(body, &mut statements, source, element_layout);
                let value_operand = Operand::Constant(ConstOperand {
                    span: source.span(body.blocks()),
                    user_ty: None,
                    const_: MirConst::from_bool(value),
                });
                let args = if keep {
                    let keep_operand =
                        mk_layout_operand(body, &mut statements, source, element_union_mask);
                    vec![ptr_operand, layout_operand, keep_operand, value_operand]
                } else {
                    vec![ptr_operand, layout_operand, value_operand]
                };
                Terminator {
                    kind: TerminatorKind::Call {
                        func: Operand::Copy(Place::from(body.new_local(
                            set_ptr_initialized_instance.ty(),
                            source.span(body.blocks()),
                            Mutability::Not,
                        ))),
                        args,
                        destination: ret_place.clone(),
                        target: Some(0), // The current value does not matter, since it will be overwritten in add_bb.
                        unwind: UnwindAction::Terminate,
                    },
                    source_info: synthetic_source_info(source.span(body.blocks())),
                }
            }
            PointeeLayout::TraitObject => {
                // The layout, and so the padding, depends on the concrete type behind the vtable,
                // so the update cannot be applied here.
                //
                // Marking the pointee initialized can be skipped soundly. For a trait object it only
                // happens where a pointer to an existing, valid value is created: from a reference
                // (`&raw const *r`) or for the place a `Drop` terminator drops. An unsized value
                // cannot be assigned through a pointer. Creating a pointer does not change which
                // bytes are initialized, so skipping the update leaves the tracked state as it was.
                // At worst Kani then treats bytes that were not tracked yet as uninitialized: a
                // false alarm rather than a missed one.
                //
                // Marking it uninitialized cannot be skipped: that would leave memory that is no
                // longer initialized marked as initialized.
                if !value {
                    let reason = "Kani does not support marking the memory behind a pointer to a \
                                  trait object as uninitialized.";
                    self.inject_unsupported_check(body, source, operation.position(), reason);
                }
                return;
            }
            PointeeLayout::Union { field_layouts } => {
                // Writing union data, which could be either creating a union from scratch or
                // performing some pointer operations with it. If we are creating a union from
                // scratch, an operation will contain a union field.
                let union_field = match operation.union_field() {
                    Some(field) => field,
                    // No field is written: the union is marked as a whole, e.g. where a pointer
                    // to it is created from a reference, or where a whole union is written through
                    // a pointer (whose state is copied from the source, see `UninitVisitor`).
                    // Marking it initialized then leaves its bytes unchanged, as for unions nested
                    // in other types. Marking it uninitialized resets all of its bytes.
                    None if value => return,
                    None => {
                        let size = field_layouts.iter().map(Vec::len).max().unwrap_or(0);
                        let layout = vec![false; size];
                        let layout_operand =
                            mk_layout_operand(body, &mut statements, source, &layout);
                        let set_instance = resolve_mem_init_fn(
                            get_mem_init_fn_def(KANI_SET_PTR_INITIALIZED, self.mem_init_fn_cache),
                            size,
                            *pointee_info.ty(),
                        );
                        let terminator = Terminator {
                            kind: TerminatorKind::Call {
                                func: Operand::Copy(Place::from(body.new_local(
                                    set_instance.ty(),
                                    source.span(body.blocks()),
                                    Mutability::Not,
                                ))),
                                args: vec![
                                    ptr_operand,
                                    layout_operand,
                                    Operand::Constant(ConstOperand {
                                        span: source.span(body.blocks()),
                                        user_ty: None,
                                        const_: MirConst::from_bool(false),
                                    }),
                                ],
                                destination: ret_place.clone(),
                                target: Some(0), // this will be overriden in add_bb
                                unwind: UnwindAction::Terminate,
                            },
                            source_info: synthetic_source_info(source.span(body.blocks())),
                        };
                        body.insert_bb(
                            BasicBlock { statements, terminator },
                            source,
                            operation.position(),
                        );
                        return;
                    }
                };
                let layout = &field_layouts[union_field];
                let layout_operand = mk_layout_operand(body, &mut statements, source, layout);
                let diagnostic = KANI_SET_PTR_INITIALIZED;
                let args = vec![
                    ptr_operand,
                    layout_operand,
                    Operand::Constant(ConstOperand {
                        span: source.span(body.blocks()),
                        user_ty: None,
                        const_: MirConst::from_bool(value),
                    }),
                ];
                let set_ptr_initialized_instance = resolve_mem_init_fn(
                    get_mem_init_fn_def(diagnostic, self.mem_init_fn_cache),
                    layout.len(),
                    *pointee_info.ty(),
                );
                Terminator {
                    kind: TerminatorKind::Call {
                        func: Operand::Copy(Place::from(body.new_local(
                            set_ptr_initialized_instance.ty(),
                            source.span(body.blocks()),
                            Mutability::Not,
                        ))),
                        args,
                        destination: ret_place.clone(),
                        target: Some(0), // this will be overriden in add_bb
                        unwind: UnwindAction::Terminate,
                    },
                    source_info: synthetic_source_info(source.span(body.blocks())),
                }
            }
        };
        // Construct the basic block and insert it into the body.
        body.insert_bb(BasicBlock { statements, terminator }, source, operation.position());
    }

    /// Copy memory initialization state from one pointer to the other.
    fn build_copy(
        &mut self,
        body: &mut MutableBody,
        source: &mut SourceInstruction,
        operation: MemoryInitOp,
        pointee_info: PointeeInfo,
    ) {
        let ret_place = Place {
            local: body.new_local(Ty::new_tuple(&[]), source.span(body.blocks()), Mutability::Not),
            projection: vec![],
        };
        let layout_size = pointee_info.layout().maybe_size().unwrap();
        let copy_init_state_instance = resolve_mem_init_fn(
            get_mem_init_fn_def(KANI_COPY_INIT_STATE, self.mem_init_fn_cache),
            layout_size,
            *pointee_info.ty(),
        );
        let position = operation.position();
        let (from, to) = operation.expect_copy_operands();
        let count = operation.expect_count();
        body.insert_call(
            &copy_init_state_instance,
            source,
            position,
            vec![from, to, count],
            ret_place.clone(),
        );
    }

    /// Instrument the code to pass information about arguments containing unions. Whenever a
    /// function is called and some of the arguments contain unions, we store the information. And
    /// when we enter the callee, we load the information.
    fn build_argument_operation(
        &mut self,
        body: &mut MutableBody,
        source: &mut SourceInstruction,
        operation: MemoryInitOp,
        pointee_info: PointeeInfo,
    ) {
        let ret_place = Place {
            local: body.new_local(Ty::new_tuple(&[]), source.span(body.blocks()), Mutability::Not),
            projection: vec![],
        };
        let mut statements = vec![];
        let layout_size = pointee_info.layout().maybe_size().unwrap();
        let diagnostic = match operation {
            MemoryInitOp::LoadArgument { .. } | MemoryInitOp::LoadReturnValue { .. } => {
                KANI_LOAD_ARGUMENT
            }
            MemoryInitOp::StoreArgument { .. } => KANI_STORE_ARGUMENT,
            _ => unreachable!(),
        };
        let argument_operation_instance = resolve_mem_init_fn(
            get_mem_init_fn_def(diagnostic, self.mem_init_fn_cache),
            layout_size,
            *pointee_info.ty(),
        );
        let operand = operation.mk_operand(body, &mut statements, source);
        let argument_no = operation.expect_argument_no();
        let terminator = Terminator {
            kind: TerminatorKind::Call {
                func: Operand::Copy(Place::from(body.new_local(
                    argument_operation_instance.ty(),
                    source.span(body.blocks()),
                    Mutability::Not,
                ))),
                args: vec![
                    operand,
                    Operand::Constant(ConstOperand {
                        span: source.span(body.blocks()),
                        user_ty: None,
                        const_: MirConst::try_from_uint(argument_no as u128, UintTy::Usize)
                            .unwrap(),
                    }),
                ],
                destination: ret_place.clone(),
                target: Some(0), // this will be overriden in add_bb
                unwind: UnwindAction::Terminate,
            },
            source_info: synthetic_source_info(source.span(body.blocks())),
        };

        // Construct the basic block and insert it into the body.
        body.insert_bb(BasicBlock { statements, terminator }, source, operation.position());
    }

    /// Copy memory initialization state from one union variable to another.
    fn build_assign_union(
        &mut self,
        body: &mut MutableBody,
        source: &mut SourceInstruction,
        operation: MemoryInitOp,
        pointee_info: PointeeInfo,
    ) {
        let ret_place = Place {
            local: body.new_local(Ty::new_tuple(&[]), source.span(body.blocks()), Mutability::Not),
            projection: vec![],
        };
        let mut statements = vec![];
        let layout_size = pointee_info.layout().maybe_size().unwrap();
        let (from, to) = operation.expect_assign_union_operands(body, &mut statements, source);
        let (model, args) = match pointee_info.layout() {
            // A type with unions nested in it: copy only the union bytes. The data and padding
            // bytes are marked as for any write of the type, which is then independent of the
            // order of the two operations.
            PointeeLayout::Sized { union_mask, .. } if !union_mask.is_empty() => {
                let mask = mk_layout_operand(body, &mut statements, source, union_mask);
                (KANI_COPY_INIT_STATE_MASKED, vec![from, to, mask])
            }
            // A union: all of its bytes take the source's state.
            _ => (KANI_COPY_INIT_STATE_SINGLE, vec![from, to]),
        };
        let copy_init_state_instance = resolve_mem_init_fn(
            get_mem_init_fn_def(model, self.mem_init_fn_cache),
            layout_size,
            *pointee_info.ty(),
        );
        let terminator = Terminator {
            kind: TerminatorKind::Call {
                func: Operand::Copy(Place::from(body.new_local(
                    copy_init_state_instance.ty(),
                    source.span(body.blocks()),
                    Mutability::Not,
                ))),
                args,
                destination: ret_place.clone(),
                target: Some(0), // this will be overriden in add_bb
                unwind: UnwindAction::Terminate,
            },
            source_info: synthetic_source_info(source.span(body.blocks())),
        };

        // Construct the basic block and insert it into the body.
        body.insert_bb(BasicBlock { statements, terminator }, source, operation.position());
    }

    fn inject_safety_check(
        &self,
        body: &mut MutableBody,
        source: &mut SourceInstruction,
        position: InsertPosition,
        reason: &str,
    ) {
        let span = source.span(body.blocks());
        let rvalue = Rvalue::Use(
            Operand::Constant(ConstOperand {
                const_: MirConst::from_bool(false),
                span,
                user_ty: None,
            }),
            WithRetag::No,
        );
        let result = body.insert_assignment(rvalue, source, position);
        body.insert_check(&self.safety_check_type, source, position, Some(result), reason);
    }

    fn inject_unsupported_check(
        &self,
        body: &mut MutableBody,
        source: &mut SourceInstruction,
        position: InsertPosition,
        reason: &str,
    ) {
        body.insert_check(&self.unsupported_check_type, source, position, None, reason);
    }
}

/// Create an operand from a bit array that represents a byte mask for a type layout where padding
/// bytes are marked as `false` and data bytes are marked as `true`.
///
/// For example, the layout for:
/// ```
/// [repr(C)]
/// struct {
///     a: u16,
///     b: u8
/// }
/// ```
/// will have the following byte mask `[true, true, true, false]`.
pub fn mk_layout_operand(
    body: &mut MutableBody,
    statements: &mut Vec<Statement>,
    source: &mut SourceInstruction,
    layout_byte_mask: &[bool],
) -> Operand {
    let span = source.span(body.blocks());
    let rvalue = Rvalue::Aggregate(
        AggregateKind::Array(Ty::bool_ty()),
        layout_byte_mask
            .iter()
            .map(|byte| {
                Operand::Constant(ConstOperand {
                    span: source.span(body.blocks()),
                    user_ty: None,
                    const_: MirConst::from_bool(*byte),
                })
            })
            .collect(),
    );
    let ret_ty = rvalue.ty(body.locals()).unwrap();
    let result = body.new_local(ret_ty, span, Mutability::Not);
    let stmt = Statement {
        kind: StatementKind::Assign(Place::from(result), rvalue),
        source_info: synthetic_source_info(span),
    };
    statements.push(stmt);

    Operand::Move(Place { local: result, projection: vec![] })
}

/// Retrieve a function definition by diagnostic string, caching the result.
pub fn get_mem_init_fn_def(
    diagnostic: KaniFunction,
    cache: &mut HashMap<KaniFunction, FnDef>,
) -> FnDef {
    cache[&diagnostic]
}

/// Resolves a given memory initialization function with passed type parameters.
pub fn resolve_mem_init_fn(fn_def: FnDef, layout_size: usize, associated_type: Ty) -> Instance {
    Instance::resolve(
        fn_def,
        &GenericArgs(vec![
            GenericArgKind::Const(TyConst::try_from_target_usize(layout_size as u64).unwrap()),
            GenericArgKind::Type(associated_type),
        ]),
    )
    .unwrap()
}
