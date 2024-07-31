// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
//! Visitor that collects all instructions relevant to uninitialized memory access.

use crate::kani_middle::transform::body::{InsertPosition, MutableBody, SourceInstruction};
use stable_mir::mir::alloc::GlobalAlloc;
use stable_mir::mir::mono::{Instance, InstanceKind};
use stable_mir::mir::visit::{Location, PlaceContext};
use stable_mir::mir::{
    BasicBlockIdx, CastKind, LocalDecl, MirVisitor, Mutability, NonDivergingIntrinsic, Operand,
    Place, PointerCoercion, ProjectionElem, Rvalue, Statement, StatementKind, Terminator,
    TerminatorKind,
};
use stable_mir::ty::{ConstantKind, RigidTy, Ty, TyKind};
use strum_macros::AsRefStr;

use super::{PointeeInfo, PointeeLayout};

/// Memory initialization operations: set or get memory initialization state for a given pointer.
#[derive(AsRefStr, Clone, Debug)]
pub enum MemoryInitOp {
    /// Check memory initialization of data bytes in a memory region starting from the pointer
    /// `operand` and of length `sizeof(operand)` bytes.
    Check { operand: Operand },
    /// Set memory initialization state of data bytes in a memory region starting from the pointer
    /// `operand` and of length `sizeof(operand)` bytes.
    Set { operand: Operand, value: bool, position: InsertPosition },
    /// Check memory initialization of data bytes in a memory region starting from the pointer
    /// `operand` and of length `count * sizeof(operand)` bytes.
    CheckSliceChunk { operand: Operand, count: Operand },
    /// Set memory initialization state of data bytes in a memory region starting from the pointer
    /// `operand` and of length `count * sizeof(operand)` bytes.
    SetSliceChunk { operand: Operand, count: Operand, value: bool, position: InsertPosition },
    /// Set memory initialization of data bytes in a memory region starting from the reference to
    /// `operand` and of length `sizeof(operand)` bytes.
    SetRef { operand: Operand, value: bool, position: InsertPosition },
    /// Unsupported memory initialization operation.
    Unsupported { reason: String },
    /// Operation that trivially accesses uninitialized memory, results in injecting `assert!(false)`.
    TriviallyUnsafe { reason: String },
}

impl MemoryInitOp {
    /// Produce an operand for the relevant memory initialization related operation. This is mostly
    /// required so that the analysis can create a new local to take a reference in
    /// `MemoryInitOp::SetRef`.
    pub fn mk_operand(&self, body: &mut MutableBody, source: &mut SourceInstruction) -> Operand {
        match self {
            MemoryInitOp::Check { operand, .. }
            | MemoryInitOp::Set { operand, .. }
            | MemoryInitOp::CheckSliceChunk { operand, .. }
            | MemoryInitOp::SetSliceChunk { operand, .. } => operand.clone(),
            MemoryInitOp::SetRef { operand, .. } => Operand::Copy(Place {
                local: {
                    let place = match operand {
                        Operand::Copy(place) | Operand::Move(place) => place,
                        Operand::Constant(_) => unreachable!(),
                    };
                    body.insert_assignment(
                        Rvalue::AddressOf(Mutability::Not, place.clone()),
                        source,
                        self.position(),
                    )
                },
                projection: vec![],
            }),
            MemoryInitOp::Unsupported { .. } | MemoryInitOp::TriviallyUnsafe { .. } => {
                unreachable!()
            }
        }
    }

    pub fn expect_count(&self) -> Operand {
        match self {
            MemoryInitOp::CheckSliceChunk { count, .. }
            | MemoryInitOp::SetSliceChunk { count, .. } => count.clone(),
            MemoryInitOp::Check { .. }
            | MemoryInitOp::Set { .. }
            | MemoryInitOp::SetRef { .. }
            | MemoryInitOp::Unsupported { .. }
            | MemoryInitOp::TriviallyUnsafe { .. } => unreachable!(),
        }
    }

    pub fn expect_value(&self) -> bool {
        match self {
            MemoryInitOp::Set { value, .. }
            | MemoryInitOp::SetSliceChunk { value, .. }
            | MemoryInitOp::SetRef { value, .. } => *value,
            MemoryInitOp::Check { .. }
            | MemoryInitOp::CheckSliceChunk { .. }
            | MemoryInitOp::Unsupported { .. }
            | MemoryInitOp::TriviallyUnsafe { .. } => unreachable!(),
        }
    }

    pub fn position(&self) -> InsertPosition {
        match self {
            MemoryInitOp::Set { position, .. }
            | MemoryInitOp::SetSliceChunk { position, .. }
            | MemoryInitOp::SetRef { position, .. } => *position,
            MemoryInitOp::Check { .. }
            | MemoryInitOp::CheckSliceChunk { .. }
            | MemoryInitOp::Unsupported { .. }
            | MemoryInitOp::TriviallyUnsafe { .. } => InsertPosition::Before,
        }
    }
}

/// Represents an instruction in the source code together with all memory initialization checks/sets
/// that are connected to the memory used in this instruction and whether they should be inserted
/// before or after the instruction.
#[derive(Clone, Debug)]
pub struct InitRelevantInstruction {
    /// The instruction that affects the state of the memory.
    pub source: SourceInstruction,
    /// All memory-related operations that should happen after the instruction.
    pub before_instruction: Vec<MemoryInitOp>,
    /// All memory-related operations that should happen after the instruction.
    pub after_instruction: Vec<MemoryInitOp>,
}

impl InitRelevantInstruction {
    pub fn push_operation(&mut self, source_op: MemoryInitOp) {
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

    fn push_target(&mut self, source_op: MemoryInitOp) {
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
                    self.push_target(MemoryInitOp::CheckSliceChunk {
                        operand: copy.src.clone(),
                        count: copy.count.clone(),
                    });
                    // Destimation is a *mut T so it gets initialized.
                    self.push_target(MemoryInitOp::SetSliceChunk {
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
                        // First, check that we are not dereferencing extra pointers along the way
                        // (e.g., **ptr = _). If yes, check whether these pointers are initialized.
                        let mut place_to_add_projections =
                            Place { local: place_without_deref.local, projection: vec![] };
                        for projection_elem in place_without_deref.projection.iter() {
                            // If the projection is Deref and the current type is raw pointer, check
                            // if it points to initialized memory.
                            if *projection_elem == ProjectionElem::Deref {
                                if let TyKind::RigidTy(RigidTy::RawPtr(..)) =
                                    place_to_add_projections.ty(&self.locals).unwrap().kind()
                                {
                                    self.push_target(MemoryInitOp::Check {
                                        operand: Operand::Copy(place_to_add_projections.clone()),
                                    });
                                };
                            }
                            place_to_add_projections.projection.push(projection_elem.clone());
                        }
                        if place_without_deref.ty(&self.locals).unwrap().kind().is_raw_ptr() {
                            self.push_target(MemoryInitOp::Set {
                                operand: Operand::Copy(place_without_deref),
                                value: true,
                                position: InsertPosition::After,
                            });
                        }
                    }
                    // Check whether Rvalue creates a new initialized pointer previously not captured inside shadow memory.
                    if place.ty(&self.locals).unwrap().kind().is_raw_ptr() {
                        if let Rvalue::AddressOf(..) = rvalue {
                            self.push_target(MemoryInitOp::Set {
                                operand: Operand::Copy(place.clone()),
                                value: true,
                                position: InsertPosition::After,
                            });
                        }
                    }
                }
                StatementKind::Deinit(place) => {
                    self.super_statement(stmt, location);
                    self.push_target(MemoryInitOp::Set {
                        operand: Operand::Copy(place.clone()),
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
                            self.push_target(MemoryInitOp::Unsupported { reason });
                            return;
                        }
                    };
                    match instance.kind {
                        InstanceKind::Intrinsic => {
                            match instance.intrinsic_name().unwrap().as_str() {
                                intrinsic_name if can_skip_intrinsic(intrinsic_name) => {
                                    /* Intrinsics that can be safely skipped */
                                }
                                name if name.starts_with("atomic") => {
                                    let num_args = match name {
                                        // All `atomic_cxchg` intrinsics take `dst, old, src` as arguments.
                                        name if name.starts_with("atomic_cxchg") => 3,
                                        // All `atomic_load` intrinsics take `src` as an argument.
                                        name if name.starts_with("atomic_load") => 1,
                                        // All other `atomic` intrinsics take `dst, src` as arguments.
                                        _ => 2,
                                    };
                                    assert_eq!(
                                        args.len(),
                                        num_args,
                                        "Unexpected number of arguments for `{name}`"
                                    );
                                    assert!(matches!(
                                        args[0].ty(self.locals).unwrap().kind(),
                                        TyKind::RigidTy(RigidTy::RawPtr(..))
                                    ));
                                    self.push_target(MemoryInitOp::Check {
                                        operand: args[0].clone(),
                                    });
                                }
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
                                    self.push_target(MemoryInitOp::CheckSliceChunk {
                                        operand: args[0].clone(),
                                        count: args[2].clone(),
                                    });
                                    self.push_target(MemoryInitOp::CheckSliceChunk {
                                        operand: args[1].clone(),
                                        count: args[2].clone(),
                                    });
                                }
                                "copy"
                                | "volatile_copy_memory"
                                | "volatile_copy_nonoverlapping_memory" => {
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
                                    self.push_target(MemoryInitOp::CheckSliceChunk {
                                        operand: args[0].clone(),
                                        count: args[2].clone(),
                                    });
                                    self.push_target(MemoryInitOp::SetSliceChunk {
                                        operand: args[1].clone(),
                                        count: args[2].clone(),
                                        value: true,
                                        position: InsertPosition::After,
                                    });
                                }
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
                                    self.push_target(MemoryInitOp::Check {
                                        operand: args[0].clone(),
                                    });
                                    self.push_target(MemoryInitOp::Check {
                                        operand: args[1].clone(),
                                    });
                                }
                                "volatile_load" | "unaligned_volatile_load" => {
                                    assert_eq!(
                                        args.len(),
                                        1,
                                        "Unexpected number of arguments for `volatile_load`"
                                    );
                                    assert!(matches!(
                                        args[0].ty(self.locals).unwrap().kind(),
                                        TyKind::RigidTy(RigidTy::RawPtr(_, Mutability::Not))
                                    ));
                                    self.push_target(MemoryInitOp::Check {
                                        operand: args[0].clone(),
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
                                    self.push_target(MemoryInitOp::Set {
                                        operand: args[0].clone(),
                                        value: true,
                                        position: InsertPosition::After,
                                    });
                                }
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
                                    self.push_target(MemoryInitOp::SetSliceChunk {
                                        operand: args[0].clone(),
                                        count: args[2].clone(),
                                        value: true,
                                        position: InsertPosition::After,
                                    })
                                }
                                intrinsic => {
                                    self.push_target(MemoryInitOp::Unsupported {
                                    reason: format!("Kani does not support reasoning about memory initialization of intrinsic `{intrinsic}`."),
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
                                        self.push_target(MemoryInitOp::SetSliceChunk {
                                            operand: Operand::Copy(destination.clone()),
                                            count: args[0].clone(),
                                            value: true,
                                            position: InsertPosition::After,
                                        });
                                    }
                                    "alloc::alloc::__rust_dealloc" => {
                                        /* Memory is uninitialized here, need to update shadow memory. */
                                        self.push_target(MemoryInitOp::SetSliceChunk {
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

                    // When drop is codegen'ed for types that could define their own dropping
                    // behavior, a reference is taken to the place which is later implicitly coerced
                    // to a pointer. Hence, we need to bless this pointer as initialized.
                    match place
                        .ty(&self.locals)
                        .unwrap()
                        .kind()
                        .rigid()
                        .expect("should be working with monomorphized code")
                    {
                        RigidTy::Adt(..) | RigidTy::Dynamic(_, _, _) => {
                            self.push_target(MemoryInitOp::SetRef {
                                operand: Operand::Copy(place.clone()),
                                value: true,
                                position: InsertPosition::Before,
                            });
                        }
                        _ => {}
                    }

                    if place_ty.kind().is_raw_ptr() {
                        self.push_target(MemoryInitOp::Set {
                            operand: Operand::Copy(place.clone()),
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
                        self.push_target(MemoryInitOp::Check {
                            operand: Operand::Copy(intermediate_place.clone()),
                        });
                    }
                }
                ProjectionElem::Field(idx, target_ty) => {
                    if target_ty.kind().is_union()
                        && (!ptx.is_mutating() || place.projection.len() > idx + 1)
                    {
                        self.push_target(MemoryInitOp::Unsupported {
                            reason: "Kani does not support reasoning about memory initialization of unions.".to_string(),
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
                        if constant.ty().kind().is_raw_ptr() {
                            // If a static is a raw pointer, need to mark it as initialized.
                            self.push_target(MemoryInitOp::Set {
                                operand: Operand::Constant(constant.clone()),
                                value: true,
                                position: InsertPosition::Before,
                            });
                        }
                    };
                }
            }
        }
        self.super_operand(operand, location);
    }

    fn visit_rvalue(&mut self, rvalue: &Rvalue, location: Location) {
        if let Rvalue::Cast(cast_kind, operand, ty) = rvalue {
            match cast_kind {
                CastKind::PointerCoercion(PointerCoercion::Unsize) => {
                    if let TyKind::RigidTy(RigidTy::RawPtr(pointee_ty, _)) = ty.kind() {
                        if pointee_ty.kind().is_trait() {
                            self.push_target(MemoryInitOp::Unsupported {
                                reason: "Kani does not support reasoning about memory initialization of unsized pointers.".to_string(),
                            });
                        }
                    }
                }
                CastKind::PtrToPtr => {
                    let operand_ty = operand.ty(&self.locals).unwrap();
                    if let (
                        RigidTy::RawPtr(from_ty, Mutability::Mut),
                        RigidTy::RawPtr(to_ty, Mutability::Mut),
                    ) = (operand_ty.kind().rigid().unwrap(), ty.kind().rigid().unwrap())
                    {
                        if !tys_layout_compatible(from_ty, to_ty) {
                            // If casting from a mutable pointer to a mutable pointer with
                            // different layouts, delayed UB could occur.
                            self.push_target(MemoryInitOp::Unsupported {
                                reason: "Kani does not support reasoning about memory initialization in presence of mutable raw pointer casts that could cause delayed UB.".to_string(),
                            });
                        }
                    }
                }
                CastKind::Transmute => {
                    let operand_ty = operand.ty(&self.locals).unwrap();
                    if let (
                        RigidTy::RawPtr(from_ty, Mutability::Mut),
                        RigidTy::RawPtr(to_ty, Mutability::Mut),
                    ) = (operand_ty.kind().rigid().unwrap(), ty.kind().rigid().unwrap())
                    {
                        if !tys_layout_compatible(from_ty, to_ty) {
                            // If casting from a mutable pointer to a mutable pointer with different
                            // layouts, delayed UB could occur.
                            self.push_target(MemoryInitOp::Unsupported {
                                reason: "Kani does not support reasoning about memory initialization in presence of mutable raw pointer casts that could cause delayed UB.".to_string(),
                            });
                        }
                    } else if !tys_layout_compatible(&operand_ty, &ty) {
                        // If transmuting between two types of incompatible layouts, padding
                        // bytes are exposed, which is UB.
                        self.push_target(MemoryInitOp::TriviallyUnsafe {
                            reason: "Transmuting between types of incompatible layouts."
                                .to_string(),
                        });
                    }
                }
                _ => {}
            }
        };
        self.super_rvalue(rvalue, location);
    }
}

/// Determines if the intrinsic has no memory initialization related function and hence can be
/// safely skipped.
fn can_skip_intrinsic(intrinsic_name: &str) -> bool {
    match intrinsic_name {
        "add_with_overflow"
        | "arith_offset"
        | "assert_inhabited"
        | "assert_mem_uninitialized_valid"
        | "assert_zero_valid"
        | "assume"
        | "bitreverse"
        | "black_box"
        | "breakpoint"
        | "bswap"
        | "caller_location"
        | "ceilf32"
        | "ceilf64"
        | "copysignf32"
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
        | "needs_drop"
        | "powf32"
        | "powf64"
        | "powif32"
        | "powif64"
        | "pref_align_of"
        | "raw_eq"
        | "rintf32"
        | "rintf64"
        | "rotate_left"
        | "rotate_right"
        | "roundf32"
        | "roundf64"
        | "saturating_add"
        | "saturating_sub"
        | "sinf32"
        | "sinf64"
        | "sqrtf32"
        | "sqrtf64"
        | "sub_with_overflow"
        | "truncf32"
        | "truncf64"
        | "type_id"
        | "type_name"
        | "unchecked_div"
        | "unchecked_rem"
        | "unlikely"
        | "vtable_size"
        | "vtable_align"
        | "wrapping_add"
        | "wrapping_mul"
        | "wrapping_sub" => {
            /* Intrinsics that do not interact with memory initialization. */
            true
        }
        "ptr_guaranteed_cmp" | "ptr_offset_from" | "ptr_offset_from_unsigned" | "size_of_val" => {
            /* AFAICS from the documentation, none of those require the pointer arguments to be actually initialized. */
            true
        }
        name if name.starts_with("simd") => {
            /* SIMD operations */
            true
        }
        name if name.starts_with("atomic_fence")
            || name.starts_with("atomic_singlethreadfence") =>
        {
            /* Atomic fences */
            true
        }
        "copy_nonoverlapping" => unreachable!(
            "Expected `core::intrinsics::unreachable` to be handled by `StatementKind::CopyNonOverlapping`"
        ),
        "offset" => unreachable!(
            "Expected `core::intrinsics::unreachable` to be handled by `BinOp::OffSet`"
        ),
        "unreachable" => unreachable!(
            "Expected `std::intrinsics::unreachable` to be handled by `TerminatorKind::Unreachable`"
        ),
        "transmute" | "transmute_copy" | "unchecked_add" | "unchecked_mul" | "unchecked_shl"
        | "size_of" | "unchecked_shr" | "unchecked_sub" => {
            unreachable!("Expected intrinsic to be lowered before codegen")
        }
        "catch_unwind" => {
            unimplemented!("")
        }
        "retag_box_to_raw" => {
            unreachable!("This was removed in the latest Rust version.")
        }
        _ => {
            /* Everything else */
            false
        }
    }
}

/// Try removing a topmost deref projection from a place if it exists, returning a place without it.
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

/// Returns true if `to_ty` has a smaller or equal size and the same padding bytes as `from_ty` up until
/// its size.
fn tys_layout_compatible(from_ty: &Ty, to_ty: &Ty) -> bool {
    // Retrieve layouts to assess compatibility.
    let from_ty_info = PointeeInfo::from_ty(*from_ty);
    let to_ty_info = PointeeInfo::from_ty(*to_ty);
    if let (Ok(from_ty_info), Ok(to_ty_info)) = (from_ty_info, to_ty_info) {
        let from_ty_layout = match from_ty_info.layout() {
            PointeeLayout::Sized { layout } => layout,
            PointeeLayout::Slice { element_layout } => element_layout,
            PointeeLayout::TraitObject => return false,
        };
        let to_ty_layout = match to_ty_info.layout() {
            PointeeLayout::Sized { layout } => layout,
            PointeeLayout::Slice { element_layout } => element_layout,
            PointeeLayout::TraitObject => return false,
        };
        // Ensure `to_ty_layout` does not have a larger size.
        if to_ty_layout.len() <= from_ty_layout.len() {
            // Check data and padding bytes pair-wise.
            if from_ty_layout.iter().zip(to_ty_layout.iter()).all(
                |(from_ty_layout_byte, to_ty_layout_byte)| {
                    // Make sure all data and padding bytes match.
                    from_ty_layout_byte == to_ty_layout_byte
                },
            ) {
                return true;
            }
        }
    };
    false
}
