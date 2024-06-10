// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
//! Utility functions that allow us to modify a function body.

use crate::kani_middle::find_fn_def;
use rustc_middle::ty::TyCtxt;
use stable_mir::mir::mono::Instance;
use stable_mir::mir::*;
use stable_mir::ty::{Const, GenericArgs, Span, Ty, UintTy};
use std::fmt::Debug;
use std::mem;

/// This structure mimics a Body that can actually be modified.
pub struct MutableBody {
    blocks: Vec<BasicBlock>,

    /// Declarations of locals within the function.
    ///
    /// The first local is the return value pointer, followed by `arg_count`
    /// locals for the function arguments, followed by any user-declared
    /// variables and temporaries.
    locals: Vec<LocalDecl>,

    /// The number of arguments this function takes.
    arg_count: usize,

    /// Debug information pertaining to user variables, including captures.
    var_debug_info: Vec<VarDebugInfo>,

    /// Mark an argument (which must be a tuple) as getting passed as its individual components.
    ///
    /// This is used for the "rust-call" ABI such as closures.
    spread_arg: Option<Local>,

    /// The span that covers the entire function body.
    span: Span,
}

impl MutableBody {
    /// Get the basic blocks of this builder.
    pub fn blocks(&self) -> &[BasicBlock] {
        &self.blocks
    }

    pub fn locals(&self) -> &[LocalDecl] {
        &self.locals
    }

    /// Create a mutable body from the original MIR body.
    pub fn from(body: Body) -> Self {
        MutableBody {
            locals: body.locals().to_vec(),
            arg_count: body.arg_locals().len(),
            spread_arg: body.spread_arg(),
            blocks: body.blocks,
            var_debug_info: body.var_debug_info,
            span: body.span,
        }
    }

    /// Create the new body consuming this mutable body.
    pub fn into(self) -> Body {
        Body::new(
            self.blocks,
            self.locals,
            self.arg_count,
            self.var_debug_info,
            self.spread_arg,
            self.span,
        )
    }

    /// Add a new local to the body with the given attributes.
    pub fn new_local(&mut self, ty: Ty, span: Span, mutability: Mutability) -> Local {
        let decl = LocalDecl { ty, span, mutability };
        let local = self.locals.len();
        self.locals.push(decl);
        local
    }

    pub fn new_str_operand(&mut self, msg: &str, span: Span) -> Operand {
        let literal = Const::from_str(msg);
        Operand::Constant(Constant { span, user_ty: None, literal })
    }

    pub fn new_const_operand(&mut self, val: u128, uint_ty: UintTy, span: Span) -> Operand {
        let literal = Const::try_from_uint(val, uint_ty).unwrap();
        Operand::Constant(Constant { span, user_ty: None, literal })
    }

    /// Create a raw pointer of `*mut type` and return a new local where that value is stored.
    pub fn new_cast_ptr(
        &mut self,
        from: Operand,
        pointee_ty: Ty,
        mutability: Mutability,
        before: &mut SourceInstruction,
    ) -> Local {
        assert!(from.ty(self.locals()).unwrap().kind().is_raw_ptr());
        let target_ty = Ty::new_ptr(pointee_ty, mutability);
        let rvalue = Rvalue::Cast(CastKind::PtrToPtr, from, target_ty);
        self.new_assignment(rvalue, before)
    }

    /// Add a new assignment for the given binary operation.
    ///
    /// Return the local where the result is saved.
    pub fn new_binary_op(
        &mut self,
        bin_op: BinOp,
        lhs: Operand,
        rhs: Operand,
        before: &mut SourceInstruction,
    ) -> Local {
        let rvalue = Rvalue::BinaryOp(bin_op, lhs, rhs);
        self.new_assignment(rvalue, before)
    }

    /// Add a new assignment.
    ///
    /// Return  local where the result is saved.
    pub fn new_assignment(&mut self, rvalue: Rvalue, before: &mut SourceInstruction) -> Local {
        let span = before.span(&self.blocks);
        let ret_ty = rvalue.ty(&self.locals).unwrap();
        let result = self.new_local(ret_ty, span, Mutability::Not);
        let stmt = Statement { kind: StatementKind::Assign(Place::from(result), rvalue), span };
        self.insert_stmt(stmt, before);
        result
    }

    /// Add a new assert to the basic block indicated by the given index.
    ///
    /// The new assertion will have the same span as the source instruction, and the basic block
    /// will be split. The source instruction will be adjusted to point to the first instruction in
    /// the new basic block.
    pub fn add_check(
        &mut self,
        tcx: TyCtxt,
        check_type: &CheckType,
        source: &mut SourceInstruction,
        value: Local,
        msg: &str,
    ) {
        assert_eq!(
            self.locals[value].ty,
            Ty::bool_ty(),
            "Expected boolean value as the assert input"
        );
        let new_bb = self.blocks.len();
        let span = source.span(&self.blocks);
        match check_type {
            CheckType::Assert(assert_fn) => {
                let assert_op = Operand::Copy(Place::from(self.new_local(
                    assert_fn.ty(),
                    span,
                    Mutability::Not,
                )));
                let msg_op = self.new_str_operand(msg, span);
                let kind = TerminatorKind::Call {
                    func: assert_op,
                    args: vec![Operand::Move(Place::from(value)), msg_op],
                    destination: Place {
                        local: self.new_local(Ty::new_tuple(&[]), span, Mutability::Not),
                        projection: vec![],
                    },
                    target: Some(new_bb),
                    unwind: UnwindAction::Terminate,
                };
                let terminator = Terminator { kind, span };
                self.split_bb(source, terminator);
            }
            CheckType::Panic | CheckType::NoCore => {
                tcx.sess
                    .dcx()
                    .struct_err("Failed to instrument the code. Cannot find `kani::assert`")
                    .with_note("Kani requires `kani` library in order to verify a crate.")
                    .emit();
                tcx.sess.dcx().abort_if_errors();
                unreachable!();
            }
        }
    }

    /// Split a basic block right before the source location and use the new terminator
    /// in the basic block that was split.
    ///
    /// The source is updated to point to the same instruction which is now in the new basic block.
    pub fn split_bb(&mut self, source: &mut SourceInstruction, new_term: Terminator) {
        let new_bb_idx = self.blocks.len();
        let (idx, bb) = match source {
            SourceInstruction::Statement { idx, bb } => {
                let (orig_idx, orig_bb) = (*idx, *bb);
                *idx = 0;
                *bb = new_bb_idx;
                (orig_idx, orig_bb)
            }
            SourceInstruction::Terminator { bb } => {
                let orig_bb = *bb;
                *bb = new_bb_idx;
                (self.blocks[orig_bb].statements.len(), orig_bb)
            }
        };
        let old_term = mem::replace(&mut self.blocks[bb].terminator, new_term);
        let bb_stmts = &mut self.blocks[bb].statements;
        let remaining = bb_stmts.split_off(idx);
        let new_bb = BasicBlock { statements: remaining, terminator: old_term };
        self.blocks.push(new_bb);
    }

    /// Insert statement before the source instruction and update the source as needed.
    pub fn insert_stmt(&mut self, new_stmt: Statement, before: &mut SourceInstruction) {
        match before {
            SourceInstruction::Statement { idx, bb } => {
                self.blocks[*bb].statements.insert(*idx, new_stmt);
                *idx += 1;
            }
            SourceInstruction::Terminator { bb } => {
                // Append statements at the end of the basic block.
                self.blocks[*bb].statements.push(new_stmt);
            }
        }
    }

    /// Clear all the existing logic of this body and turn it into a simple `return`.
    ///
    /// This function can be used when a new implementation of the body is needed.
    /// For example, Kani intrinsics usually have a dummy body, which is replaced
    /// by the compiler. This function allow us to delete the dummy body before
    /// creating a new one.
    ///
    /// Note: We do not prune the local variables today for simplicity.
    pub fn clear_body(&mut self) {
        self.blocks.clear();
        let terminator = Terminator { kind: TerminatorKind::Return, span: self.span };
        self.blocks.push(BasicBlock { statements: Vec::default(), terminator })
    }
}

#[derive(Clone, Debug)]
pub enum CheckType {
    /// This is used by default when the `kani` crate is available.
    Assert(Instance),
    /// When the `kani` crate is not available, we have to model the check as an `if { panic!() }`.
    Panic,
    /// When building non-core crate, such as `rustc-std-workspace-core`, we cannot
    /// instrument code, but we can still compile them.
    NoCore,
}

impl CheckType {
    /// This will create the type of check that is available in the current crate.
    ///
    /// If `kani` crate is available, this will return [CheckType::Assert], and the instance will
    /// point to `kani::assert`. Otherwise, we will collect the `core::panic_str` method and return
    /// [CheckType::Panic].
    pub fn new(tcx: TyCtxt) -> CheckType {
        if let Some(instance) = find_instance(tcx, "KaniAssert") {
            CheckType::Assert(instance)
        } else if find_instance(tcx, "panic_str").is_some() {
            CheckType::Panic
        } else {
            CheckType::NoCore
        }
    }
}

/// We store the index of an instruction to avoid borrow checker issues and unnecessary copies.
#[derive(Copy, Clone, Debug)]
pub enum SourceInstruction {
    Statement { idx: usize, bb: BasicBlockIdx },
    Terminator { bb: BasicBlockIdx },
}

impl SourceInstruction {
    pub fn span(&self, blocks: &[BasicBlock]) -> Span {
        match *self {
            SourceInstruction::Statement { idx, bb } => blocks[bb].statements[idx].span,
            SourceInstruction::Terminator { bb } => blocks[bb].terminator.span,
        }
    }
}

fn find_instance(tcx: TyCtxt, diagnostic: &str) -> Option<Instance> {
    Instance::resolve(find_fn_def(tcx, diagnostic)?, &GenericArgs(vec![])).ok()
}

/// Basic mutable body visitor.
///
/// We removed many methods for simplicity.
///
/// TODO: Contribute this to stable_mir.
/// <https://github.com/rust-lang/project-stable-mir/issues/81>
///
/// This code was based on the existing MirVisitor:
/// <https://github.com/rust-lang/rust/blob/master/compiler/stable_mir/src/mir/visit.rs>
pub trait MutMirVisitor {
    fn visit_body(&mut self, body: &mut MutableBody) {
        self.super_body(body)
    }

    fn visit_basic_block(&mut self, bb: &mut BasicBlock) {
        self.super_basic_block(bb)
    }

    fn visit_statement(&mut self, stmt: &mut Statement) {
        self.super_statement(stmt)
    }

    fn visit_terminator(&mut self, term: &mut Terminator) {
        self.super_terminator(term)
    }

    fn visit_rvalue(&mut self, rvalue: &mut Rvalue) {
        self.super_rvalue(rvalue)
    }

    fn visit_operand(&mut self, _operand: &mut Operand) {}

    fn super_body(&mut self, body: &mut MutableBody) {
        for bb in body.blocks.iter_mut() {
            self.visit_basic_block(bb);
        }
    }

    fn super_basic_block(&mut self, bb: &mut BasicBlock) {
        for stmt in &mut bb.statements {
            self.visit_statement(stmt);
        }
        self.visit_terminator(&mut bb.terminator);
    }

    fn super_statement(&mut self, stmt: &mut Statement) {
        match &mut stmt.kind {
            StatementKind::Assign(_, rvalue) => {
                self.visit_rvalue(rvalue);
            }
            StatementKind::Intrinsic(intrisic) => match intrisic {
                NonDivergingIntrinsic::Assume(operand) => {
                    self.visit_operand(operand);
                }
                NonDivergingIntrinsic::CopyNonOverlapping(CopyNonOverlapping {
                    src,
                    dst,
                    count,
                }) => {
                    self.visit_operand(src);
                    self.visit_operand(dst);
                    self.visit_operand(count);
                }
            },
            StatementKind::FakeRead(_, _)
            | StatementKind::SetDiscriminant { .. }
            | StatementKind::Deinit(_)
            | StatementKind::StorageLive(_)
            | StatementKind::StorageDead(_)
            | StatementKind::Retag(_, _)
            | StatementKind::PlaceMention(_)
            | StatementKind::AscribeUserType { .. }
            | StatementKind::Coverage(_)
            | StatementKind::ConstEvalCounter
            | StatementKind::Nop => {}
        }
    }

    fn super_terminator(&mut self, term: &mut Terminator) {
        let Terminator { kind, .. } = term;
        match kind {
            TerminatorKind::Assert { cond, .. } => {
                self.visit_operand(cond);
            }
            TerminatorKind::Call { func, args, .. } => {
                self.visit_operand(func);
                for arg in args {
                    self.visit_operand(arg);
                }
            }
            TerminatorKind::SwitchInt { discr, .. } => {
                self.visit_operand(discr);
            }
            TerminatorKind::InlineAsm { .. } => {
                // we don't support inline assembly.
            }
            TerminatorKind::Return
            | TerminatorKind::Goto { .. }
            | TerminatorKind::Resume
            | TerminatorKind::Abort
            | TerminatorKind::Drop { .. }
            | TerminatorKind::Unreachable => {}
        }
    }

    fn super_rvalue(&mut self, rvalue: &mut Rvalue) {
        match rvalue {
            Rvalue::Aggregate(_, operands) => {
                for op in operands {
                    self.visit_operand(op);
                }
            }
            Rvalue::BinaryOp(_, lhs, rhs) | Rvalue::CheckedBinaryOp(_, lhs, rhs) => {
                self.visit_operand(lhs);
                self.visit_operand(rhs);
            }
            Rvalue::Cast(_, op, _) => {
                self.visit_operand(op);
            }
            Rvalue::Repeat(op, _) => {
                self.visit_operand(op);
            }
            Rvalue::ShallowInitBox(op, _) => self.visit_operand(op),
            Rvalue::UnaryOp(_, op) | Rvalue::Use(op) => {
                self.visit_operand(op);
            }
            Rvalue::AddressOf(..) => {}
            Rvalue::CopyForDeref(_) | Rvalue::Discriminant(_) | Rvalue::Len(_) => {}
            Rvalue::Ref(..) => {}
            Rvalue::ThreadLocalRef(_) => {}
            Rvalue::NullaryOp(..) => {}
        }
    }
}
