use stable_mir::mir::{
    BorrowKind, Local, LocalDecl, Mutability, Operand, ProjectionElem, Rvalue,
    Statement, StatementKind, Place,
};
use stable_mir::ty::{RigidTy, Ty, TyKind};

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
    actions: Vec<Action>,
    /// The locals, required to ensure that the references
    /// and pointers are picked appropriately.
    locals: &'locals [LocalDecl],
}

impl<'locals> CollectActions<'locals> {
    /// Initialize the struct using the given locals
    pub fn new(locals: &'locals [LocalDecl]) -> Self {
        CollectActions { actions: Vec::new(), locals }
    }

    /// Finalize the code actions to be taken
    pub fn finalize(self) -> Vec<Action> {
        self.actions
    }

    /// Collect the actions for assigning the lvalue
    /// to the dereferenced rvalue
    fn visit_assign_reference_dereference(&mut self, lvalue: Local, rvalue: Local) {
        match self.locals[rvalue].ty.kind() {
            TyKind::RigidTy(RigidTy::Ref(_, ty, _)) | TyKind::RigidTy(RigidTy::RawPtr(ty, _)) => {
                // reborrow
                self.actions.push(Action::NewMutRefFromRaw {
                    lvalue,
                    rvalue,
                    ty,
                });
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
                let rvalue = from.local.clone();
                self.actions.push(Action::NewStackReference { lvalue, rvalue });
            },
            [ProjectionElem::Deref] => {
                // Reborrow
                // x : &mut T = &*(y : *mut T OR &mut T)
                let lvalue = to; // Copy to avoid borrow
                let rvalue = from.local; // Copy to avoid borrow
                self.visit_assign_reference_dereference(lvalue, rvalue);
            }
            _ => { /* not yet handled */ }
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
                        self.actions.push(Action::NewMutRawFromRef {
                            lvalue,
                            rvalue,
                            ty,
                        });
                    }
                    _ => {
                        panic!("Dereference of rvalue case not yet handled for raw pointers {:?}", from);
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
            },
            [ProjectionElem::Deref] => {
                // Dereference -- instrument stack check
            },
            _ => {
                // Field access -- not yet handled.
                return;
            }
        };
        match self.locals[place.local].ty.kind() {
            TyKind::RigidTy(RigidTy::Ref(_, ty, _)) => {
                self.actions.push(Action::StackUpdateReference { place: place.local, ty });
                self.actions.push(Action::StackCheck);
            },
            TyKind::RigidTy(RigidTy::RawPtr(ty, _)) => {
                self.actions.push(Action::StackUpdatePointer { place: place.local, ty });
                self.actions.push(Action::StackCheck);
            },
            _ => {},
        }
    }

    /// Collect the actions for the places of an Rvalue
    fn visit_rvalue_places(&mut self, rvalue: &Rvalue) {
        match rvalue {
            Rvalue::AddressOf(_, place) => {
                self.visit_place(place);
            },
            Rvalue::Ref(_, _, place) => {
                self.visit_place(place);
            }
            // The rest are not yet handled
            Rvalue::Aggregate(_, _) => {},
            Rvalue::BinaryOp(_, _, _) => {},
            Rvalue::Cast(_, _, _) => {},
            Rvalue::CheckedBinaryOp(_, _, _) => {},
            Rvalue::CopyForDeref(_) => {},
            Rvalue::Discriminant(_) => {},
            Rvalue::Len(_) => {},
            Rvalue::Repeat(_, _) => {},
            Rvalue::ShallowInitBox(_, _) => {},
            Rvalue::ThreadLocalRef(_) => {},
            Rvalue::NullaryOp(_, _) => {},
            Rvalue::UnaryOp(_, _) => {},
            Rvalue::Use(_) => {},
        }
    }

    /// Collect the actions for the places of a statement
    fn visit_statement_places(&mut self, stmt: &Statement) {
        match &stmt.kind {
            StatementKind::Assign(place, rvalue) => {
                self.visit_rvalue_places(rvalue);
                self.visit_place(place);
            }
            StatementKind::FakeRead(_, _) => {},
            StatementKind::SetDiscriminant { .. } => {},
            StatementKind::Deinit(_) => {},
            StatementKind::StorageLive(_) => {},
            StatementKind::StorageDead(_) => {},
            StatementKind::Retag(_, _) => {},
            StatementKind::PlaceMention(_) => {},
            StatementKind::AscribeUserType { .. } => {},
            StatementKind::Coverage(_) => {},
            StatementKind::Intrinsic(_) => {},
            StatementKind::ConstEvalCounter => {},
            StatementKind::Nop => {},
        }
    }

    /// Collect the actions for the places of the statement,
    /// then find assignments of pointer values to lvalues
    /// and collect updates to the stacked borrows state
    /// accordingly
    pub fn visit_statement(&mut self, stmt: &Statement) {
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
            // The following are not yet handled, however, no info is printed
            // to avoid blowups:
            StatementKind::Retag(_, _) => {}
            StatementKind::FakeRead(_, _) => {}
            StatementKind::SetDiscriminant { .. } => {}
            StatementKind::Deinit(_) => {}
            StatementKind::StorageLive(_) => {}
            StatementKind::StorageDead(_) => {}
            StatementKind::PlaceMention(_) => {}
            StatementKind::AscribeUserType { .. } => {}
            StatementKind::Coverage(_) => {}
            StatementKind::Intrinsic(_) => {}
            StatementKind::ConstEvalCounter => {}
            StatementKind::Nop => {}
        }
    }
}
