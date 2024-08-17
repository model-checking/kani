// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
//! Module containing multiple transformation passes that instrument the code to detect possible UB
//! due to the accesses to uninitialized memory.

use crate::kani_middle::{
    find_fn_def,
    transform::body::{CheckType, InsertPosition, MutableBody, SourceInstruction},
};
use relevant_instruction::{InitRelevantInstruction, MemoryInitOp};
use rustc_middle::ty::TyCtxt;
use rustc_smir::rustc_internal;
use stable_mir::{
    mir::{
        mono::Instance, AggregateKind, BasicBlock, Body, ConstOperand, Mutability, Operand, Place,
        Rvalue, Statement, StatementKind, Terminator, TerminatorKind, UnwindAction,
    },
    ty::{FnDef, GenericArgKind, GenericArgs, MirConst, RigidTy, Ty, TyConst, TyKind, UintTy},
    CrateDef,
};
use std::collections::HashMap;

pub use delayed_ub::DelayedUbPass;
pub use ptr_uninit::UninitPass;
pub use ty_layout::{PointeeInfo, PointeeLayout};

mod delayed_ub;
mod ptr_uninit;
mod relevant_instruction;
mod ty_layout;

/// Trait that the instrumentation target providers must implement to work with the instrumenter.
pub trait TargetFinder {
    fn find_next(
        &mut self,
        body: &MutableBody,
        source: &SourceInstruction,
    ) -> Option<InitRelevantInstruction>;
}

// Function bodies of those functions will not be instrumented as not to cause infinite recursion.
const SKIPPED_DIAGNOSTIC_ITEMS: &[&str] = &[
    "KaniIsPtrInitialized",
    "KaniSetPtrInitialized",
    "KaniIsSliceChunkPtrInitialized",
    "KaniSetSliceChunkPtrInitialized",
    "KaniIsSlicePtrInitialized",
    "KaniSetSlicePtrInitialized",
    "KaniIsStrPtrInitialized",
    "KaniSetStrPtrInitialized",
];

/// Instruments the code with checks for uninitialized memory, agnostic to the source of targets.
pub struct UninitInstrumenter<'a, 'tcx> {
    check_type: CheckType,
    /// Used to cache FnDef lookups of injected memory initialization functions.
    mem_init_fn_cache: &'a mut HashMap<&'static str, FnDef>,
    tcx: TyCtxt<'tcx>,
}

impl<'a, 'tcx> UninitInstrumenter<'a, 'tcx> {
    /// Create the instrumenter and run it with the given parameters.
    pub(crate) fn run(
        body: Body,
        tcx: TyCtxt<'tcx>,
        instance: Instance,
        check_type: CheckType,
        mem_init_fn_cache: &'a mut HashMap<&'static str, FnDef>,
        target_finder: impl TargetFinder,
    ) -> (bool, Body) {
        let mut instrumenter = Self { check_type, mem_init_fn_cache, tcx };
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
        mut target_finder: impl TargetFinder,
    ) -> (bool, MutableBody) {
        // Need to break infinite recursion when memory initialization checks are inserted, so the
        // internal functions responsible for memory initialization are skipped.
        if self
            .tcx
            .get_diagnostic_name(rustc_internal::internal(self.tcx, instance.def.def_id()))
            .map(|diagnostic_name| {
                SKIPPED_DIAGNOSTIC_ITEMS.contains(&diagnostic_name.to_ident_string().as_str())
            })
            .unwrap_or(false)
        {
            return (false, body);
        }

        let orig_len = body.blocks().len();
        let mut source = SourceInstruction::Terminator { bb: body.blocks().len() - 1 };
        loop {
            if let Some(candidate) = target_finder.find_next(&body, &source) {
                self.build_check_for_instruction(&mut body, candidate, source);
            }
            source = match source {
                SourceInstruction::Statement { idx, bb } => {
                    if bb == 0 && idx == 0 {
                        break;
                    } else if idx == 0 {
                        SourceInstruction::Terminator { bb: bb - 1 }
                    } else {
                        SourceInstruction::Statement { idx: idx - 1, bb }
                    }
                }
                SourceInstruction::Terminator { bb } => {
                    let stmt_len = body.blocks()[bb].statements.len();
                    if bb == 0 && stmt_len == 0 {
                        break;
                    } else if stmt_len == 0 {
                        SourceInstruction::Terminator { bb: bb - 1 }
                    } else {
                        SourceInstruction::Statement { idx: stmt_len - 1, bb }
                    }
                }
            }
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
        if let MemoryInitOp::Unsupported { reason } | MemoryInitOp::TriviallyUnsafe { reason } =
            &operation
        {
            // If the operation is unsupported or trivially accesses uninitialized memory, encode
            // the check as `assert!(false)`.
            self.inject_assert_false(self.tcx, body, source, operation.position(), reason);
            return;
        };

        let pointee_info = {
            // Sanity check: since CBMC memory object primitives only accept pointers, need to
            // ensure the correct type.
            let ptr_operand_ty = operation.operand_ty(body);
            let pointee_ty = match ptr_operand_ty.kind() {
                TyKind::RigidTy(RigidTy::RawPtr(pointee_ty, _)) => pointee_ty,
                _ => {
                    unreachable!(
                        "Should only build checks for raw pointers, `{ptr_operand_ty}` encountered."
                    )
                }
            };
            // Calculate pointee layout for byte-by-byte memory initialization checks.
            match PointeeInfo::from_ty(pointee_ty) {
                Ok(type_info) => type_info,
                Err(_) => {
                    let reason = format!(
                        "Kani currently doesn't support checking memory initialization for pointers to `{pointee_ty}.",
                    );
                    self.inject_assert_false(self.tcx, body, source, operation.position(), &reason);
                    return;
                }
            }
        };

        match &operation {
            MemoryInitOp::CheckSliceChunk { .. }
            | MemoryInitOp::Check { .. }
            | MemoryInitOp::CheckRef { .. } => {
                self.build_get_and_check(body, source, operation, pointee_info)
            }
            MemoryInitOp::SetSliceChunk { .. }
            | MemoryInitOp::Set { .. }
            | MemoryInitOp::SetRef { .. } => self.build_set(body, source, operation, pointee_info),
            MemoryInitOp::Copy { .. } => self.build_copy(body, source, operation, pointee_info),
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
            PointeeLayout::Sized { layout } => {
                let layout_operand = mk_layout_operand(body, &mut statements, source, &layout);
                // Depending on whether accessing the known number of elements in the slice, need to
                // pass is as an argument.
                let (diagnostic, args) = match &operation {
                    MemoryInitOp::Check { .. } | MemoryInitOp::CheckRef { .. } => {
                        let diagnostic = "KaniIsPtrInitialized";
                        let args = vec![ptr_operand.clone(), layout_operand];
                        (diagnostic, args)
                    }
                    MemoryInitOp::CheckSliceChunk { .. } => {
                        let diagnostic = "KaniIsSliceChunkPtrInitialized";
                        let args =
                            vec![ptr_operand.clone(), layout_operand, operation.expect_count()];
                        (diagnostic, args)
                    }
                    _ => unreachable!(),
                };
                let is_ptr_initialized_instance = resolve_mem_init_fn(
                    get_mem_init_fn_def(self.tcx, diagnostic, &mut self.mem_init_fn_cache),
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
                    span: source.span(body.blocks()),
                }
            }
            PointeeLayout::Slice { element_layout } => {
                // Since `str`` is a separate type, need to differentiate between [T] and str.
                let (slicee_ty, diagnostic) = match pointee_info.ty().kind() {
                    TyKind::RigidTy(RigidTy::Slice(slicee_ty)) => {
                        (slicee_ty, "KaniIsSlicePtrInitialized")
                    }
                    TyKind::RigidTy(RigidTy::Str) => {
                        (Ty::unsigned_ty(UintTy::U8), "KaniIsStrPtrInitialized")
                    }
                    _ => unreachable!(),
                };
                let is_ptr_initialized_instance = resolve_mem_init_fn(
                    get_mem_init_fn_def(self.tcx, diagnostic, &mut self.mem_init_fn_cache),
                    element_layout.len(),
                    slicee_ty,
                );
                let layout_operand =
                    mk_layout_operand(body, &mut statements, source, &element_layout);
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
                    span: source.span(body.blocks()),
                }
            }
            PointeeLayout::TraitObject => {
                let reason = "Kani does not support reasoning about memory initialization of pointers to trait objects.";
                self.inject_assert_false(self.tcx, body, source, operation.position(), reason);
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
            _ => unreachable!(),
        };
        body.insert_check(
            self.tcx,
            &self.check_type,
            source,
            operation.position(),
            ret_place.local,
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
            PointeeLayout::Sized { layout } => {
                let layout_operand = mk_layout_operand(body, &mut statements, source, &layout);
                // Depending on whether writing to the known number of elements in the slice, need to
                // pass is as an argument.
                let (diagnostic, args) = match &operation {
                    MemoryInitOp::Set { .. } | MemoryInitOp::SetRef { .. } => {
                        let diagnostic = "KaniSetPtrInitialized";
                        let args = vec![
                            ptr_operand,
                            layout_operand,
                            Operand::Constant(ConstOperand {
                                span: source.span(body.blocks()),
                                user_ty: None,
                                const_: MirConst::from_bool(value),
                            }),
                        ];
                        (diagnostic, args)
                    }
                    MemoryInitOp::SetSliceChunk { .. } => {
                        let diagnostic = "KaniSetSliceChunkPtrInitialized";
                        let args = vec![
                            ptr_operand,
                            layout_operand,
                            operation.expect_count(),
                            Operand::Constant(ConstOperand {
                                span: source.span(body.blocks()),
                                user_ty: None,
                                const_: MirConst::from_bool(value),
                            }),
                        ];
                        (diagnostic, args)
                    }
                    _ => unreachable!(),
                };
                let set_ptr_initialized_instance = resolve_mem_init_fn(
                    get_mem_init_fn_def(self.tcx, diagnostic, &mut self.mem_init_fn_cache),
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
                    span: source.span(body.blocks()),
                }
            }
            PointeeLayout::Slice { element_layout } => {
                // Since `str`` is a separate type, need to differentiate between [T] and str.
                let (slicee_ty, diagnostic) = match pointee_info.ty().kind() {
                    TyKind::RigidTy(RigidTy::Slice(slicee_ty)) => {
                        (slicee_ty, "KaniSetSlicePtrInitialized")
                    }
                    TyKind::RigidTy(RigidTy::Str) => {
                        (Ty::unsigned_ty(UintTy::U8), "KaniSetStrPtrInitialized")
                    }
                    _ => unreachable!(),
                };
                let set_ptr_initialized_instance = resolve_mem_init_fn(
                    get_mem_init_fn_def(self.tcx, diagnostic, &mut self.mem_init_fn_cache),
                    element_layout.len(),
                    slicee_ty,
                );
                let layout_operand =
                    mk_layout_operand(body, &mut statements, source, &element_layout);
                Terminator {
                    kind: TerminatorKind::Call {
                        func: Operand::Copy(Place::from(body.new_local(
                            set_ptr_initialized_instance.ty(),
                            source.span(body.blocks()),
                            Mutability::Not,
                        ))),
                        args: vec![
                            ptr_operand,
                            layout_operand,
                            Operand::Constant(ConstOperand {
                                span: source.span(body.blocks()),
                                user_ty: None,
                                const_: MirConst::from_bool(value),
                            }),
                        ],
                        destination: ret_place.clone(),
                        target: Some(0), // The current value does not matter, since it will be overwritten in add_bb.
                        unwind: UnwindAction::Terminate,
                    },
                    span: source.span(body.blocks()),
                }
            }
            PointeeLayout::TraitObject => {
                unreachable!("Cannot change the initialization state of a trait object directly.");
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
        let PointeeLayout::Sized { layout } = pointee_info.layout() else { unreachable!() };
        let copy_init_state_instance = resolve_mem_init_fn(
            get_mem_init_fn_def(self.tcx, "KaniCopyInitState", &mut self.mem_init_fn_cache),
            layout.len(),
            *pointee_info.ty(),
        );
        let position = operation.position();
        let MemoryInitOp::Copy { from, to, count } = operation else { unreachable!() };
        body.insert_call(
            &copy_init_state_instance,
            source,
            position,
            vec![from, to, count],
            ret_place.clone(),
        );
    }

    fn inject_assert_false(
        &self,
        tcx: TyCtxt,
        body: &mut MutableBody,
        source: &mut SourceInstruction,
        position: InsertPosition,
        reason: &str,
    ) {
        let span = source.span(body.blocks());
        let rvalue = Rvalue::Use(Operand::Constant(ConstOperand {
            const_: MirConst::from_bool(false),
            span,
            user_ty: None,
        }));
        let result = body.insert_assignment(rvalue, source, position);
        body.insert_check(tcx, &self.check_type, source, position, result, reason);
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
    let stmt = Statement { kind: StatementKind::Assign(Place::from(result), rvalue), span };
    statements.push(stmt);

    Operand::Move(Place { local: result, projection: vec![] })
}

/// Retrieve a function definition by diagnostic string, caching the result.
pub fn get_mem_init_fn_def(
    tcx: TyCtxt,
    diagnostic: &'static str,
    cache: &mut HashMap<&'static str, FnDef>,
) -> FnDef {
    let entry = cache.entry(diagnostic).or_insert_with(|| find_fn_def(tcx, diagnostic).unwrap());
    *entry
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
