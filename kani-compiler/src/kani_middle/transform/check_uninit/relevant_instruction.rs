// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
//! Module containing data structures used in identifying places that need instrumentation and the
//! character of instrumentation needed.

use crate::kani_middle::transform::body::{InsertPosition, MutableBody, SourceInstruction};
use stable_mir::{
    mir::{FieldIdx, Mutability, Operand, Place, Rvalue, Statement, StatementKind},
    ty::{RigidTy, Ty},
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
    /// Copy memory initialization state over to another operand.
    Copy { from: Operand, to: Operand, count: Operand },
    /// Copy memory initialization state over from one union variable to another.
    AssignUnion { lvalue: Place, rvalue: Operand },
    /// Create a union from scratch with a given field index and store it in the provided operand.
    CreateUnion { operand: Operand, field: FieldIdx },
    /// Load argument containing a union from the argument buffer together if the argument number
    /// provided matches.
    LoadArgument { operand: Operand, argument_no: usize },
    /// Store argument containing a union into the argument buffer together with the argument number
    /// provided.
    StoreArgument { operand: Operand, argument_no: usize },
}

impl MemoryInitOp {
    /// Produce an operand for the relevant memory initialization related operation. This is mostly
    /// required so that the analysis can create a new local to take a reference in
    /// `MemoryInitOp::SetRef`.
    pub fn mk_operand(
        &self,
        body: &mut MutableBody,
        statements: &mut Vec<Statement>,
        source: &mut SourceInstruction,
    ) -> Operand {
        match self {
            MemoryInitOp::Check { operand, .. }
            | MemoryInitOp::Set { operand, .. }
            | MemoryInitOp::CheckSliceChunk { operand, .. }
            | MemoryInitOp::SetSliceChunk { operand, .. } => operand.clone(),
            MemoryInitOp::CheckRef { operand, .. }
            | MemoryInitOp::SetRef { operand, .. }
            | MemoryInitOp::CreateUnion { operand, .. }
            | MemoryInitOp::LoadArgument { operand, .. }
            | MemoryInitOp::StoreArgument { operand, .. } => {
                mk_ref(operand, body, statements, source)
            }
            MemoryInitOp::Copy { .. }
            | MemoryInitOp::AssignUnion { .. }
            | MemoryInitOp::Unsupported { .. }
            | MemoryInitOp::TriviallyUnsafe { .. } => {
                unreachable!()
            }
        }
    }

    /// A helper to access operands of copy operation.
    pub fn expect_copy_operands(&self) -> (Operand, Operand) {
        match self {
            MemoryInitOp::Copy { from, to, .. } => (from.clone(), to.clone()),
            _ => unreachable!(),
        }
    }

    /// A helper to access operands of union assign, automatically creates references to them.
    pub fn expect_assign_union_operands(
        &self,
        body: &mut MutableBody,
        statements: &mut Vec<Statement>,
        source: &mut SourceInstruction,
    ) -> (Operand, Operand) {
        match self {
            MemoryInitOp::AssignUnion { lvalue, rvalue } => {
                let lvalue_as_operand = Operand::Copy(lvalue.clone());
                (
                    mk_ref(rvalue, body, statements, source),
                    mk_ref(&lvalue_as_operand, body, statements, source),
                )
            }
            _ => unreachable!(),
        }
    }

    pub fn operand_ty(&self, body: &MutableBody) -> Ty {
        match self {
            MemoryInitOp::Check { operand, .. }
            | MemoryInitOp::Set { operand, .. }
            | MemoryInitOp::CheckSliceChunk { operand, .. }
            | MemoryInitOp::SetSliceChunk { operand, .. } => operand.ty(body.locals()).unwrap(),
            MemoryInitOp::SetRef { operand, .. }
            | MemoryInitOp::CheckRef { operand, .. }
            | MemoryInitOp::CreateUnion { operand, .. }
            | MemoryInitOp::LoadArgument { operand, .. }
            | MemoryInitOp::StoreArgument { operand, .. } => {
                let place = match operand {
                    Operand::Copy(place) | Operand::Move(place) => place,
                    Operand::Constant(_) => unreachable!(),
                };
                let rvalue = Rvalue::AddressOf(Mutability::Not, place.clone());
                rvalue.ty(body.locals()).unwrap()
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
                from.ty(body.locals()).unwrap()
            }
            MemoryInitOp::AssignUnion { lvalue, .. } => {
                // It does not matter which operand to return for layout generation, since both of
                // them have the same pointee type.
                let address_of = Rvalue::AddressOf(Mutability::Not, lvalue.clone());
                address_of.ty(body.locals()).unwrap()
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
            | MemoryInitOp::CreateUnion { .. }
            | MemoryInitOp::AssignUnion { .. }
            | MemoryInitOp::Unsupported { .. }
            | MemoryInitOp::TriviallyUnsafe { .. }
            | MemoryInitOp::StoreArgument { .. }
            | MemoryInitOp::LoadArgument { .. } => unreachable!(),
        }
    }

    pub fn expect_value(&self) -> bool {
        match self {
            MemoryInitOp::Set { value, .. }
            | MemoryInitOp::SetSliceChunk { value, .. }
            | MemoryInitOp::SetRef { value, .. } => *value,
            MemoryInitOp::CreateUnion { .. } => true,
            MemoryInitOp::Check { .. }
            | MemoryInitOp::CheckSliceChunk { .. }
            | MemoryInitOp::CheckRef { .. }
            | MemoryInitOp::Unsupported { .. }
            | MemoryInitOp::TriviallyUnsafe { .. }
            | MemoryInitOp::Copy { .. }
            | MemoryInitOp::AssignUnion { .. }
            | MemoryInitOp::StoreArgument { .. }
            | MemoryInitOp::LoadArgument { .. } => unreachable!(),
        }
    }

    pub fn union_field(&self) -> Option<FieldIdx> {
        match self {
            MemoryInitOp::CreateUnion { field, .. } => Some(*field),
            MemoryInitOp::Check { .. }
            | MemoryInitOp::CheckSliceChunk { .. }
            | MemoryInitOp::CheckRef { .. }
            | MemoryInitOp::Set { .. }
            | MemoryInitOp::SetSliceChunk { .. }
            | MemoryInitOp::SetRef { .. }
            | MemoryInitOp::Unsupported { .. }
            | MemoryInitOp::TriviallyUnsafe { .. }
            | MemoryInitOp::Copy { .. }
            | MemoryInitOp::AssignUnion { .. }
            | MemoryInitOp::StoreArgument { .. }
            | MemoryInitOp::LoadArgument { .. } => None,
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
            | MemoryInitOp::TriviallyUnsafe { .. }
            | MemoryInitOp::StoreArgument { .. }
            | MemoryInitOp::LoadArgument { .. } => InsertPosition::Before,
            MemoryInitOp::Copy { .. }
            | MemoryInitOp::AssignUnion { .. }
            | MemoryInitOp::CreateUnion { .. } => InsertPosition::After,
        }
    }

    pub fn expect_argument_no(&self) -> usize {
        match self {
            MemoryInitOp::LoadArgument { argument_no, .. }
            | MemoryInitOp::StoreArgument { argument_no, .. } => *argument_no,
            _ => unreachable!(),
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

/// A helper to generate instrumentation for taking a reference to a given operand. Returns the
/// operand which is a reference and stores all instrumentation in the statements vector passed.
fn mk_ref(
    operand: &Operand,
    body: &mut MutableBody,
    statements: &mut Vec<Statement>,
    source: &mut SourceInstruction,
) -> Operand {
    let span = source.span(body.blocks());

    let ref_local = {
        let place = match operand {
            Operand::Copy(place) | Operand::Move(place) => place,
            Operand::Constant(_) => unreachable!(),
        };
        let rvalue = Rvalue::AddressOf(Mutability::Not, place.clone());
        let ret_ty = rvalue.ty(body.locals()).unwrap();
        let result = body.new_local(ret_ty, span, Mutability::Not);
        let stmt = Statement { kind: StatementKind::Assign(Place::from(result), rvalue), span };
        statements.push(stmt);
        result
    };

    Operand::Copy(Place { local: ref_local, projection: vec![] })
}
