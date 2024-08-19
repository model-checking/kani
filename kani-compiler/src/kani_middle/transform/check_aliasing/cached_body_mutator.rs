use std::collections::HashMap;
use super::{MirInstance, BodyMutator};
use stable_mir::mir::{Body, ConstOperand, Local, LocalDecl, Mutability, Operand, Place, Statement};
use stable_mir::ty::{Ty, Span};
use super::{MutatorIndex, MutatorIndexStatus, Instruction};

/// Body mutator which wraps the BodyMutator
/// interface with a cache of the locals that
/// store function calls.
pub struct CachedBodyMutator {
    body: BodyMutator,
    unit: Local,
    valid: Local,
    cache: HashMap<MirInstance, Local>,
}

impl CachedBodyMutator {
    /// Create a new cached body mutator
    pub fn from(body: Body) -> Self {
        let mut body = BodyMutator::from(body);
        let unit = body.new_local(Ty::new_tuple(&[]), Mutability::Not);
        let valid = body.new_local(Ty::from_rigid_kind(stable_mir::ty::RigidTy::Bool), Mutability::Mut);
        body.insert_statement(Statement { kind: stable_mir::mir::StatementKind::Assign(Place::from(valid), stable_mir::mir::Rvalue::Use(Operand::Constant(ConstOperand { span: body.span(), user_ty: None, const_: stable_mir::ty::MirConst::from_bool(true) }))), span: body.span() });
        let cache = HashMap::new();
        CachedBodyMutator { body, unit, valid, cache }
    }

    /// Get the local at idx
    pub fn local(&self, idx: usize) -> &LocalDecl {
        self.body.local(idx)
    }

    /// Get a new local
    pub fn new_local(&mut self, ty: Ty, mutability: Mutability) -> Local {
        self.body.new_local(ty, mutability)
    }

    /// Locals in `self` that correspond to this function's arguments.
    pub fn arg_locals(&self) -> &[LocalDecl] {
        self.body.arg_locals()
    }

    /// Locals
    pub fn locals(&self) -> &[LocalDecl] {
        self.body.locals()
    }

    /// Insert a call to the function stored at local with the args
    /// stored at args
    pub fn call(&mut self, callee: &MirInstance, args: Vec<Local>, local: Local) {
        let func_local;
        {
            let cache = &mut self.cache;
            let body = &mut self.body;
            {
                func_local = cache
                    .entry(*callee)
                    .or_insert_with(|| body.new_local(callee.ty(), Mutability::Not));
            }
        }
        self.body.call(*func_local, args, local);
    }

    pub fn assert(&mut self, assert_fn: &MirInstance, cond: Local, message: String, span: Span) {
        let func_local;
        {
            let cache = &mut self.cache;
            let body = &mut self.body;
            {
                func_local = cache
                    .entry(*assert_fn)
                    .or_insert_with(|| body.new_local(assert_fn.ty(), Mutability::Not));
            }
        }
        self.body.assert(*func_local, self.unit, cond, message, span);
    }

    /// Finalize the prologue, initializing all of the locals
    pub fn finalize_prologue(&mut self) {
        self.body.finalize_prologue();
    }

    /// Insert a ghost statement
    pub fn insert_statement(&mut self, stmt: Statement) {
        self.body.insert_statement(stmt);
    }

    /// Get an index with which to iterate over the body
    pub fn new_index(&self) -> MutatorIndex {
        self.body.new_index()
    }

    /// Decrement the index
    pub fn decrement_index(&mut self, idx: &mut MutatorIndex) -> MutatorIndexStatus {
        self.body.decrement(idx)
    }

    /// Split at the index causing the ghost code to be called
    /// after that index
    pub fn split(&mut self, idx: &MutatorIndex) {
        self.body.split(idx);
    }

    /// Inspect the instruction at the index
    pub fn inspect(&self, idx: &MutatorIndex) -> Instruction {
        self.body.inspect(idx)
    }

    /// Finalize the body
    pub fn finalize(self) -> Body {
        self.body.finalize()
    }

    /// Get the span
    pub fn span(&self) -> Span {
        self.body.span()

    }

    /// Get the unit local
    pub fn unit(&self) -> Local {
        self.unit
    }

    /// Get the violated local
    pub fn valid(&self) -> Local {
        self.valid
    }
}
