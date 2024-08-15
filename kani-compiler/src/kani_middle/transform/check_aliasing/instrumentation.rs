use std::collections::HashMap;
use rustc_middle::ty::TyCtxt;
use stable_mir::mir::{BorrowKind, Local, Mutability, Place, ProjectionElem, Rvalue, Statement, StatementKind};
use stable_mir::ty::{GenericArgKind, Ty, Span, TyKind, RigidTy};
use super::{MirError, CachedBodyMutator, Cache, Signature, MutatorIndex, Instruction, MutatorIndexStatus};

pub struct InstrumentationData<'tcx, 'cache> {
    tcx: TyCtxt<'tcx>,
    cache: &'cache mut Cache,
    meta_stack: HashMap<Local, Local>,
    body: CachedBodyMutator,
}

impl<'tcx, 'cache> InstrumentationData<'tcx, 'cache> {
    pub fn new(tcx: TyCtxt<'tcx>, cache: &'cache mut Cache, body: CachedBodyMutator) -> Self {
        let meta_stack = HashMap::new();
        InstrumentationData { tcx, cache, meta_stack, body }
    }

    /// Assign lvalue to the address of rvalue with the given span.
    pub fn assign_ptr(body: &mut CachedBodyMutator, lvalue: Local, rvalue: Local, span: Span) {
        let lvalue = Place::from(lvalue);
        let rvalue = Rvalue::AddressOf(Mutability::Not, Place::from(rvalue));
        let kind = StatementKind::Assign(lvalue, rvalue);
        body.insert_statement(Statement { kind, span });
    }

    /// Instrument the code with a call to initialize the monitors.
    pub fn instrument_initialize(&mut self) -> Result<(), MirError> {
        let instance =
            self.cache.register(&self.tcx, Signature::new("KaniInitializeSState", &[]))?;
        let body = &mut self.body;
        body.call(instance, [].to_vec(), body.unit());
        Ok(())
    }

    /// For some local, say let x: T;
    /// instrument it with the functions that initialize the stack:
    /// let ptr_x: *const T = &raw const x;
    /// initialize_local(ptr_x);
    pub fn instrument_local(&mut self, local: usize) -> Result<(), MirError> {
        let ty = self.body.local(local).ty;
        let ptr_ty = Ty::new_ptr(ty, Mutability::Not);
        let span = self.body.span().clone();
        let body = &mut self.body;
        let local_ptr =
            self.meta_stack.entry(local).or_insert_with(|| body.new_local(ptr_ty, Mutability::Not));
        Self::assign_ptr(body, *local_ptr, local, span);
        let instance = self.cache.register(
            &self.tcx,
            Signature::new("KaniInitializeLocal",
            &[GenericArgKind::Type(ty)]))?;
        body.call(instance, [*local_ptr].to_vec(), body.unit());
        Ok(())
    }

    /// Instrument a stack reference of the form
    /// lvalue = &rvalue
    /// with an update to the stacked borrows state,
    /// at the code index idx.
    pub fn instrument_new_stack_reference(
        &mut self,
        idx: &MutatorIndex,
        lvalue: Local,
        rvalue: Local,
    ) -> Result<(), MirError> {
        // Initialize the constants
        let ty = self.body.local(rvalue).ty;
        let lvalue_ref = self.meta_stack.get(&lvalue).unwrap();
        let rvalue_ref = self.meta_stack.get(&rvalue).unwrap();
        let instance = self.cache.register(
            &self.tcx,
            Signature::new("KaniNewMutRefFromValue", &[GenericArgKind::Type(ty)]),
        )?;
        self.body.call(instance, vec![*lvalue_ref, *rvalue_ref], self.body.unit());
        self.body.split(idx);
        Ok(())
    }

    /// Instrument with stack violated / not violated
    pub fn instrument_stack_check(&mut self, idx: &MutatorIndex) -> Result<(), MirError> {
        // Initialize the constants
        let instance = self.cache.register(
            &self.tcx,
            Signature::new("KaniStackValid", &[])
        )?;
        self.body.call(instance, vec![], self.body.valid());
        let msg = format!("Stacked borrows aliasing model violated at {:?}:{:?}", idx.span().get_filename(), idx.span().get_lines());
        let instance = self.cache.register_assert(&self.tcx)?;
        self.body.assert(instance, self.body.valid(), msg, idx.span());
        Ok(())
    }

    /// Instrument a validity assertion on the stacked borrows state
    /// at idx for (place: &mut T).
    pub fn instrument_stack_update_ref(
        &mut self,
        place: Local,
        ty: Ty,
    ) -> Result<(), MirError> {
        // Initialize the constants
        let place_ref = self.meta_stack.get(&place).unwrap();
        let instance = self.cache.register(
            &self.tcx,
            Signature::new("KaniStackCheckRef", &[GenericArgKind::Type(ty)]),
        )?;
        self.body.call(instance, vec![*place_ref], self.body.unit());
        Ok(())
    }

    /// Instrument a validity assertion on the stacked borrows state
    /// at idx for (place: *const T).
    pub fn instrument_stack_update_ptr(
        &mut self,
        place: Local,
        ty: Ty,
    ) -> Result<(), MirError> {
        // Initialize the constants
        let place_ref = self.meta_stack.get(&place).unwrap();
        let instance = self.cache.register(
            &self.tcx,
            Signature::new("KaniStackCheckPtr", &[GenericArgKind::Type(ty)]),
        )?;
        self.body.call(instance, vec![*place_ref], self.body.unit());
        Ok(())
    }

    /// Instrument code of the form
    /// created = &mut *(raw: const *T).
    pub fn instrument_new_mut_ref_from_raw(
        &mut self,
        idx: &MutatorIndex,
        created: Local,
        raw: Local,
    ) -> Result<(), MirError> {
        // Initialize the constants
        let ty = self.body.local(created).ty;
        let created_ref = self.meta_stack.get(&created).unwrap();
        let reference_ref = self.meta_stack.get(&raw).unwrap();
        let instance = self.cache.register(
            &self.tcx,
            Signature::new("KaniNewMutRefFromRaw", &[GenericArgKind::Type(ty)]),
        )?;
        self.body.call(instance, vec![*created_ref, *reference_ref], self.body.unit());
        self.body.split(idx);
        Ok(())
    }

    /// Instrument code of the form
    /// created = (ref: &mut T) as *mut T
    pub fn instrument_new_mut_raw_from_ref(
        &mut self,
        idx: &MutatorIndex,
        created: Local,
        reference: Local,
    ) -> Result<(), MirError> {
        // Initialize the constants
        let ty = self.body.local(created).ty;
        let created_ref = self.meta_stack.get(&created).unwrap();
        let reference_ref = self.meta_stack.get(&reference).unwrap();
        let instance = self.cache.register(
            &self.tcx,
            Signature::new("KaniNewMutRawFromRef", &[GenericArgKind::Type(ty)]),
        )?;
        self.body.call(instance, vec![*created_ref, *reference_ref], self.body.unit());
        self.body.split(idx);
        Ok(())
    }

    /// Instrument at the code index idx with the appropriate updates
    /// to the stacked borrows state and with assertions for the validity
    /// of that state.
    pub fn instrument_index(&mut self, idx: &MutatorIndex) -> Result<(), MirError> {
        match self.body.inspect(idx) {
            Instruction::Stmt(Statement { kind, .. }) => {
                match kind {
                    StatementKind::Assign(to, rvalue) => {
                        let to = to.clone();
                        match rvalue {
                            Rvalue::Ref(_, BorrowKind::Mut { .. }, from) => {
                                match from.projection[..] {
                                    [] => {
                                        // Direct reference to stack local
                                        // x = &y
                                        self.instrument_new_stack_reference(
                                            idx, to.local, from.local,
                                        )?;
                                    }
                                    [ProjectionElem::Deref] => {
                                        // Reborrow
                                        // x : &mut T = &*(y : *mut T)
                                        let from = from.local; // Copy to avoid borrow
                                        let to = to.local; // Copy to avoid borrow
                                        match self.body.local(to).ty.kind() {
                                            TyKind::RigidTy(RigidTy::Ref(_, _ty, _)) => {
                                                eprintln!(
                                                    "Reborrow from reference not yet handled"
                                                );
                                            }
                                            TyKind::RigidTy(RigidTy::RawPtr(ty, _)) => {
                                                self.instrument_stack_update_ref(from, ty)?;
                                                self.instrument_stack_check(idx)?;
                                                self.body.split(idx);
                                            }
                                            _ => {}
                                        }
                                    }
                                    _ => {
                                        eprintln!("Field projections not yet handled");
                                    }
                                }
                            }
                            Rvalue::AddressOf(Mutability::Mut, from) => {
                                match from.projection[..] {
                                    [] => {
                                        // x = &raw y
                                        eprintln!("addr of not yet handled");
                                    }
                                    [ProjectionElem::Deref] => {
                                        // x = &raw mut *(y: &mut T)
                                        let from = from.local; // Copy to avoid borrow
                                        let to = to.local; // Copy to avoid borrow
                                        match self.body.local(to).ty.kind() {
                                            TyKind::RigidTy(RigidTy::Ref(_, ty, _)) => {
                                                self.instrument_stack_update_ref(from, ty)?;
                                                self.instrument_stack_check(idx)?;
                                                self.body.split(idx);
                                                self.instrument_new_mut_raw_from_ref(
                                                    idx, to, from,
                                                )?;
                                            }
                                            TyKind::RigidTy(RigidTy::RawPtr(_ty, _)) => {
                                                eprintln!(
                                                    "Pointer to pointer casts not yet handled"
                                                );
                                            }
                                            _ => {}
                                        }
                                    }
                                    _ => {}
                                }
                            }
                            _ => {
                                eprintln!("Rvalue kind: {:?} not yet handled", rvalue);
                            }
                        }
                        match to.projection[..] {
                            [] => {
                                // Assignment directly to local
                                Ok(())
                            }
                            [ProjectionElem::Deref] => {
                                // *x = rvalue
                                let to = to.local;
                                println!("Self body local to is: {:?}", self.body.local(to));
                                match self.body.local(to).ty.kind() {
                                    TyKind::RigidTy(RigidTy::Ref(_, ty, _)) => {
                                        self.instrument_stack_update_ref(to, ty)?;
                                        self.instrument_stack_check(idx)?;
                                        self.body.split(idx);
                                    }
                                    TyKind::RigidTy(RigidTy::RawPtr(ty, _)) => {
                                        self.instrument_stack_update_ptr(to, ty)?;
                                        self.instrument_stack_check(idx)?;
                                        self.body.split(idx);
                                    }
                                    _ => {}
                                }
                                Ok(())
                            }
                            _ => {
                                eprintln!("Field assignment not yet handled");
                                Ok(())
                            }
                        }
                    }
                    // The following are not yet handled, however, no info is printed
                    // to avoid blowups:
                    StatementKind::Retag(_, _) => Ok(()),
                    StatementKind::FakeRead(_, _) => Ok(()),
                    StatementKind::SetDiscriminant { .. } => Ok(()),
                    StatementKind::Deinit(_) => Ok(()),
                    StatementKind::StorageLive(_) => Ok(()),
                    StatementKind::StorageDead(_) => Ok(()),
                    StatementKind::PlaceMention(_) => Ok(()),
                    StatementKind::AscribeUserType { .. } => Ok(()),
                    StatementKind::Coverage(_) => Ok(()),
                    StatementKind::Intrinsic(_) => Ok(()),
                    StatementKind::ConstEvalCounter => Ok(()),
                    StatementKind::Nop => Ok(()),
                }
            }
            Instruction::Term(_) => Ok(()),
        }
    }

    /// Instrument each of the locals collected into values with
    /// initialization data.
    pub fn instrument_locals(&mut self) -> Result<(), MirError> {
        self.instrument_initialize()?;
        for local in (self.body.arg_locals().len() + 1)..self.body.locals().len() {
            self.instrument_local(local)?
        }
        Ok(())
    }

    /// Instrument all of the instructions and terminators in the function body
    /// with appropriate updates to the stacked borrows state
    /// and with validity assertions on the stacked borrows state.
    pub fn instrument_instructions(&mut self) -> Result<(), MirError> {
        let mut index = self.body.new_index();
        let mut status = MutatorIndexStatus::Remaining;
        while status == MutatorIndexStatus::Remaining {
            self.instrument_index(&index)?;
            status = self.body.decrement_index(&mut index);
        }
        Ok(())
    }

    pub fn finalize_prologue(&mut self) {
        self.body.finalize_prologue();
    }

    pub fn finalize(self) -> stable_mir::mir::Body {
        self.body.finalize()
    }
}
