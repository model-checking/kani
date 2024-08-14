// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
//! Implement a transformation pass that instruments the code to detect possible UB due to
//! the accesses to uninitialized memory.

use crate::args::ExtraChecks;
use crate::kani_middle::transform::{TransformPass, TransformationType};
use crate::kani_queries::QueryDb;
use rustc_middle::ty::TyCtxt;
use stable_mir::mir::mono::Instance;
use std::fmt::Debug;
use stable_mir::ty::{
    GenericArgKind, GenericArgs, RigidTy, Span, Ty, TyKind
};
use stable_mir::mir::{
    BasicBlockIdx, Body, Local, LocalDecl, Mutability, Place, TerminatorKind, UnwindAction
};
use stable_mir::Error;
use stable_mir::mir::{BasicBlock, BorrowKind, MirVisitor, Operand, ProjectionElem, Rvalue, Statement, StatementKind, Terminator, VarDebugInfo};
use std::collections::HashMap;
use tracing::trace;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FunctionSignature {
    name: String,
    args: Vec<GenericArgKind>,
}

impl FunctionSignature {
    pub fn new(name: &str, args: &[GenericArgKind]) -> FunctionSignature {
        FunctionSignature {
            name: name.to_string(),
            args: args.to_vec(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FunctionInstance {
    signature: FunctionSignature,
    instance: Instance,
}

impl FunctionInstance {
    pub fn new(signature: FunctionSignature, instance: Instance) -> FunctionInstance {
        FunctionInstance {
            signature,
            instance,
        }
    }
}

#[derive(Default, Debug)]
pub struct FunctionInstanceCache(Vec<FunctionInstance>);

/// Instrument the code with checks for uninitialized memory.
#[derive(Debug, Default)]
pub struct AliasingPass {
    cache: FunctionInstanceCache,
}

impl AliasingPass {
    pub fn new() -> AliasingPass {
        Default::default()
    }
}

struct InitializedPassState<'tcx, 'cache> {
    body: Body,
    tcx: TyCtxt<'tcx>,
    cache: &'cache mut FunctionInstanceCache,
}

impl<'tcx, 'cache> InitializedPassState<'tcx, 'cache> {
    fn new(body: Body, tcx: TyCtxt<'tcx>, cache: &'cache mut FunctionInstanceCache) -> Self {
        Self { body, tcx, cache }
    }

    fn collect_locals(self) -> LocalPassState<'tcx, 'cache> {
        let mut visitor = CollectLocalVisitor::new();
        visitor.visit_body(&self.body);
        LocalPassState { tcx: self.tcx, cache: self.cache, values: visitor.values, body: self.body }
    }
}

/// Functions containing any of the following in their
/// prefix or in their name will be ignored.
/// This allows skipping instrumenting functions that
/// are called by the instrumentation functions.
const ALIASING_BLACKLIST: &'static [&'static str] = &[
    "kani", // Skip kani functions
    "std::mem::size_of", // skip size_of::<T>
    "core::num", // Skip numerical ops (like .wrapping_add)
    "std::ptr", // Skip pointer manipulation functions
    "KaniInitializeSState",
    "KaniInitializeLocal",
    "KaniStackCheckPtr",
    "KaniStackCheckRef",
    "KaniNewMutRefFromValue",
    "KaniNewMutRawFromRef",
    "KaniNewMutRefFromRaw",
    "std::array",
    "std::ops",
    "core::panicking",
    "std::rt",
    "std::panic",
    "core::panic",
    "std::fmt",
    "std::iter",
    "core::ub_checks",
    "std::cmp",
    "core::slice",
    "std::mem",
    // This blacklist needs expansion.
];

// Currently, the above list of functions is too
// coarse-grained; because all kani functions
// are skipped, many std modules are skipped,
// and kani functions are skipped, this pass
// cannot be used to verify functions
// in those modules, despite the fact that
// only some of those functions in those modules
// are called by the instrumented code.

impl TransformPass for AliasingPass {
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
        args.ub_check.contains(&ExtraChecks::Aliasing)
    }

    fn transform(&mut self, tcx: TyCtxt, body: Body, instance: Instance) -> (bool, Body) {
        trace!(function=?instance.name(), "transform: aliasing pass");
        if ALIASING_BLACKLIST.iter().fold(false, |blacklisted, member| blacklisted || instance.name().contains(member)) {
            (false, body)
        } else {
            let pass = InitializedPassState::new(body, tcx, &mut self.cache);
            let out = pass.collect_locals().collect_body().finalize();
            (true, out)
        }
    }
}

struct LocalPassState<'tcx, 'cache> {
    body: Body,
    tcx: TyCtxt<'tcx>,
    cache: &'cache mut FunctionInstanceCache,
    values: Vec<Local>,
}

struct InstrumentationData<'tcx, 'cache> {
    tcx: TyCtxt<'tcx>,
    cache: &'cache mut FunctionInstanceCache,
    meta_stack: HashMap<Local, Local>,
    body: CachedBodyMutator,
}

struct BodyMutationPassState<'tcx, 'cache> {
    values: Vec<Local>,
    instrumentation_data: InstrumentationData<'tcx, 'cache>,
}

impl<'tcx, 'cache> InstrumentationData<'tcx, 'cache> {
    fn assign_ptr(
        body: &mut CachedBodyMutator,
        lvalue: Local,
        rvalue: Local,
        span: Span) {
        let lvalue = Place::from(lvalue);
        let rvalue = Rvalue::AddressOf(Mutability::Not, Place::from(rvalue));
        let kind = StatementKind::Assign(lvalue, rvalue);
        body.insert_statement(Statement { kind, span });
    }

    /// Initialize the monitors
    fn instrument_initialize(
        &mut self,
    ) -> Result<(), Error> {
        let instance = self.cache.register(&self.tcx, FunctionSignature::new("KaniInitializeSState", &[]))?;
        let body = &mut self.body;
        body.call(instance, [].to_vec(), body.unit);
        Ok(())
    }

    /// For some local, say let x: T;
    /// instrument it with the functions that initialize the stack:
    /// let ptr_x: *const T = &raw const x;
    /// initialize_local(ptr_x);
    fn instrument_local(
        &mut self,
        local: usize,
    ) -> Result<(), Error> {
        let ty = self.body.local(local).ty;
        let ptr_ty = Ty::new_ptr(ty, Mutability::Not);
        let span = self.body.span().clone();
        let body = &mut self.body;
        let local_ptr = self.meta_stack.entry(local).or_insert_with(|| body.new_local(ptr_ty, Mutability::Not));
        Self::assign_ptr(body, *local_ptr, local, span);
        let instance = self.cache.register(&self.tcx, FunctionSignature::new("KaniInitializeLocal", &[GenericArgKind::Type(ty)]))?;
        body.call(instance, [*local_ptr].to_vec(), body.unit);
        Ok(())
    }

    fn instrument_new_stack_reference(&mut self, idx: &MutatorIndex, lvalue: Local, rvalue: Local) -> Result<(), Error> {
        // Initialize the constants
        let ty = self.body.local(rvalue).ty;
        let lvalue_ref = self.meta_stack.get(&lvalue).unwrap();
        let rvalue_ref = self.meta_stack.get(&rvalue).unwrap();
        let instance = self.cache.register(&self.tcx, FunctionSignature::new("KaniNewMutRefFromValue", &[GenericArgKind::Type(ty)]))?;
        self.body.call(instance, vec![*lvalue_ref, *rvalue_ref], self.body.unit);
        self.body.split(idx);
        Ok(())
    }

    fn instrument_stack_check_ref(&mut self, idx: &MutatorIndex, place: Local, ty: Ty) -> Result<(), Error> {
        // Initialize the constants
        let place_ref = self.meta_stack.get(&place).unwrap();
        let instance = self.cache.register(&self.tcx, FunctionSignature::new("KaniStackCheckRef", &[GenericArgKind::Type(ty)]))?;
        self.body.call(instance, vec![*place_ref], self.body.unit);
        self.body.split(idx);
        Ok(())
    }

    fn instrument_stack_check_ptr(&mut self, idx: &MutatorIndex, place: Local, ty: Ty) -> Result<(), Error> {
        // Initialize the constants
        let place_ref = self.meta_stack.get(&place).unwrap();
        let instance = self.cache.register(&self.tcx, FunctionSignature::new("KaniStackCheckPtr", &[GenericArgKind::Type(ty)]))?;
        self.body.call(instance, vec![*place_ref], self.body.unit);
        self.body.split(idx);
        Ok(())
    }

    fn instrument_new_mut_ref_from_raw(&mut self, idx: &MutatorIndex, created: Local, raw: Local) -> Result<(), Error> {
        // Initialize the constants
        let ty = self.body.local(created).ty;
        let created_ref = self.meta_stack.get(&created).unwrap();
        let reference_ref = self.meta_stack.get(&raw).unwrap();
        let instance = self.cache.register(&self.tcx, FunctionSignature::new("KaniNewMutRefFromRaw", &[GenericArgKind::Type(ty)]))?;
        self.body.call(instance, vec![*created_ref, *reference_ref], self.body.unit);
        self.body.split(idx);
        Ok(())
    }

    fn instrument_new_mut_raw_from_ref(&mut self, idx: &MutatorIndex, created: Local, reference: Local) -> Result<(), Error> {
        // Initialize the constants
        let ty = self.body.local(created).ty;
        let created_ref = self.meta_stack.get(&created).unwrap();
        let reference_ref = self.meta_stack.get(&reference).unwrap();
        let instance = self.cache.register(&self.tcx, FunctionSignature::new("KaniNewMutRawFromRef", &[GenericArgKind::Type(ty)]))?;
        self.body.call(instance, vec![*created_ref, *reference_ref], self.body.unit);
        self.body.split(idx);
        Ok(())
    }

    fn instrument_index(&mut self, _values: &Vec<Local>, idx: &MutatorIndex) -> Result<(), Error> {
        match self.body.inspect(idx) {
            Instruction::Stmt(Statement { kind, ..} ) => {
                match kind {
                    StatementKind::Assign(to, rvalue) => {
                        let to = to.clone();
                        match rvalue {
                            Rvalue::Ref(_, BorrowKind::Mut { .. }, from) => {
                                match from.projection[..] {
                                    [] => {
                                        // Direct reference to stack local
                                        // x = &y
                                        self.instrument_new_stack_reference(idx, to.local, from.local)?;
                                    },
                                    [ProjectionElem::Deref] => {
                                        // Reborrow
                                        // x : &mut T = &*(y : *mut T)
                                        let from = from.local; // Copy to avoid borrow
                                        let to = to.local;     // Copy to avoid borrow
                                        match self.body.local(to).ty.kind() {
                                            TyKind::RigidTy(RigidTy::Ref(_, _ty, _)) => {
                                                eprintln!("Reborrow from reference not yet handled");
                                            },
                                            TyKind::RigidTy(RigidTy::RawPtr(ty, _)) => {
                                                self.instrument_stack_check_ref(idx, from, ty)?;
                                                self.instrument_new_mut_ref_from_raw(idx, to, from)?;
                                            },
                                            _ => {}
                                        }
                                    },
                                    _ => {
                                        eprintln!("Field projections not yet handled");
                                    }
                                }
                            },
                            Rvalue::AddressOf(Mutability::Mut, from) => {
                                match from.projection[..] {
                                    [] => {
                                        // x = &raw y
                                        eprintln!("addr of not yet handled");
                                    },
                                    [ProjectionElem::Deref] => {
                                        // x = &raw mut *(y: &mut T)
                                        let from = from.local; // Copy to avoid borrow
                                        let to = to.local; // Copy to avoid borrow
                                        match self.body.local(to).ty.kind() {
                                            TyKind::RigidTy(RigidTy::Ref(_, ty, _)) => {
                                                self.instrument_stack_check_ref(idx, from, ty)?;
                                                self.instrument_new_mut_raw_from_ref(idx, to, from)?;
                                            },
                                            TyKind::RigidTy(RigidTy::RawPtr(_ty, _)) => {
                                                eprintln!("Pointer to pointer casts not yet handled");
                                            }
                                            _ => {}
                                        }
                                    },
                                    _ => {
                                    }
                                }
                            },
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
                                        self.instrument_stack_check_ref(idx, to, ty)?;
                                    },
                                    TyKind::RigidTy(RigidTy::RawPtr(ty, _)) => {
                                        self.instrument_stack_check_ptr(idx, to, ty)?;
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
                    },
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

    fn instrument_locals(&mut self,
                         values: &Vec<Local>) -> Result<(), Error> {
        self.instrument_initialize()?;
        for local in values {
            self.instrument_local(*local)?
        }
        Ok(())
    }

    fn instrument_instructions(&mut self, values: &Vec<Local>) -> Result<(), Error> {
        let mut index = self.body.new_index();
        let mut status = MutatorIndexStatus::Remaining;
        while status == MutatorIndexStatus::Remaining {
            self.instrument_index(values, &index)?;
            status = self.body.decrement_index(&mut index);
        }
        Ok(())
    }
}

impl<'tcx, 'cache> BodyMutationPassState<'tcx, 'cache> {
    fn instrument_locals(&mut self) -> Result<(), Error> {
        self.instrumentation_data.instrument_locals(&self.values)
    }

    fn instrument_instructions(&mut self) -> Result<(), Error> {
        self.instrumentation_data.instrument_instructions(&self.values)?;
        Ok(())
    }

    fn finalize(mut self) -> Body {
        self.instrument_locals().unwrap();
        self.instrumentation_data.body.finalize_prologue();
        self.instrument_instructions().unwrap();
        self.instrumentation_data.body.finalize()
    }
}

struct BodyMutator {
    blocks: Vec<BasicBlock>,
    locals: Vec<LocalDecl>,
    arg_count: usize,
    var_debug_info: Vec<VarDebugInfo>,
    spread_arg: Option<Local>,
    span: Span,

    ghost_locals: Vec<LocalDecl>,
    ghost_blocks: Vec<BasicBlock>,
    ghost_statements: Vec<Statement>,
}

struct CachedBodyMutator {
    body: BodyMutator,
    unit: Local,
    cache: HashMap<Instance, Local>,
}

impl BodyMutator {
    fn new(blocks: Vec<BasicBlock>, locals: Vec<LocalDecl>, arg_count: usize, var_debug_info: Vec<VarDebugInfo>, spread_arg: Option<Local>, span: Span, ghost_locals: Vec<LocalDecl>, ghost_blocks: Vec<BasicBlock>, statements: Vec<Statement>) -> Self {
        BodyMutator { blocks, locals, arg_count, var_debug_info, spread_arg, span, ghost_locals, ghost_blocks, ghost_statements: statements }
    }

    fn gen_bb0(body: &mut Body) -> BasicBlock {
        let target = body.blocks.len() + 1;
        let kind = TerminatorKind::Goto { target };
        let span = body.span;
        let terminator = Terminator { kind, span };
        let statements = Vec::new();
        std::mem::replace(&mut body.blocks[0], BasicBlock { statements, terminator })
    }

    fn gen_unit(body: &Body) -> LocalDecl {
        let ty = Ty::new_tuple(&[]);
        let span = body.span;
        let mutability = Mutability::Not;
        LocalDecl { ty, span, mutability }
    }

    fn from(mut body: Body) -> Self {
        let bb0 = Self::gen_bb0(&mut body);
        body.blocks.push(bb0);
        let ghost_locals = vec![Self::gen_unit(&body)];
        let ghost_blocks = vec![];
        let locals = body.locals().to_vec();
        let arg_count = body.arg_locals().len();
        let spread_arg = body.spread_arg();
        let debug_info = body.var_debug_info;
        let statements = Vec::new();
        BodyMutator::new(body.blocks, locals, arg_count, debug_info, spread_arg, body.span, ghost_locals, ghost_blocks, statements)
    }
}

impl<'tcx, 'cache> LocalPassState<'tcx, 'cache> {
    fn collect_body(self) -> BodyMutationPassState<'tcx, 'cache> {
        let values = self.values;
        let instrumentation_data = InstrumentationData {
            tcx: self.tcx,
            cache: self.cache,
            meta_stack: HashMap::new(),
            body: CachedBodyMutator::from(self.body),
        };
        BodyMutationPassState {
            values,
            instrumentation_data
        }
    }
}

struct CollectLocalVisitor {
    values: Vec<Local>,
}

impl CollectLocalVisitor {
    fn new() -> Self {
        let values = Vec::new();
        CollectLocalVisitor { values }
    }
}

impl MirVisitor for CollectLocalVisitor {
    fn visit_local_decl(&mut self, local: Local, decl: &LocalDecl) {
        // // For now collect em all
        let _ = decl;
        self.values.push(local);
        // if let TyKind::RigidTy(ty) = decl.ty.kind() {
        //     if function_ty(&ty) {
        //         eprintln!("WARN: Function types not yet supported ")
        //     } else if value_ty(&ty) {
        //         self.values.push(local);
        //     } else if !value_reference_ty(&ty) {
        //         panic!("Type {:?} not supported by the analysis.", ty);
        //     }
        // }
    }
}

impl FunctionInstanceCache {
    fn register(&mut self, ctx: &TyCtxt, sig: FunctionSignature) -> Result<&Instance, Error> {
        let FunctionInstanceCache(cache) = self;
        for i in 0..cache.len() {
            if sig == cache[i].signature {
                return Ok(&cache[i].instance);
            }
        }
        let fndef =
            super::super::find_fn_def(*ctx, &sig.name)
            .ok_or(Error::new(format!("Not found: {}", &sig.name)))?;
        let instance = Instance::resolve(fndef, &GenericArgs(sig.args.clone()))?;
        cache.push(FunctionInstance::new(sig, instance));
        Ok(&cache[cache.len() - 1].instance)
    }

    #[allow(unused)]
    fn get(&self, sig: &FunctionSignature) -> Result<&Instance, Error> {
        let FunctionInstanceCache(cache) = self;
        for FunctionInstance {
            signature,
            instance,
        } in cache {
            if *sig == *signature {
                return Ok(instance);
            }
        }
        Err(Error::new(format!("Not found: {:?}", sig)))
    }
}

impl CachedBodyMutator {
    fn from(body: Body) -> Self {
        let mut body = BodyMutator::from(body);
        let unit = body.new_local(Ty::new_tuple(&[]), Mutability::Not);
        let cache = HashMap::new();
        CachedBodyMutator { body, unit, cache }
    }

    fn local(&self, idx: usize) -> &LocalDecl {
        if idx > self.body.locals.len() {
            &self.body.ghost_locals[idx - self.body.locals.len()]
        } else {
            &self.body.locals[idx]
        }
    }

    fn new_local(&mut self, ty: Ty, mutability: Mutability) -> Local {
        self.body.new_local(ty, mutability)
    }

    fn call(&mut self, callee: &Instance, args: Vec<Local>, local: Local) {
        let func_local;
        {
            let cache = &mut self.cache;
            let body = &mut self.body;
            {
                func_local = cache.entry(*callee).or_insert_with(|| body.new_local(callee.ty(), Mutability::Not));
            }
        }
        self.body.call(*func_local, args, local);
    }

    fn finalize_prologue(&mut self) {
        self.body.finalize_prologue();
    }

    fn insert_statement(&mut self, stmt: Statement) {
        self.body.ghost_statements.push(stmt);
    }

    fn new_index(&mut self) -> MutatorIndex {
        self.body.new_index()
    }

    fn decrement_index(&mut self, idx: &mut MutatorIndex) -> MutatorIndexStatus {
        self.body.decrement(idx)
    }

    fn split(&mut self, idx: &MutatorIndex) {
        self.body.split(idx);
    }

    fn inspect(&self, idx: &MutatorIndex) -> Instruction {
        self.body.inspect(idx)
    }

    fn finalize(self) -> Body {
        self.body.finalize()
    }

    fn span(&self) -> Span {
        self.body.span
    }
}

#[derive(Debug)]
struct MutatorIndex {
    bb: BasicBlockIdx,
    idx: usize,
    span: Span
}

#[derive(PartialEq, Eq)]
enum MutatorIndexStatus {
    Remaining,
    Done
}

enum Instruction<'a> {
    Stmt(&'a Statement),
    #[allow(unused)]
    Term(&'a Terminator)
}

impl BodyMutator {
    fn new_local(&mut self, ty: Ty, mutability: Mutability) -> Local {
        let span = self.span;
        let decl = LocalDecl { ty, span, mutability };
        let local = self.locals.len() + self.ghost_locals.len();
        self.ghost_locals.push(decl);
        local
    }

    fn call(&mut self, callee: Local, args: Vec<Local>, local: Local) {
        let projection = Vec::new();
        let destination = Place { local, projection };
        let args = args.into_iter().map(|v| Operand::Copy(Place { local: v, projection: vec![] } )).collect();
        let func = Operand::Copy(Place::from(callee));
        let unwind = UnwindAction::Terminate;
        let target = Some(self.next_block());
        let kind = TerminatorKind::Call { func, args, destination, target, unwind };
        let span = self.span;
        let terminator = Terminator { kind, span };
        let statements = std::mem::replace(&mut self.ghost_statements, Vec::new());
        self.ghost_blocks.push(BasicBlock { statements, terminator });
    }

    fn finalize_prologue(&mut self) {
        let kind = TerminatorKind::Goto { target: self.blocks.len() - 1 };
        let span = self.span;
        let terminator = Terminator { kind, span };
        self.insert_bb(terminator);
    }

    fn new_index(&self) -> MutatorIndex {
        let len = self.blocks.len();
        let bb = std::cmp::max(len, 1) - 1;
        let idx = if len > 0 {
            std::cmp::max(self.blocks[bb].statements.len(), 1)
             - 1
        } else {
            0
        };
        let span = self.span;
        MutatorIndex { bb, idx, span }
    }

    fn decrement(&self, index: &mut MutatorIndex) -> MutatorIndexStatus {
        let mut status = MutatorIndexStatus::Done;
        if index.idx > 0 || index.bb > 0 {
            status = MutatorIndexStatus::Remaining;
        }
        if index.idx > 0 {
            if index.idx < self.blocks[index.bb].statements.len() {
                index.span = self.blocks[index.bb]
                    .statements[index.idx].span;
            } else {
                index.span = self.blocks[index.bb]
                    .terminator.span;
            }
            index.idx -= 1;
        } else if index.bb > 0 {
            index.bb -= 1;
            index.span = self.blocks[index.bb].terminator.span;
            index.idx = self.blocks[index.bb].statements.len()
        }
        status
    }

    fn inspect(&self, index: &MutatorIndex) -> Instruction {
        if index.idx >= self.blocks[index.bb].statements.len() {
            Instruction::Term(&self.blocks[index.bb].terminator)
        } else {
            Instruction::Stmt(&self.blocks[index.bb].statements[index.idx])
        }
    }

    fn split(&mut self, index: &MutatorIndex) {
        let kind = TerminatorKind::Goto { target: self.blocks.len() + self.ghost_blocks.len() - 1 };
        let span = index.span;
        let term = Terminator { kind, span };
        let len = self.blocks[index.bb].statements.len();
        if index.idx < len {
            self.ghost_statements.extend(self.blocks[index.bb].statements.split_off(index.idx + 1));
        }
        let term = std::mem::replace(&mut self.blocks[index.bb].terminator, term);
        self.insert_bb(term);
    }

    fn next_block(&self) -> usize {
        self.blocks.len() + self.ghost_blocks.len() + 1
    }

    fn insert_bb(&mut self, terminator: Terminator) {
        let statements = std::mem::replace(&mut self.ghost_statements, Vec::new());
        let execute_original_body = BasicBlock { statements, terminator };
        self.ghost_blocks.push(execute_original_body);
    }

    fn finalize(self) -> Body {
        match self {
            BodyMutator { mut blocks, mut locals, arg_count, var_debug_info, spread_arg, span, ghost_locals, ghost_blocks, ghost_statements } => {
                assert!(ghost_statements.len() == 0);
                blocks.extend(ghost_blocks.into_iter());
                locals.extend(ghost_locals.into_iter());
                Body::new(blocks, locals, arg_count, var_debug_info, spread_arg, span)
            }
        }
    }
}
