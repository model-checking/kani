// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
//! Visitor that collects all instructions relevant to uninitialized memory access.

use crate::kani_middle::transform::body::{InsertPosition, MutableBody, SourceInstruction};
use stable_mir::mir::alloc::GlobalAlloc;
use stable_mir::mir::mono::{Instance, InstanceKind};
use stable_mir::mir::visit::{Location, PlaceContext};
use stable_mir::mir::{
    BasicBlockIdx, CastKind, ConstOperand, LocalDecl, MirVisitor, Mutability,
    NonDivergingIntrinsic, Operand, Place, PointerCoercion, ProjectionElem, Rvalue, Statement,
    StatementKind, Terminator, TerminatorKind,
};
use stable_mir::ty::{ConstantKind, MirConst, RigidTy, Span, TyKind, UintTy};
use strum_macros::AsRefStr;

#[derive(AsRefStr, Clone, Debug)]
pub enum SourceOp {
    Get { operand: Operand, count: Operand, position: InsertPosition },
    Set { operand: Operand, count: Operand, value: bool, position: InsertPosition },
    SetRef { operand: Operand, count: Operand, value: bool, position: InsertPosition },
    Unsupported { reason: String, position: InsertPosition },
}

impl SourceOp {
    pub fn mk_operand(&self, body: &mut MutableBody, source: &mut SourceInstruction) -> Operand {
        match self {
            SourceOp::Get { operand, .. } | SourceOp::Set { operand, .. } => operand.clone(),
            SourceOp::SetRef { operand, .. } => Operand::Copy(Place {
                local: {
                    let place = match operand {
                        Operand::Copy(place) | Operand::Move(place) => place,
                        Operand::Constant(_) => unreachable!(),
                    };
                    body.new_assignment(
                        Rvalue::AddressOf(Mutability::Not, place.clone()),
                        source,
                        self.position(),
                    )
                },
                projection: vec![],
            }),
            SourceOp::Unsupported { .. } => unreachable!(),
        }
    }

    pub fn expect_count(&self) -> Operand {
        match self {
            SourceOp::Get { count, .. }
            | SourceOp::Set { count, .. }
            | SourceOp::SetRef { count, .. } => count.clone(),
            SourceOp::Unsupported { .. } => unreachable!(),
        }
    }

    pub fn expect_value(&self) -> bool {
        match self {
            SourceOp::Set { value, .. } | SourceOp::SetRef { value, .. } => *value,
            SourceOp::Get { .. } | SourceOp::Unsupported { .. } => unreachable!(),
        }
    }

    pub fn position(&self) -> InsertPosition {
        match self {
            SourceOp::Get { position, .. }
            | SourceOp::SetRef { position, .. }
            | SourceOp::Unsupported { position, .. }
            | SourceOp::Set { position, .. } => *position,
        }
    }
}

#[derive(Clone, Debug)]
pub struct InitRelevantInstruction {
    /// The instruction that affects the state of the memory.
    pub source: SourceInstruction,
    /// All memory-related operations that should happen after the instruction.
    pub before_instruction: Vec<SourceOp>,
    /// All memory-related operations that should happen after the instruction.
    pub after_instruction: Vec<SourceOp>,
}

impl InitRelevantInstruction {
    pub fn push_operation(&mut self, source_op: SourceOp) {
        match source_op.position() {
            InsertPosition::Before => self.before_instruction.push(source_op),
            InsertPosition::After => self.after_instruction.push(source_op),
        }
    }
}

pub struct CheckUninitVisitor<'a> {
    locals: &'a [LocalDecl],
    /// Whether we should skip the next instruction, since it might've been instrumented already.
    /// When we instrument an instruction, we partition the basic block, and the instruction that
    /// may trigger UB becomes the first instruction of the basic block, which we need to skip
    /// later.
    skip_next: bool,
    /// The instruction being visited at a given point.
    current: SourceInstruction,
    /// The target instruction that should be verified.
    pub target: Option<InitRelevantInstruction>,
    /// The basic block being visited.
    bb: BasicBlockIdx,
}

fn mk_const_operand(value: usize, span: Span) -> Operand {
    Operand::Constant(ConstOperand {
        span,
        user_ty: None,
        const_: MirConst::try_from_uint(value as u128, UintTy::Usize).unwrap(),
    })
}

fn try_remove_topmost_deref(place: &Place) -> Option<Place> {
    let mut new_place = place.clone();
    if let Some(ProjectionElem::Deref) = new_place.projection.pop() {
        Some(new_place)
    } else {
        None
    }
}

/// Try retrieving instance for the given function operand.
fn try_resolve_instance(locals: &[LocalDecl], func: &Operand) -> Result<Instance, String> {
    let ty = func.ty(locals).unwrap();
    match ty.kind() {
        TyKind::RigidTy(RigidTy::FnDef(def, args)) => Ok(Instance::resolve(def, &args).unwrap()),
        _ => Err(format!(
            "Kani does not support reasoning about memory initialization of arguments to `{ty:?}`."
        )),
    }
}

impl<'a> CheckUninitVisitor<'a> {
    pub fn find_next(
        body: &'a MutableBody,
        bb: BasicBlockIdx,
        skip_first: bool,
    ) -> Option<InitRelevantInstruction> {
        let mut visitor = CheckUninitVisitor {
            locals: body.locals(),
            skip_next: skip_first,
            current: SourceInstruction::Statement { idx: 0, bb },
            target: None,
            bb,
        };
        visitor.visit_basic_block(&body.blocks()[bb]);
        visitor.target
    }

    fn push_target(&mut self, source_op: SourceOp) {
        let target = self.target.get_or_insert_with(|| InitRelevantInstruction {
            source: self.current,
            after_instruction: vec![],
            before_instruction: vec![],
        });
        target.push_operation(source_op);
    }
}

impl<'a> MirVisitor for CheckUninitVisitor<'a> {
    fn visit_statement(&mut self, stmt: &Statement, location: Location) {
        if self.skip_next {
            self.skip_next = false;
        } else if self.target.is_none() {
            // Leave it as an exhaustive match to be notified when a new kind is added.
            match &stmt.kind {
                StatementKind::Intrinsic(NonDivergingIntrinsic::CopyNonOverlapping(copy)) => {
                    self.super_statement(stmt, location);
                    // Source is a *const T and it must be initialized.
                    self.push_target(SourceOp::Get {
                        operand: copy.src.clone(),
                        count: copy.count.clone(),
                        position: InsertPosition::Before,
                    });
                    // Destimation is a *mut T so it gets initialized.
                    self.push_target(SourceOp::Set {
                        operand: copy.dst.clone(),
                        count: copy.count.clone(),
                        value: true,
                        position: InsertPosition::After,
                    });
                }
                StatementKind::Assign(place, rvalue) => {
                    // First check rvalue.
                    self.visit_rvalue(rvalue, location);
                    // Check whether we are assigning into a dereference (*ptr = _).
                    if let Some(place_without_deref) = try_remove_topmost_deref(place) {
                        if place_without_deref.ty(&self.locals).unwrap().kind().is_raw_ptr() {
                            self.push_target(SourceOp::Set {
                                operand: Operand::Copy(place_without_deref),
                                count: mk_const_operand(1, location.span()),
                                value: true,
                                position: InsertPosition::After,
                            });
                        }
                    }
                    // Check whether Rvalue creates a new initialized pointer previously not captured inside shadow memory.
                    if place.ty(&self.locals).unwrap().kind().is_raw_ptr() {
                        if let Rvalue::AddressOf(..) = rvalue {
                            self.push_target(SourceOp::Set {
                                operand: Operand::Copy(place.clone()),
                                count: mk_const_operand(1, location.span()),
                                value: true,
                                position: InsertPosition::After,
                            });
                        }
                    }
                }
                StatementKind::Deinit(place) => {
                    self.super_statement(stmt, location);
                    self.push_target(SourceOp::Set {
                        operand: Operand::Copy(place.clone()),
                        count: mk_const_operand(1, location.span()),
                        value: false,
                        position: InsertPosition::After,
                    });
                }
                StatementKind::FakeRead(_, _)
                | StatementKind::SetDiscriminant { .. }
                | StatementKind::StorageLive(_)
                | StatementKind::StorageDead(_)
                | StatementKind::Retag(_, _)
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
                    self.super_terminator(term, location);
                    let instance = match try_resolve_instance(self.locals, func) {
                        Ok(instance) => instance,
                        Err(reason) => {
                            self.super_terminator(term, location);
                            self.push_target(SourceOp::Unsupported {
                                reason,
                                position: InsertPosition::Before,
                            });
                            return;
                        }
                    };
                    match instance.kind {
                        InstanceKind::Intrinsic => {
                            match instance.intrinsic_name().unwrap().as_str() {
                                "add_with_overflow"
                                | "arith_offset"
                                | "assert_inhabited"
                                | "assert_mem_uninitialized_valid"
                                | "assert_zero_valid"
                                | "assume" => {}
                                "atomic_and_seqcst"
                                | "atomic_and_acquire"
                                | "atomic_and_acqrel"
                                | "atomic_and_release"
                                | "atomic_and_relaxed"
                                | "atomic_max_seqcst"
                                | "atomic_max_acquire"
                                | "atomic_max_acqrel"
                                | "atomic_max_release"
                                | "atomic_max_relaxed"
                                | "atomic_min_seqcst"
                                | "atomic_min_acquire"
                                | "atomic_min_acqrel"
                                | "atomic_min_release"
                                | "atomic_min_relaxed"
                                | "atomic_nand_seqcst"
                                | "atomic_nand_acquire"
                                | "atomic_nand_acqrel"
                                | "atomic_nand_release"
                                | "atomic_nand_relaxed"
                                | "atomic_or_seqcst"
                                | "atomic_or_acquire"
                                | "atomic_or_acqrel"
                                | "atomic_or_release"
                                | "atomic_or_relaxed"
                                | "atomic_umax_seqcst"
                                | "atomic_umax_acquire"
                                | "atomic_umax_acqrel"
                                | "atomic_umax_release"
                                | "atomic_umax_relaxed"
                                | "atomic_umin_seqcst"
                                | "atomic_umin_acquire"
                                | "atomic_umin_acqrel"
                                | "atomic_umin_release"
                                | "atomic_umin_relaxed"
                                | "atomic_xadd_seqcst"
                                | "atomic_xadd_acquire"
                                | "atomic_xadd_acqrel"
                                | "atomic_xadd_release"
                                | "atomic_xadd_relaxed"
                                | "atomic_xor_seqcst"
                                | "atomic_xor_acquire"
                                | "atomic_xor_acqrel"
                                | "atomic_xor_release"
                                | "atomic_xor_relaxed"
                                | "atomic_xsub_seqcst"
                                | "atomic_xsub_acquire"
                                | "atomic_xsub_acqrel"
                                | "atomic_xsub_release"
                                | "atomic_xsub_relaxed" => {
                                    assert_eq!(
                                        args.len(),
                                        2,
                                        "Unexpected number of arguments for `atomic_binop`"
                                    );
                                    assert!(matches!(
                                        args[0].ty(self.locals).unwrap().kind(),
                                        TyKind::RigidTy(RigidTy::RawPtr(_, Mutability::Mut))
                                    ));
                                    self.push_target(SourceOp::Set {
                                        operand: args[0].clone(),
                                        count: mk_const_operand(1, location.span()),
                                        value: true,
                                        position: InsertPosition::After,
                                    });
                                }
                                "atomic_xchg_seqcst"
                                | "atomic_xchg_acquire"
                                | "atomic_xchg_acqrel"
                                | "atomic_xchg_release"
                                | "atomic_xchg_relaxed"
                                | "atomic_store_seqcst"
                                | "atomic_store_release"
                                | "atomic_store_relaxed"
                                | "atomic_store_unordered" => {
                                    assert_eq!(
                                        args.len(),
                                        2,
                                        "Unexpected number of arguments for `atomic_store`"
                                    );
                                    assert!(matches!(
                                        args[0].ty(self.locals).unwrap().kind(),
                                        TyKind::RigidTy(RigidTy::RawPtr(_, Mutability::Mut))
                                    ));
                                    self.push_target(SourceOp::Set {
                                        operand: args[0].clone(),
                                        count: mk_const_operand(1, location.span()),
                                        value: true,
                                        position: InsertPosition::After,
                                    });
                                }
                                "atomic_load_seqcst"
                                | "atomic_load_acquire"
                                | "atomic_load_relaxed"
                                | "atomic_load_unordered" => {
                                    assert_eq!(
                                        args.len(),
                                        2,
                                        "Unexpected number of arguments for `atomic_load`"
                                    );
                                    assert!(matches!(
                                        args[0].ty(self.locals).unwrap().kind(),
                                        TyKind::RigidTy(RigidTy::RawPtr(_, Mutability::Not))
                                    ));
                                    self.push_target(SourceOp::Get {
                                        operand: args[0].clone(),
                                        count: mk_const_operand(1, location.span()),
                                        position: InsertPosition::Before,
                                    });
                                }
                                name if name.starts_with("atomic_cxchg") => {
                                    assert_eq!(
                                        args.len(),
                                        3,
                                        "Unexpected number of arguments for `atomic_cxchg"
                                    );
                                    assert!(matches!(
                                        args[0].ty(self.locals).unwrap().kind(),
                                        TyKind::RigidTy(RigidTy::RawPtr(_, Mutability::Mut))
                                    ));
                                    self.push_target(SourceOp::Get {
                                        operand: args[0].clone(),
                                        count: mk_const_operand(1, location.span()),
                                        position: InsertPosition::Before,
                                    });
                                }
                                "bitreverse" | "black_box" | "breakpoint" | "bswap"
                                | "caller_location" => {}
                                "catch_unwind" => {
                                    unimplemented!()
                                }
                                "ceilf32" | "ceilf64" => {}
                                "compare_bytes" => {
                                    assert_eq!(
                                        args.len(),
                                        3,
                                        "Unexpected number of arguments for `compare_bytes`"
                                    );
                                    assert!(matches!(
                                        args[0].ty(self.locals).unwrap().kind(),
                                        TyKind::RigidTy(RigidTy::RawPtr(_, Mutability::Not))
                                    ));
                                    assert!(matches!(
                                        args[1].ty(self.locals).unwrap().kind(),
                                        TyKind::RigidTy(RigidTy::RawPtr(_, Mutability::Not))
                                    ));
                                    self.push_target(SourceOp::Get {
                                        operand: args[0].clone(),
                                        count: args[2].clone(),
                                        position: InsertPosition::Before,
                                    });
                                    self.push_target(SourceOp::Get {
                                        operand: args[1].clone(),
                                        count: args[2].clone(),
                                        position: InsertPosition::Before,
                                    });
                                }
                                "copy" => {
                                    assert_eq!(
                                        args.len(),
                                        3,
                                        "Unexpected number of arguments for `copy`"
                                    );
                                    assert!(matches!(
                                        args[0].ty(self.locals).unwrap().kind(),
                                        TyKind::RigidTy(RigidTy::RawPtr(_, Mutability::Not))
                                    ));
                                    assert!(matches!(
                                        args[1].ty(self.locals).unwrap().kind(),
                                        TyKind::RigidTy(RigidTy::RawPtr(_, Mutability::Mut))
                                    ));
                                    self.push_target(SourceOp::Get {
                                        operand: args[0].clone(),
                                        count: args[2].clone(),
                                        position: InsertPosition::Before,
                                    });
                                    self.push_target(SourceOp::Set {
                                        operand: args[1].clone(),
                                        count: args[2].clone(),
                                        value: true,
                                        position: InsertPosition::After,
                                    });
                                }
                                "copy_nonoverlapping" => unreachable!(
                                    "Expected `core::intrinsics::unreachable` to be handled by `StatementKind::CopyNonOverlapping`"
                                ),
                                "copysignf32"
                                | "copysignf64"
                                | "cosf32"
                                | "cosf64"
                                | "ctlz"
                                | "ctlz_nonzero"
                                | "ctpop"
                                | "cttz"
                                | "cttz_nonzero"
                                | "discriminant_value"
                                | "exact_div"
                                | "exp2f32"
                                | "exp2f64"
                                | "expf32"
                                | "expf64"
                                | "fabsf32"
                                | "fabsf64"
                                | "fadd_fast"
                                | "fdiv_fast"
                                | "floorf32"
                                | "floorf64"
                                | "fmaf32"
                                | "fmaf64"
                                | "fmul_fast"
                                | "forget"
                                | "fsub_fast"
                                | "is_val_statically_known"
                                | "likely"
                                | "log10f32"
                                | "log10f64"
                                | "log2f32"
                                | "log2f64"
                                | "logf32"
                                | "logf64"
                                | "maxnumf32"
                                | "maxnumf64"
                                | "min_align_of"
                                | "min_align_of_val"
                                | "minnumf32"
                                | "minnumf64"
                                | "mul_with_overflow"
                                | "nearbyintf32"
                                | "nearbyintf64"
                                | "needs_drop" => {}
                                "offset" => unreachable!(
                                    "Expected `core::intrinsics::unreachable` to be handled by `BinOp::OffSet`"
                                ),
                                "powf32" | "powf64" | "powif32" | "powif64" | "pref_align_of" => {}
                                "ptr_guaranteed_cmp"
                                | "ptr_offset_from"
                                | "ptr_offset_from_unsigned" => {
                                    /* AFAICS from the documentation, none of those require the pointer arguments to be actually initialized. */
                                }
                                "raw_eq" | "retag_box_to_raw" => {
                                    unreachable!("This was removed in the latest Rust version.")
                                }
                                "rintf32" | "rintf64" | "rotate_left" | "rotate_right"
                                | "roundf32" | "roundf64" | "saturating_add" | "saturating_sub"
                                | "sinf32" | "sinf64" => {}
                                name if name.starts_with("simd") => { /* SIMD operations */ }
                                "size_of" => unreachable!(),
                                "size_of_val" => {
                                    /* AFAICS from the documentation, this does not require the pointer argument to be initialized. */
                                }
                                "sqrtf32" | "sqrtf64" | "sub_with_overflow" => {}
                                "transmute" | "transmute_copy" => {
                                    unreachable!("Should've been lowered")
                                }
                                "truncf32" | "truncf64" | "type_id" | "type_name" => {}
                                "typed_swap" => {
                                    assert_eq!(
                                        args.len(),
                                        2,
                                        "Unexpected number of arguments for `typed_swap`"
                                    );
                                    assert!(matches!(
                                        args[0].ty(self.locals).unwrap().kind(),
                                        TyKind::RigidTy(RigidTy::RawPtr(_, Mutability::Mut))
                                    ));
                                    assert!(matches!(
                                        args[1].ty(self.locals).unwrap().kind(),
                                        TyKind::RigidTy(RigidTy::RawPtr(_, Mutability::Mut))
                                    ));
                                    self.push_target(SourceOp::Get {
                                        operand: args[0].clone(),
                                        count: mk_const_operand(1, location.span()),
                                        position: InsertPosition::Before,
                                    });
                                    self.push_target(SourceOp::Get {
                                        operand: args[1].clone(),
                                        count: mk_const_operand(1, location.span()),
                                        position: InsertPosition::Before,
                                    });
                                }
                                "unaligned_volatile_load" => {
                                    assert_eq!(
                                        args.len(),
                                        1,
                                        "Unexpected number of arguments for `unaligned_volatile_load`"
                                    );
                                    assert!(matches!(
                                        args[0].ty(self.locals).unwrap().kind(),
                                        TyKind::RigidTy(RigidTy::RawPtr(_, Mutability::Not))
                                    ));
                                    self.push_target(SourceOp::Get {
                                        operand: args[0].clone(),
                                        count: mk_const_operand(1, location.span()),
                                        position: InsertPosition::Before,
                                    });
                                }
                                "unchecked_add" | "unchecked_mul" | "unchecked_shl"
                                | "unchecked_shr" | "unchecked_sub" => {
                                    unreachable!("Expected intrinsic to be lowered before codegen")
                                }
                                "unchecked_div" | "unchecked_rem" | "unlikely" => {}
                                "unreachable" => unreachable!(
                                    "Expected `std::intrinsics::unreachable` to be handled by `TerminatorKind::Unreachable`"
                                ),
                                "volatile_copy_memory" | "volatile_copy_nonoverlapping_memory" => {
                                    assert_eq!(
                                        args.len(),
                                        3,
                                        "Unexpected number of arguments for `volatile_copy`"
                                    );
                                    assert!(matches!(
                                        args[0].ty(self.locals).unwrap().kind(),
                                        TyKind::RigidTy(RigidTy::RawPtr(_, Mutability::Not))
                                    ));
                                    assert!(matches!(
                                        args[1].ty(self.locals).unwrap().kind(),
                                        TyKind::RigidTy(RigidTy::RawPtr(_, Mutability::Mut))
                                    ));
                                    self.push_target(SourceOp::Get {
                                        operand: args[0].clone(),
                                        count: args[2].clone(),
                                        position: InsertPosition::Before,
                                    });
                                    self.push_target(SourceOp::Set {
                                        operand: args[1].clone(),
                                        count: args[2].clone(),
                                        value: true,
                                        position: InsertPosition::After,
                                    });
                                }
                                "volatile_load" => {
                                    assert_eq!(
                                        args.len(),
                                        1,
                                        "Unexpected number of arguments for `volatile_load`"
                                    );
                                    assert!(matches!(
                                        args[0].ty(self.locals).unwrap().kind(),
                                        TyKind::RigidTy(RigidTy::RawPtr(_, Mutability::Not))
                                    ));
                                    self.push_target(SourceOp::Get {
                                        operand: args[0].clone(),
                                        count: mk_const_operand(1, location.span()),
                                        position: InsertPosition::Before,
                                    });
                                }
                                "volatile_store" => {
                                    assert_eq!(
                                        args.len(),
                                        2,
                                        "Unexpected number of arguments for `volatile_store`"
                                    );
                                    assert!(matches!(
                                        args[0].ty(self.locals).unwrap().kind(),
                                        TyKind::RigidTy(RigidTy::RawPtr(_, Mutability::Mut))
                                    ));
                                    self.push_target(SourceOp::Set {
                                        operand: args[0].clone(),
                                        count: mk_const_operand(1, location.span()),
                                        value: true,
                                        position: InsertPosition::After,
                                    });
                                }
                                "vtable_size" | "vtable_align" | "wrapping_add"
                                | "wrapping_mul" | "wrapping_sub" => {}
                                "write_bytes" => {
                                    assert_eq!(
                                        args.len(),
                                        3,
                                        "Unexpected number of arguments for `write_bytes`"
                                    );
                                    assert!(matches!(
                                        args[0].ty(self.locals).unwrap().kind(),
                                        TyKind::RigidTy(RigidTy::RawPtr(_, Mutability::Mut))
                                    ));
                                    self.push_target(SourceOp::Set {
                                        operand: args[0].clone(),
                                        count: args[2].clone(),
                                        value: true,
                                        position: InsertPosition::After,
                                    })
                                }
                                intrinsic => {
                                    self.push_target(SourceOp::Unsupported {
                                    reason: format!("Kani does not support reasoning about memory initialization of intrinsic `{intrinsic}`."),
                                    position: InsertPosition::Before
                                });
                                }
                            }
                        }
                        InstanceKind::Item => {
                            if instance.is_foreign_item() {
                                match instance.name().as_str() {
                                    "alloc::alloc::__rust_alloc"
                                    | "alloc::alloc::__rust_realloc" => {
                                        /* Memory is uninitialized, nothing to do here. */
                                    }
                                    "alloc::alloc::__rust_alloc_zeroed" => {
                                        /* Memory is initialized here, need to update shadow memory. */
                                        self.push_target(SourceOp::Set {
                                            operand: Operand::Copy(destination.clone()),
                                            count: args[0].clone(),
                                            value: true,
                                            position: InsertPosition::After,
                                        });
                                    }
                                    "alloc::alloc::__rust_dealloc" => {
                                        /* Memory is uninitialized here, need to update shadow memory. */
                                        self.push_target(SourceOp::Set {
                                            operand: args[0].clone(),
                                            count: args[1].clone(),
                                            value: false,
                                            position: InsertPosition::After,
                                        });
                                    }
                                    _ => {}
                                }
                            }
                        }
                        _ => {}
                    }
                }
                TerminatorKind::Drop { place, .. } => {
                    self.super_terminator(term, location);
                    let place_ty = place.ty(&self.locals).unwrap();
                    // When drop is codegen'ed, a reference is taken to the place which is later implicitly coerced to a pointer.
                    // Hence, we need to bless this pointer as initialized.
                    self.push_target(SourceOp::SetRef {
                        operand: Operand::Copy(place.clone()),
                        count: mk_const_operand(1, location.span()),
                        value: true,
                        position: InsertPosition::Before,
                    });
                    if place_ty.kind().is_raw_ptr() {
                        self.push_target(SourceOp::Set {
                            operand: Operand::Copy(place.clone()),
                            count: mk_const_operand(1, location.span()),
                            value: false,
                            position: InsertPosition::After,
                        });
                    }
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
        for (idx, elem) in place.projection.iter().enumerate() {
            let intermediate_place =
                Place { local: place.local, projection: place.projection[..idx].to_vec() };
            match elem {
                ProjectionElem::Deref => {
                    let ptr_ty = intermediate_place.ty(self.locals).unwrap();
                    if ptr_ty.kind().is_raw_ptr() {
                        self.push_target(SourceOp::Get {
                            operand: Operand::Copy(intermediate_place.clone()),
                            count: mk_const_operand(1, location.span()),
                            position: InsertPosition::Before,
                        });
                    }
                }
                ProjectionElem::Field(idx, target_ty) => {
                    if target_ty.kind().is_union()
                        && (!ptx.is_mutating() || place.projection.len() > idx + 1)
                    {
                        self.push_target(SourceOp::Unsupported {
                            reason: "Kani does not support reasoning about memory initialization of unions.".to_string(),
                            position: InsertPosition::Before
                        });
                    }
                }
                ProjectionElem::Index(_)
                | ProjectionElem::ConstantIndex { .. }
                | ProjectionElem::Subslice { .. } => {
                    /* For a slice to be indexed, it should be valid first. */
                }
                ProjectionElem::Downcast(_) => {}
                ProjectionElem::OpaqueCast(_) => {}
                ProjectionElem::Subtype(_) => {}
            }
        }
        self.super_place(place, ptx, location)
    }

    fn visit_operand(&mut self, operand: &Operand, location: Location) {
        if let Operand::Constant(constant) = operand {
            if let ConstantKind::Allocated(allocation) = constant.const_.kind() {
                for (_, prov) in &allocation.provenance.ptrs {
                    if let GlobalAlloc::Static(_) = GlobalAlloc::from(prov.0) {
                        self.push_target(SourceOp::Set {
                            operand: Operand::Constant(constant.clone()),
                            count: mk_const_operand(1, location.span()),
                            value: true,
                            position: InsertPosition::Before,
                        });
                    };
                }
            }
        }
        self.super_operand(operand, location);
    }

    fn visit_rvalue(&mut self, rvalue: &Rvalue, location: Location) {
        if let Rvalue::Cast(CastKind::PointerCoercion(PointerCoercion::Unsize), _, _) = rvalue {
            self.push_target(SourceOp::Unsupported {
                reason: "Kani does not support reasoning about memory initialization of unsized pointers.".to_string(),
                position: InsertPosition::Before
            });
        };
        self.super_rvalue(rvalue, location);
    }
}
