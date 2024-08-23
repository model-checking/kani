use std::collections::HashMap;
use rustc_middle::ty::TyCtxt;
use stable_mir::mir::{Body, Local, Mutability, Operand, Place, Rvalue, Terminator, TerminatorKind, UnwindAction};
use stable_mir::ty::{GenericArgKind, Ty, Span};
use crate::kani_middle::transform::body::{CheckType, InsertPosition};

use super::{Action, Cache, CollectActions, MirError, MirInstance, Signature};
use super::super::body::{MutableBody, SourceInstruction};

type Result<T> = std::result::Result<T, MirError>;

pub struct InstrumentationData<'tcx, 'cache> {
    /// Compilation context, used to fetch resolved generic functions
    tcx: TyCtxt<'tcx>,
    /// Cache of resolved generic functions,
    /// potentially populated by previous passes
    cache: &'cache mut Cache,
    /// Map associating each local with a local storing
    /// its address on the stack, which is used to associate
    /// the metadata.
    meta_stack: HashMap<Local, Local>,
    /// The count of the number of locals in the original
    /// body
    local_count: usize,
    /// A local storing the unit value
    unit: Local,
    /// A local storing whether the stack is still in a valid
    /// state
    valid: Local,
    /// A map associating resolved generic functions with
    /// locals in the body that can be used to call them
    fn_pointers: HashMap<MirInstance, Local>,
    /// The span of the body
    span: Span,
    /// The minimum processed instruction.
    /// All instructions before this one belong to the original
    /// source code, and have not yet been analyzed.
    min_processed: SourceInstruction,
    /// The index after which "ghost" instrumentation code
    /// may be added.
    ghost_index: SourceInstruction,
    /// The body being instrumented
    body: MutableBody,
}

impl<'tcx, 'cache> InstrumentationData<'tcx, 'cache> {
    /// Move bb0 to the end to start instrumentation there.
    fn prepare_body(body: Body) -> MutableBody {
        let mut body = MutableBody::from(body);
        let span;
        let mut source;
        match body.blocks()[0].statements.len() {
            0 => {
                source = SourceInstruction::Terminator { bb: 0 };
                span = body.blocks()[0].terminator.span;
            },
            _ => {
                source = SourceInstruction::Statement { idx: 0, bb: 0 };
                span = body.blocks()[0].terminator.span;
            }
        }
        let kind = TerminatorKind::Goto { target: body.blocks().len() };
        let terminator = Terminator { kind, span };
        body.insert_terminator(&mut source, InsertPosition::Before, terminator);
        body
    }

    /// Using a (potentially) pre-populated cache of resolved generic
    /// functions, and the StableMir body "body", initialize the instrumentation
    /// pass data.
    pub fn new(tcx: TyCtxt<'tcx>, cache: &'cache mut Cache, body: Body) -> Self {
        let span = body.span;
        let mut body = Self::prepare_body(body);
        let meta_stack = HashMap::new();
        let local_count = body.locals().len();
        let fn_pointers = HashMap::new();
        let unit = body.new_local(Ty::new_tuple(&[]), span, Mutability::Not);
        let valid = body.new_local(Ty::from_rigid_kind(stable_mir::ty::RigidTy::Bool), span, Mutability::Mut);
        let bb = body.blocks().len() - 1;
        let min_processed = match body.blocks()[bb].statements.len() {
            0 => SourceInstruction::Terminator { bb },
            n => SourceInstruction::Statement { idx: n - 1, bb }
        };
        let ghost_index = SourceInstruction::Terminator { bb: 0 };
        InstrumentationData { tcx, cache, meta_stack, local_count, unit, valid, fn_pointers, span, min_processed, ghost_index, body }
    }

    /// Register the function described by the diagnostic
    /// and generic arguments in "Signature".
    fn register_fn(&mut self, callee: Signature) -> Result<Local> {
        let cache = &mut self.cache;
        let tcx = &self.tcx;
        let fn_pointers = &mut self.fn_pointers;
        let body = &mut self.body;
        let span = self.span.clone();
        let instance = cache.register(tcx, callee)?;
        let func_local = fn_pointers.entry(*instance)
            .or_insert_with(|| body.new_local(instance.ty(), span, Mutability::Not));
        Ok(*func_local)
    }

    /// Call at source and insert position, using the arguments
    /// in args and returning into "dest".
    /// This differs from Mutable Body's call in that the
    /// function name is cached.
    pub fn call(&mut self,
                callee: Signature,
                args: Vec<Local>,
                dest: Local) -> Result<()> {
        let func_local = self.register_fn(callee)?;
        let new_bb = self.body.blocks().len();
        let span = self.body.blocks()[self.min_processed.bb()].terminator.span;
        let callee_op = Operand::Copy(Place::from(func_local));
        let args = args
            .into_iter()
            .map(|v| Operand::Copy(Place { local: v, projection: vec![] }))
            .collect();
        let destination = Place::from(dest);
        let kind = TerminatorKind::Call {
            func: callee_op,
            args,
            destination,
            target: Some(new_bb),
            unwind: UnwindAction::Terminate,
        };
        let terminator = Terminator { kind, span };
        let source = &mut self.ghost_index;
        self.body.insert_terminator(source, InsertPosition::After, terminator);
        Ok(())
    }

    /// Instrument an assignment to a local
    pub fn assign_pointer(&mut self, lvalue: Local, rvalue: Local) {
        let source = &mut self.ghost_index;
        let position = InsertPosition::After;
        self.body.assign_to(Place::from(lvalue), Rvalue::AddressOf(Mutability::Not, Place::from(rvalue)), source, position);
    }

    /// For some local, say let x: T;
    /// instrument it with the functions that initialize the stack:
    /// let ptr_x: *const T = &raw const x;
    /// initialize_local(ptr_x);
    pub fn instrument_local(&mut self, local: Local) -> Result<()> {
        let ty = self.body.locals()[local].ty;
        let ptr_ty = Ty::new_ptr(ty, Mutability::Not);
        let span = self.span.clone();
        let body = &mut self.body;
        let local_ptr =
            self.meta_stack.entry(local).or_insert_with(|| body.new_local(ptr_ty, span, Mutability::Not));
        let local_ptr = *local_ptr;
        self.assign_pointer(local_ptr, local);
        self.call(Signature::new("KaniInitializeLocal", &[GenericArgKind::Type(ty)]), vec![local_ptr], self.unit)?;
        Ok(())
    }

    /// Split at the minimum processed instruction,
    /// allowing instrumentation of ghost code following
    /// that source instruction.
    pub fn process_instruction(&mut self) {
        // If the instruction under processing is a terminator,
        // special care is needed; it is impossible to instrument
        // "after" a terminator with no target.
        // Therefore we handle the terminators manually,
        // inserting the terminator into the ghost code,
        // then inserting a jump to that terminator.
        // These will be called the "enter ghost,"
        // "execute ghost", and "execute terminator"
        // terminators
        match self.min_processed {
            SourceInstruction::Terminator { bb } => {
                let original = self.body.blocks()[bb].terminator.clone();
                let original_span = self.body.blocks()[bb].terminator.span.clone();
                let span = self.span;
                let kind = TerminatorKind::Goto { target: 0 };
                let terminator = Terminator { kind, span };
                let source = &mut self.min_processed.clone();
                let enter_ghost_block = source.clone();
                let body = &mut self.body;
                body.replace_terminator(source, terminator.clone()); // replace terminator so you can instrument "after" it
                body.insert_terminator(source, InsertPosition::After, terminator.clone());
                let execute_terminator_block = source.clone();
                body.insert_terminator(source, InsertPosition::After, terminator.clone());
                let execute_ghost_block = source.clone();

                // Instrument enter ghost:
                let span = original_span;
                let target = execute_ghost_block.bb();
                let kind = TerminatorKind::Goto { target };
                let terminator = Terminator { kind, span };
                body.replace_terminator(&enter_ghost_block, terminator);
                // Instrument execute ghost:
                let target = execute_terminator_block.bb();
                let kind = TerminatorKind::Goto { target };
                let terminator = Terminator { kind, span };
                body.replace_terminator(&execute_ghost_block, terminator);
                // Instrument execute terminator
                body.replace_terminator(&execute_terminator_block, original);

                self.ghost_index = execute_ghost_block;
            },
            SourceInstruction::Statement { idx, bb } => {
                // In this case it is simple, merely goto the ghost code
                // immdediately.
                let span = self.body.blocks()[bb].statements[idx].span;
                let target = self.body.blocks().len();
                let kind = TerminatorKind::Goto { target };
                let terminator = Terminator { kind, span };
                let min_processed = &mut self.min_processed.clone();
                self.body.insert_terminator(min_processed, InsertPosition::After, terminator);
                self.ghost_index = *min_processed;
            }
        }
    }

    /// Instrument a stack reference of the fo
    /// lvalue = &rvalue
    /// with an update to the stacked borrows state,
    /// at the code index source.
    pub fn instrument_new_stack_reference(
        &mut self,
        lvalue: Local,
        rvalue: Local,
    ) -> Result<()> {
        // Initialize the constants
        let ty = self.body.locals()[rvalue].ty;
        let lvalue_ref = self.meta_stack.get(&lvalue).unwrap();
        let rvalue_ref = self.meta_stack.get(&rvalue).unwrap();
        self.call(Signature::new("KaniNewMutRefFromValue", &[GenericArgKind::Type(ty)]), vec![*lvalue_ref, *rvalue_ref], self.unit)?;
        Ok(())
    }

    /// Instrument with stack violated / not violated
    pub fn instrument_stack_check(&mut self) -> Result<()> {
        let span = match self.min_processed {
            SourceInstruction::Statement { idx, bb } => self.body.blocks()[bb].statements[idx].span,
            SourceInstruction::Terminator { bb } => self.body.blocks()[bb].terminator.span,
        };
        self.call(Signature::new("KaniStackValid", &[]), vec![], self.valid)?;
        let msg = format!("Stacked borrows aliasing model violated at {:?}:{:?}", span.get_filename(), span.get_lines());
        let check_fn = self.cache.register_assert(&self.tcx)?;
        let check_type = &CheckType::Assert(*check_fn);
        self.body.insert_check(self.tcx, check_type, &mut self.ghost_index, InsertPosition::After, self.valid, &msg);
        Ok(())
    }

    /// Instrument a validity assertion on the stacked borrows state
    /// at idx for (place: &mut T).
    pub fn instrument_stack_update_ref(
        &mut self,
        place: Local,
        ty: Ty,
    ) -> Result<()> {
        // Initialize the constants
        let place_ref = self.meta_stack.get(&place).unwrap();
        self.call(Signature::new("KaniStackCheckRef", &[GenericArgKind::Type(ty)]), vec![*place_ref], self.unit)?;
        Ok(())
    }

    /// Instrument a validity assertion on the stacked borrows state
    /// at idx for (place: *const T).
    pub fn instrument_stack_update_ptr(
        &mut self,
        place: Local,
        ty: Ty,
    ) -> Result<()> {
        // Initialize the constants
        let place_ref = self.meta_stack.get(&place).unwrap();
        self.call(Signature::new("KaniStackCheckPtr", &[GenericArgKind::Type(ty)]), vec![*place_ref], self.unit)?;
        Ok(())
    }

    /// Instrument code of the form
    /// created = &mut *(raw: const *T).
    pub fn instrument_new_mut_ref_from_raw(
        &mut self,
        created: Local,
        raw: Local,
        ty: Ty,
    ) -> Result<()> {
        // Initialize the constants
        let created_ref = self.meta_stack.get(&created).unwrap();
        let reference_ref = self.meta_stack.get(&raw).unwrap();
        self.call(Signature::new("KaniNewMutRefFromRaw", &[GenericArgKind::Type(ty)]), vec![*created_ref, *reference_ref], self.unit)?;
        Ok(())
    }

    /// Instrument code of the form
    /// created = (ref: &mut T) as *mut T
    pub fn instrument_new_mut_raw_from_ref(
        &mut self,
        created: Local,
        reference: Local,
        ty: Ty,
    ) -> Result<()> {
        // Initialize the constants
        let created_ref = self.meta_stack.get(&created).unwrap();
        let reference_ref = self.meta_stack.get(&reference).unwrap();
        self.call(Signature::new("KaniNewMutRawFromRef", &[GenericArgKind::Type(ty)]), vec![*created_ref, *reference_ref], self.unit)?;
        Ok(())
    }

    /// Instrument each of the locals collected into values with
    /// initialization data.
    pub fn instrument_locals(&mut self) -> Result<()> {
        for local in (self.body.arg_count() + 1)..self.local_count {
            self.instrument_local(local)?
        }
        Ok(())
    }

    /// Fetch the actions to be instrumented at the current instruction.
    pub fn instruction_actions(&self) -> Vec<Action> {
        let mut visitor = CollectActions::new(self.body.locals());
        match self.min_processed {
            SourceInstruction::Terminator { .. } => { /* not yet handled */ },
            SourceInstruction::Statement { idx, bb } => {
                visitor.visit_statement(&self.body.blocks()[bb].statements[idx]);
            }
        }
        visitor.finalize()
    }

    /// Instrument the action given in "action" with the appropriate
    /// update to the stacked borrows state.
    fn instrument_action(&mut self, action: Action) -> Result<()> {
        match action {
            Action::StackCheck => self.instrument_stack_check(),
            Action::NewStackReference { lvalue, rvalue } => self.instrument_new_stack_reference(lvalue, rvalue),
            Action::StackUpdateReference { place, ty } => self.instrument_stack_update_ref(place, ty),
            Action::NewMutRefFromRaw { lvalue, rvalue, ty } => self.instrument_new_mut_ref_from_raw(lvalue, rvalue, ty),
            Action::StackUpdatePointer { place, ty } => self.instrument_stack_update_ptr(place, ty),
            Action::NewMutRawFromRef { lvalue, rvalue, ty } => self.instrument_new_mut_raw_from_ref(lvalue, rvalue, ty),
        }
    }

    /// Instrument all of the instructions and terminators in the function body
    /// with appropriate updates to the stacked borrows state
    /// and with validity assertions on the stacked borrows state.
    pub fn instrument_instructions(&mut self) -> Result<()> {
        loop {
            let actions = self.instruction_actions();
            if actions.len() > 0 {
                eprintln!("Instrumenting actions:");
                self.process_instruction();
            }
            for action in actions {
                eprintln!("Action is: {:?}", action);
                self.instrument_action(action)?;
            }
            self.min_processed = match self.min_processed {
                SourceInstruction::Statement { idx: 0, bb: 0 } => { break; },
                SourceInstruction::Statement { idx: 0, bb } => SourceInstruction::Terminator { bb: bb - 1 },
                SourceInstruction::Statement { idx, bb } => SourceInstruction::Statement { idx: idx - 1, bb },
                SourceInstruction::Terminator { bb } if self.body.blocks()[bb].statements.len() > 0 =>
                    SourceInstruction::Statement { idx: self.body.blocks()[bb].statements.len() - 1, bb },
                SourceInstruction::Terminator { bb } if bb > 0 => SourceInstruction::Terminator { bb: bb - 1 },
                SourceInstruction::Terminator { .. } => { break; }
            }
        }
        Ok(())
    }

    /// Finalize the instrumentation of the body
    pub fn finalize(self) -> MutableBody {
        self.body
    }
}
