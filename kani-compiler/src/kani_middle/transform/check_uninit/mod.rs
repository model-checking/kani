// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
//! Implement a transformation pass that instruments the code to detect possible UB due to
//! the accesses to uninitialized memory.

use crate::args::ExtraChecks;
use crate::kani_middle::find_fn_def;
use crate::kani_middle::transform::body::{
    CheckType, InsertPosition, MutableBody, SourceInstruction,
};
use crate::kani_middle::transform::{TransformPass, TransformationType};
use crate::kani_queries::QueryDb;
use rustc_middle::ty::TyCtxt;
use rustc_smir::rustc_internal;
use stable_mir::mir::mono::Instance;
use stable_mir::mir::{AggregateKind, Body, ConstOperand, Mutability, Operand, Place, Rvalue};
use stable_mir::ty::{
    FnDef, GenericArgKind, GenericArgs, MirConst, RigidTy, Ty, TyConst, TyKind, UintTy,
};
use stable_mir::CrateDef;
use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use tracing::{debug, trace};

mod ty_layout;
mod uninit_visitor;

pub use ty_layout::{PointeeInfo, PointeeLayout};
use uninit_visitor::{CheckUninitVisitor, InitRelevantInstruction, MemoryInitOp};

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

/// Instrument the code with checks for uninitialized memory.
#[derive(Debug)]
pub struct UninitPass {
    pub check_type: CheckType,
    /// Used to cache FnDef lookups of injected memory initialization functions.
    pub mem_init_fn_cache: HashMap<&'static str, FnDef>,
}

impl TransformPass for UninitPass {
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
        args.ub_check.contains(&ExtraChecks::Uninit)
    }

    fn transform(&mut self, tcx: TyCtxt, body: Body, instance: Instance) -> (bool, Body) {
        trace!(function=?instance.name(), "transform");

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

        let mut new_body = MutableBody::from(body);
        let orig_len = new_body.blocks().len();

        // Inject a call to set-up memory initialization state if the function is a harness.
        if is_harness(instance, tcx) {
            inject_memory_init_setup(&mut new_body, tcx, &mut self.mem_init_fn_cache);
        }

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
        while bb_idx < new_body.blocks().len() {
            if let Some(candidate) =
                CheckUninitVisitor::find_next(&new_body, bb_idx, skip_first.contains(&bb_idx))
            {
                self.build_check_for_instruction(tcx, &mut new_body, candidate, &mut skip_first);
                bb_idx += 1
            } else {
                bb_idx += 1;
            };
        }
        (orig_len != new_body.blocks().len(), new_body.into())
    }
}

impl UninitPass {
    /// Inject memory initialization checks for each operation in an instruction.
    fn build_check_for_instruction(
        &mut self,
        tcx: TyCtxt,
        body: &mut MutableBody,
        instruction: InitRelevantInstruction,
        skip_first: &mut HashSet<usize>,
    ) {
        debug!(?instruction, "build_check");
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
            MemoryInitOp::CheckSliceChunk { .. } | MemoryInitOp::Check { .. } => {
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
                    MemoryInitOp::Check { .. } => {
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
                body.add_call(
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
                body.add_call(
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
        let ptr_operand_ty = ptr_operand.ty(body.locals()).unwrap();
        body.add_check(
            tcx,
            &self.check_type,
            source,
            operation.position(),
            ret_place.local,
            &format!("Undefined Behavior: Reading from an uninitialized pointer of type `{ptr_operand_ty}`"),
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
                body.add_call(
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
                body.add_call(
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
        let result = body.new_assignment(rvalue, source, position);
        body.add_check(tcx, &self.check_type, source, position, result, reason);
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
        local: body.new_assignment(
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

/// Checks if the instance is a harness -- an entry point of Kani analysis.
fn is_harness(instance: Instance, tcx: TyCtxt) -> bool {
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
        tcx.has_attrs_with_path(rustc_internal::internal(tcx, instance.def.def_id()), attr_path)
    })
}

/// Inject an initial call to set-up memory initialization tracking.
fn inject_memory_init_setup(
    new_body: &mut MutableBody,
    tcx: TyCtxt,
    mem_init_fn_cache: &mut HashMap<&'static str, FnDef>,
) {
    // First statement or terminator in the harness.
    let mut source = if !new_body.blocks()[0].statements.is_empty() {
        SourceInstruction::Statement { idx: 0, bb: 0 }
    } else {
        SourceInstruction::Terminator { bb: 0 }
    };

    // Dummy return place.
    let ret_place = Place {
        local: new_body.new_local(
            Ty::new_tuple(&[]),
            source.span(new_body.blocks()),
            Mutability::Not,
        ),
        projection: vec![],
    };

    // Resolve the instance and inject a call to set-up the memory initialization state.
    let memory_initialization_init = Instance::resolve(
        get_mem_init_fn_def(tcx, "KaniInitializeMemoryInitializationState", mem_init_fn_cache),
        &GenericArgs(vec![]),
    )
    .unwrap();

    new_body.add_call(
        &memory_initialization_init,
        &mut source,
        InsertPosition::Before,
        vec![],
        ret_place,
    );
}
