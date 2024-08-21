// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
//! Module containing data structures used in identifying places that need instrumentation and the
//! character of instrumentation needed.

use crate::kani_middle::transform::body::{InsertPosition, MutableBody, SourceInstruction};
use stable_mir::{
    mir::{Mutability, Operand, Place, Rvalue},
    ty::RigidTy,
};
use strum_macros::AsRefStr;

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
    CheckRef { operand: Operand },
    /// Set memory initialization of data bytes in a memory region starting from the reference to
    /// `operand` and of length `sizeof(operand)` bytes.
    SetRef { operand: Operand, value: bool, position: InsertPosition },
    /// Unsupported memory initialization operation.
    Unsupported { reason: String },
    /// Operation that trivially accesses uninitialized memory, results in injecting `assert!(false)`.
    TriviallyUnsafe { reason: String },
    /// Operation that copies memory initialization state over to another operand.
    Copy { from: Operand, to: Operand, count: Operand },
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
            MemoryInitOp::CheckRef { operand, .. } | MemoryInitOp::SetRef { operand, .. } => {
                Operand::Copy(Place {
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
                })
            }
            MemoryInitOp::Unsupported { .. } | MemoryInitOp::TriviallyUnsafe { .. } => {
                unreachable!("operands do not exist for this operation")
            }
            MemoryInitOp::Copy { from, to, .. } => {
                // It does not matter which operand to return for layout generation, since both of
                // them have the same pointee type, so we assert that.
                let from_kind = from.ty(body.locals()).unwrap().kind();
                let to_kind = to.ty(body.locals()).unwrap().kind();

                let RigidTy::RawPtr(from_pointee_ty, _) = from_kind.rigid().unwrap().clone() else {
                    unreachable!()
                };
                let RigidTy::RawPtr(to_pointee_ty, _) = to_kind.rigid().unwrap().clone() else {
                    unreachable!()
                };
                assert!(from_pointee_ty == to_pointee_ty);
                from.clone()
            }
        }
    }

    pub fn expect_count(&self) -> Operand {
        match self {
            MemoryInitOp::CheckSliceChunk { count, .. }
            | MemoryInitOp::SetSliceChunk { count, .. }
            | MemoryInitOp::Copy { count, .. } => count.clone(),
            MemoryInitOp::Check { .. }
            | MemoryInitOp::Set { .. }
            | MemoryInitOp::CheckRef { .. }
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
            | MemoryInitOp::CheckRef { .. }
            | MemoryInitOp::Unsupported { .. }
            | MemoryInitOp::TriviallyUnsafe { .. }
            | MemoryInitOp::Copy { .. } => unreachable!(),
        }
    }

    pub fn position(&self) -> InsertPosition {
        match self {
            MemoryInitOp::Set { position, .. }
            | MemoryInitOp::SetSliceChunk { position, .. }
            | MemoryInitOp::SetRef { position, .. } => *position,
            MemoryInitOp::Check { .. }
            | MemoryInitOp::CheckSliceChunk { .. }
            | MemoryInitOp::CheckRef { .. }
            | MemoryInitOp::Unsupported { .. }
            | MemoryInitOp::TriviallyUnsafe { .. } => InsertPosition::Before,
            MemoryInitOp::Copy { .. } => InsertPosition::After,
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
