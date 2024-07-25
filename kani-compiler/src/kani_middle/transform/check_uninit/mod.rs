// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
//! Module containing multiple transformation passes that instrument the code to detect possible UB
//! due to the accesses to uninitialized memory.

use crate::kani_middle::find_fn_def;
use crate::kani_middle::transform::body::CheckType;
use crate::kani_middle::transform::body::{InsertPosition, MutableBody, SourceInstruction};
use relevant_instruction::{InitRelevantInstruction, MemoryInitOp};
use rustc_middle::ty::TyCtxt;
use rustc_smir::rustc_internal;
use stable_mir::mir::mono::Instance;
use stable_mir::mir::{
    AggregateKind, BasicBlockIdx, ConstOperand, Mutability, Operand, Place, Rvalue,
};
use stable_mir::ty::{
    FnDef, GenericArgKind, GenericArgs, MirConst, RigidTy, Ty, TyConst, TyKind, UintTy,
};
use stable_mir::CrateDef;
use std::collections::{HashMap, HashSet};

pub use ty_layout::{PointeeInfo, PointeeLayout};

pub(crate) mod delayed_ub;
pub(crate) mod ptr_uninit;
mod relevant_instruction;
mod ty_layout;

/// Trait that the instrumentation target providers must implement to work with the instrumenter.
trait TargetFinder {
    fn find_next(
        &mut self,
        body: &MutableBody,
        bb: BasicBlockIdx,
        skip_first: bool,
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
#[derive(Debug)]
pub struct UninitInstrumenter<'a> {
    pub check_type: CheckType,
    /// Used to cache FnDef lookups of injected memory initialization functions.
    pub mem_init_fn_cache: &'a mut HashMap<&'static str, FnDef>,
}

impl<'a> UninitInstrumenter<'a> {
    /// Instrument a body with memory initialization checks, the visitor that generates
    /// instrumentation targets must be provided via a TF type parameter.
    fn instrument(
        &mut self,
        tcx: TyCtxt,
        mut body: MutableBody,
        instance: Instance,
        mut target_finder: impl TargetFinder,
    ) -> (bool, MutableBody) {
        // Need to break infinite recursion when memory initialization checks are inserted, so the
        // internal functions responsible for memory initialization are skipped.
        if tcx
            .get_diagnostic_name(rustc_internal::internal(tcx, instance.def.def_id()))
            .map(|diagnostic_name| {
                SKIPPED_DIAGNOSTIC_ITEMS.contains(&diagnostic_name.to_ident_string().as_str())
            })
            .unwrap_or(false)
        {
            return (false, body);
        }

        let orig_len = body.blocks().len();

        // Set of basic block indices for which analyzing first statement should be skipped.
        //
        // This is necessary because some checks are inserted before the source instruction, which, in
        // turn, gets moved to the next basic block. Hence, we would not need to look at the
        // instruction again as a part of new basic block. However, if the check is inserted after the
        // source instruction, we still need to look at the first statement of the new basic block, so
        // we need to keep track of which basic blocks were created as a part of injecting checks after
        // the source instruction.
        let mut skip_first = HashSet::new();

        // Do not cache body.blocks().len() since it will change as we add new checks.
        let mut bb_idx = 0;
        while bb_idx < body.blocks().len() {
            if let Some(candidate) =
                target_finder.find_next(&body, bb_idx, skip_first.contains(&bb_idx))
            {
                self.build_check_for_instruction(tcx, &mut body, candidate, &mut skip_first);
                bb_idx += 1
            } else {
                bb_idx += 1;
            };
        }
        (orig_len != body.blocks().len(), body)
    }

    /// Inject memory initialization checks for each operation in an instruction.
    fn build_check_for_instruction(
        &mut self,
        tcx: TyCtxt,
        body: &mut MutableBody,
        instruction: InitRelevantInstruction,
        skip_first: &mut HashSet<usize>,
    ) {
        let mut source = instruction.source;
        for operation in instruction.before_instruction {
            self.build_check_for_operation(tcx, body, &mut source, operation, skip_first);
        }
        for operation in instruction.after_instruction {
            self.build_check_for_operation(tcx, body, &mut source, operation, skip_first);
        }
    }

    /// Inject memory initialization check for an operation.
    fn build_check_for_operation(
        &mut self,
        tcx: TyCtxt,
        body: &mut MutableBody,
        source: &mut SourceInstruction,
        operation: MemoryInitOp,
        skip_first: &mut HashSet<usize>,
    ) {
        if let MemoryInitOp::Unsupported { reason } | MemoryInitOp::TriviallyUnsafe { reason } =
            &operation
        {
            collect_skipped(&operation, body, skip_first);
            self.inject_assert_false(tcx, body, source, operation.position(), reason);
            return;
        };

        let pointee_ty_info = {
            let ptr_operand = operation.mk_operand(body, source);
            let ptr_operand_ty = ptr_operand.ty(body.locals()).unwrap();
            let pointee_ty = match ptr_operand_ty.kind() {
                TyKind::RigidTy(RigidTy::RawPtr(pointee_ty, _)) => pointee_ty,
                _ => {
                    unreachable!(
                        "Should only build checks for raw pointers, `{ptr_operand_ty}` encountered."
                    )
                }
            };
            match PointeeInfo::from_ty(pointee_ty) {
                Ok(type_info) => type_info,
                Err(_) => {
                    let reason = format!(
                        "Kani currently doesn't support checking memory initialization for pointers to `{pointee_ty}.",
                    );
                    collect_skipped(&operation, body, skip_first);
                    self.inject_assert_false(tcx, body, source, operation.position(), &reason);
                    return;
                }
            }
        };

        match operation {
            MemoryInitOp::CheckSliceChunk { .. }
            | MemoryInitOp::Check { .. }
            | MemoryInitOp::CheckRef { .. } => {
                self.build_get_and_check(tcx, body, source, operation, pointee_ty_info, skip_first)
            }
            MemoryInitOp::SetSliceChunk { .. }
            | MemoryInitOp::Set { .. }
            | MemoryInitOp::SetRef { .. } => {
                self.build_set(tcx, body, source, operation, pointee_ty_info, skip_first)
            }
            MemoryInitOp::Unsupported { .. } | MemoryInitOp::TriviallyUnsafe { .. } => {
                unreachable!()
            }
        }
    }

    /// Inject a load from memory initialization state and an assertion that all non-padding bytes
    /// are initialized.
    fn build_get_and_check(
        &mut self,
        tcx: TyCtxt,
        body: &mut MutableBody,
        source: &mut SourceInstruction,
        operation: MemoryInitOp,
        pointee_info: PointeeInfo,
        skip_first: &mut HashSet<usize>,
    ) {
        let ret_place = Place {
            local: body.new_local(Ty::bool_ty(), source.span(body.blocks()), Mutability::Not),
            projection: vec![],
        };
        let ptr_operand = operation.mk_operand(body, source);
        match pointee_info.layout() {
            PointeeLayout::Sized { layout } => {
                let layout_operand = mk_layout_operand(body, source, operation.position(), &layout);
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
                    get_mem_init_fn_def(tcx, diagnostic, &mut self.mem_init_fn_cache),
                    layout.len(),
                    *pointee_info.ty(),
                );
                collect_skipped(&operation, body, skip_first);
                body.insert_call(
                    &is_ptr_initialized_instance,
                    source,
                    operation.position(),
                    args,
                    ret_place.clone(),
                );
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
                    get_mem_init_fn_def(tcx, diagnostic, &mut self.mem_init_fn_cache),
                    element_layout.len(),
                    slicee_ty,
                );
                let layout_operand =
                    mk_layout_operand(body, source, operation.position(), &element_layout);
                collect_skipped(&operation, body, skip_first);
                body.insert_call(
                    &is_ptr_initialized_instance,
                    source,
                    operation.position(),
                    vec![ptr_operand.clone(), layout_operand],
                    ret_place.clone(),
                );
            }
            PointeeLayout::TraitObject => {
                collect_skipped(&operation, body, skip_first);
                let reason = "Kani does not support reasoning about memory initialization of pointers to trait objects.";
                self.inject_assert_false(tcx, body, source, operation.position(), reason);
                return;
            }
        };

        // Make sure all non-padding bytes are initialized.
        collect_skipped(&operation, body, skip_first);
        // Find the real operand type for a good error message.
        let operand_ty = match &operation {
            MemoryInitOp::Check { operand }
            | MemoryInitOp::CheckSliceChunk { operand, .. }
            | MemoryInitOp::CheckRef { operand } => operand.ty(body.locals()).unwrap(),
            _ => unreachable!(),
        };
        body.insert_check(
            tcx,
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
        tcx: TyCtxt,
        body: &mut MutableBody,
        source: &mut SourceInstruction,
        operation: MemoryInitOp,
        pointee_info: PointeeInfo,
        skip_first: &mut HashSet<usize>,
    ) {
        let ret_place = Place {
            local: body.new_local(Ty::new_tuple(&[]), source.span(body.blocks()), Mutability::Not),
            projection: vec![],
        };
        let ptr_operand = operation.mk_operand(body, source);
        let value = operation.expect_value();

        match pointee_info.layout() {
            PointeeLayout::Sized { layout } => {
                let layout_operand = mk_layout_operand(body, source, operation.position(), &layout);
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
                    get_mem_init_fn_def(tcx, diagnostic, &mut self.mem_init_fn_cache),
                    layout.len(),
                    *pointee_info.ty(),
                );
                collect_skipped(&operation, body, skip_first);
                body.insert_call(
                    &set_ptr_initialized_instance,
                    source,
                    operation.position(),
                    args,
                    ret_place,
                );
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
                    get_mem_init_fn_def(tcx, diagnostic, &mut self.mem_init_fn_cache),
                    element_layout.len(),
                    slicee_ty,
                );
                let layout_operand =
                    mk_layout_operand(body, source, operation.position(), &element_layout);
                collect_skipped(&operation, body, skip_first);
                body.insert_call(
                    &set_ptr_initialized_instance,
                    source,
                    operation.position(),
                    vec![
                        ptr_operand,
                        layout_operand,
                        Operand::Constant(ConstOperand {
                            span: source.span(body.blocks()),
                            user_ty: None,
                            const_: MirConst::from_bool(value),
                        }),
                    ],
                    ret_place,
                );
            }
            PointeeLayout::TraitObject => {
                unreachable!("Cannot change the initialization state of a trait object directly.");
            }
        };
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
    source: &mut SourceInstruction,
    position: InsertPosition,
    layout_byte_mask: &[bool],
) -> Operand {
    Operand::Move(Place {
        local: body.insert_assignment(
            Rvalue::Aggregate(
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
            ),
            source,
            position,
        ),
        projection: vec![],
    })
}

/// If injecting a new call to the function before the current statement, need to skip the original
/// statement when analyzing it as a part of the new basic block.
fn collect_skipped(operation: &MemoryInitOp, body: &MutableBody, skip_first: &mut HashSet<usize>) {
    if operation.position() == InsertPosition::Before {
        let new_bb_idx = body.blocks().len();
        skip_first.insert(new_bb_idx);
    }
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
