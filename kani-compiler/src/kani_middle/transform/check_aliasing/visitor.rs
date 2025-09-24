// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! This module contains stacked borrows "actions,"
//! or updates to the stacked borrows state, as well as
//! methods that collect the actions that need to be applied from the
//! statements of the code.

use stable_mir::mir::visit::Location;
use stable_mir::mir::{
    BorrowKind, Local, LocalDecl, MirVisitor, Mutability, Operand, Place, ProjectionElem, Rvalue,
    Statement, StatementKind, Terminator,
};
use stable_mir::ty::{RigidTy, Ty, TyKind};

use crate::kani_middle::transform::body::SourceInstruction;

/// Update action to the stacked borrows state
#[derive(Debug)]
pub enum Action {
    StackCheck,
    NewStackReference { lvalue: Local, rvalue: usize },
    StackUpdateReference { place: usize, ty: Ty },
    NewMutRefFromRaw { lvalue: usize, rvalue: usize, ty: Ty },
    StackUpdatePointer { place: usize, ty: Ty },
    NewMutRawFromRef { lvalue: usize, rvalue: usize, ty: Ty },
}

/// The actions of a statement
pub struct CollectActions<'locals> {
    /// The source instruction currently being visited
    source: SourceInstruction,
    /// The current actions collected
    collected: Vec<Action>,
    /// The code actions to insert behind the given
    /// instruction
    actions: Vec<(SourceInstruction, Vec<Action>)>,
    /// The locals, required to ensure that the references
    /// and pointers are picked appropriately.
    locals: &'locals [LocalDecl],
}

impl<'locals> CollectActions<'locals> {
    /// Initialize the struct using the given locals
    pub fn new(locals: &'locals [LocalDecl]) -> Self {
        CollectActions {
            source: SourceInstruction::Statement { idx: 0, bb: 0 },
            collected: Vec::new(),
            actions: Vec::new(),
            locals,
        }
    }

    pub fn finalize(self) -> Vec<(SourceInstruction, Vec<Action>)> {
        self.actions
    }

    /// Collect the actions for assigning the lvalue
    /// to the dereferenced rvalue
    fn visit_assign_reference_dereference(&mut self, lvalue: Local, rvalue: Local) {
        match self.locals[rvalue].ty.kind() {
            TyKind::RigidTy(RigidTy::Ref(_, ty, _)) | TyKind::RigidTy(RigidTy::RawPtr(ty, _)) => {
                self.collected.push(Action::NewMutRefFromRaw { lvalue, rvalue, ty });
            }
            _ => {}
        }
    }

    /// Collect the actions for assigning the reference in
    /// "place" to the local at "to".
    fn visit_assign_reference(&mut self, to: Local, from: Place) {
        match from.projection[..] {
            [] => {
                // Direct reference to stack local
                // x = &y;
                let lvalue = to;
                let rvalue = from.local;
                self.collected.push(Action::NewStackReference { lvalue, rvalue });
            }
            [ProjectionElem::Deref] => {
                // Reborrow
                // x : &mut T = &*(y : *mut T OR &mut T)
                let lvalue = to; // Copy to avoid borrow
                let rvalue = from.local; // Copy to avoid borrow
                self.visit_assign_reference_dereference(lvalue, rvalue);
            }
            _ => {
                eprintln!("not yet handled: assignment to reference {:?}", from)
            }
        }
    }

    /// Collect the actions for assigning the data at
    /// from to the local at to.
    fn visit_assign_pointer(&mut self, to: Local, from: Place) {
        match from.projection[..] {
            [] => {
                // x = &raw y
                panic!("Addr of not yet handled");
            }
            [ProjectionElem::Deref] => {
                // x = &raw mut *(y: &mut T OR *mut T)
                let rvalue = from.local; // Copy to avoid borrow
                let lvalue = to;
                match self.locals[rvalue].ty.kind() {
                    TyKind::RigidTy(RigidTy::Ref(_, ty, _)) => {
                        self.collected.push(Action::NewMutRawFromRef { lvalue, rvalue, ty });
                    }
                    _ => {
                        panic!(
                            "Dereference of rvalue case not yet handled for raw pointers {:?}",
                            from
                        );
                    }
                }
            }
            _ => {}
        }
    }

    /// Collect the actions for a Place,
    /// incurring a stack check in the case of a
    /// dereference of a pointer or reference
    fn visit_place(&mut self, place: &Place) {
        match &place.projection[..] {
            [] => {
                // Direct usage -- no update needed
                return;
            }
            [ProjectionElem::Deref] => {
                // Dereference -- instrument stack check
            }
            _ => {
                // Field access -- not yet handled.
                return;
            }
        };
        if self.locals[place.local].ty.kind().is_ref() {
            let ty = place.ty(self.locals).unwrap();
            self.collected.push(Action::StackUpdateReference { place: place.local, ty });
            self.collected.push(Action::StackCheck);
        }
        if self.locals[place.local].ty.kind().is_raw_ptr() {
            let ty = place.ty(self.locals).unwrap();
            self.collected.push(Action::StackUpdatePointer { place: place.local, ty });
            self.collected.push(Action::StackCheck);
        }
    }

    /// Collect the actions for the places of an Rvalue
    fn visit_rvalue_places(&mut self, rvalue: &Rvalue) {
        match rvalue {
            Rvalue::AddressOf(_, place) => {
                self.visit_place(place);
            }
            Rvalue::Ref(_, _, place) => {
                self.visit_place(place);
            }
            // The rest are not yet handled
            _ => {
                eprintln!("Not yet handled: {:?}", rvalue);
            }
        }
    }

    /// Collect the actions for the places of a statement
    fn visit_statement_places(&mut self, stmt: &Statement) {
        match &stmt.kind {
            StatementKind::Assign(place, rvalue) => {
                self.visit_rvalue_places(rvalue);
                self.visit_place(place);
            }
            _ => {
                eprintln!("Not yet handled: {:?}", stmt);
            }
        }
    }

    /// Collect the actions for the places of the statement,
    /// then find assignments of pointer values to lvalues
    /// and collect updates to the stacked borrows state
    /// accordingly.
    /// This is performed in a different order than Mir Visitor's
    /// visit statement, and so we call it from the visitor.
    fn visit_statement_internal(&mut self, stmt: &Statement) {
        self.visit_statement_places(stmt);
        match &stmt.kind {
            StatementKind::Assign(to, rvalue) => {
                match rvalue {
                    Rvalue::Ref(_, BorrowKind::Mut { .. }, from) => {
                        self.visit_assign_reference(to.local, from.clone());
                    }
                    Rvalue::AddressOf(Mutability::Mut, from) => {
                        self.visit_assign_pointer(to.local, from.clone());
                    }
                    Rvalue::Use(Operand::Constant(_)) => {
                        // Do nothing for the constants case
                    }
                    Rvalue::Use(Operand::Copy(_)) => {
                        eprintln!("Copy not yet handled");
                        // Do nothing for the constants case
                    }
                    Rvalue::Use(Operand::Move(_)) => {
                        eprintln!("Move not yet handled");
                        // Do nothing for the constants case
                    }
                    Rvalue::BinaryOp(_, _, _) => {
                        eprintln!("Binary op not yet handled");
                    }
                    Rvalue::CheckedBinaryOp(_, _, _) => {
                        eprintln!("Checked binary op not yet handled");
                    }
                    _ => {
                        panic!("Rvalue kind: {:?} not yet handled", rvalue);
                    }
                }
            }
            _ => {
                eprintln!("Not yet handled, {:?}", stmt);
            }
        }
    }
}

impl<'locals> MirVisitor for CollectActions<'locals> {
    /// Visit the statement stmt.
    /// Associate the actions collected so far with the
    /// current source index, then collect the actions
    /// of the statement and increase the source index
    fn visit_statement(&mut self, stmt: &Statement, _location: Location) {
        let collected = std::mem::take(&mut self.collected);
        self.actions.push((self.source, collected));
        self.visit_statement_internal(stmt);
        self.source = match self.source {
            SourceInstruction::Statement { idx, bb } => {
                SourceInstruction::Statement { idx: idx + 1, bb }
            }
            _ => {
                unreachable!("Statements follow the first instruction or a terminator.")
            }
        };
    }

    /// Visit the terminator.
    /// Associate the actions collected so far with the
    /// current source index, then increase the source index
    /// to the next basic block
    fn visit_terminator(&mut self, _term: &Terminator, _location: Location) {
        let collected = std::mem::take(&mut self.collected);
        let bb = self.source.bb();
        self.actions.push((SourceInstruction::Terminator { bb }, collected));
        self.source = SourceInstruction::Statement { idx: 0, bb: bb + 1 };
    }
}
