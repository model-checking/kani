// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::io::Write;

use crate::codegen_boogie::overrides::{fn_hooks, BoogieHooks};
use crate::kani_queries::QueryDb;
use boogie_ast::boogie_program::{BinaryOp, BoogieProgram, Expr, Literal, Procedure, Stmt, Type};
use rustc_middle::mir::interpret::{ConstValue, Scalar};
use rustc_middle::mir::traversal::reverse_postorder;
use rustc_middle::mir::{
    BasicBlock, BasicBlockData, BinOp, Constant, ConstantKind, HasLocalDecls, Local, LocalDecls,
    Operand, Place, Rvalue, Statement, StatementKind, Terminator, TerminatorKind,
};
use rustc_middle::span_bug;
use rustc_middle::ty::layout::{
    HasParamEnv, HasTyCtxt, LayoutError, LayoutOf, LayoutOfHelpers, TyAndLayout,
};
use rustc_middle::ty::{self, Instance, IntTy, Ty, TyCtxt, UintTy};
use rustc_span::Span;
use rustc_target::abi::{HasDataLayout, TargetDataLayout};
use tracing::{debug, debug_span, trace};

/// A context that provides the main methods for translating MIR constructs to
/// Boogie and stores what has been codegen so far
pub struct BoogieCtx<'tcx> {
    /// the typing context
    pub tcx: TyCtxt<'tcx>,
    /// a snapshot of the query values. The queries shouldn't change at this point,
    /// so we just keep a copy.
    pub queries: QueryDb,
    /// the Boogie program
    program: BoogieProgram,
    pub hooks: BoogieHooks<'tcx>,
}

impl<'tcx> BoogieCtx<'tcx> {
    pub fn new(tcx: TyCtxt<'tcx>, queries: QueryDb) -> BoogieCtx<'tcx> {
        BoogieCtx { tcx, queries, program: BoogieProgram::new(), hooks: fn_hooks() }
    }

    /// Codegen a function into a Boogie procedure.
    /// Returns `None` if the function is a hook.
    pub fn codegen_function(&self, instance: Instance<'tcx>) -> Option<Procedure> {
        debug!(?instance, "boogie_codegen_function");
        if self.hooks.hook_applies(self.tcx, instance).is_some() {
            debug!("skipping hook function `{instance}`");
            return None;
        }
        let mut decl = self.codegen_declare_variables(instance);
        let body = self.codegen_body(instance);
        decl.push(body);
        Some(Procedure::new(
            self.tcx.symbol_name(instance).name.to_string(),
            vec![],
            vec![],
            None,
            Stmt::Block { statements: decl },
        ))
    }

    pub fn codegen_declare_variables(&self, instance: Instance<'tcx>) -> Vec<Stmt> {
        let mir = self.tcx.instance_mir(instance.def);
        let ldecls = mir.local_decls();
        let decls: Vec<Stmt> = ldecls
            .indices()
            .enumerate()
            .filter_map(|(_idx, lc)| {
                let typ = ldecls[lc].ty;
                if self.layout_of(typ).is_zst() {
                    return None;
                }
                debug!(?lc, ?typ, "codegen_declare_variables");
                let name = format!("{lc:?}");
                let boogie_type = self.codegen_type(typ);
                Some(Stmt::Decl { name, typ: boogie_type })
            })
            .collect();
        decls
    }

    pub fn codegen_type(&self, ty: Ty<'tcx>) -> Type {
        trace!(typ=?ty, "codegen_type");
        match ty.kind() {
            ty::Bool => Type::Bool,
            ty::Int(_ity) => Type::Int, // TODO: use Bv
            _ => todo!(),
        }
    }

    pub fn codegen_body(&self, instance: Instance<'tcx>) -> Stmt {
        let mir = self.tcx.instance_mir(instance.def);
        let statements: Vec<Stmt> = reverse_postorder(mir)
            .map(|(bb, bbd)| self.codegen_block(mir.local_decls(), bb, bbd))
            .collect();
        Stmt::Block { statements }
    }

    pub fn codegen_block(
        &self,
        local_decls: &LocalDecls<'tcx>,
        bb: BasicBlock,
        bbd: &BasicBlockData<'tcx>,
    ) -> Stmt {
        debug!(?bb, ?bbd, "codegen_block");
        // the first statement should be labelled. if there is no statements, then the
        // terminator should be labelled.
        let statements = match bbd.statements.len() {
            0 => {
                let term = bbd.terminator();
                let tcode = self.codegen_terminator(local_decls, term);
                vec![tcode]
            }
            _ => {
                let mut statements: Vec<Stmt> =
                    bbd.statements.iter().map(|stmt| self.codegen_statement(stmt)).collect();

                let term = self.codegen_terminator(local_decls, bbd.terminator());
                statements.push(term);
                statements
            }
        };
        Stmt::Block { statements }
    }

    pub fn codegen_statement(&self, stmt: &Statement<'tcx>) -> Stmt {
        match &stmt.kind {
            StatementKind::Assign(box (place, rvalue)) => {
                debug!(?place, ?rvalue, "codegen_statement");
                let rv = self.codegen_rvalue(rvalue);
                Stmt::Assignment { target: format!("{:?}", place.local), value: rv }
            }
            StatementKind::FakeRead(..)
            | StatementKind::SetDiscriminant { .. }
            | StatementKind::Deinit(..)
            | StatementKind::StorageLive(..)
            | StatementKind::StorageDead(..)
            | StatementKind::Retag(..)
            | StatementKind::PlaceMention(..)
            | StatementKind::AscribeUserType(..)
            | StatementKind::Coverage(..)
            | StatementKind::Intrinsic(..)
            | StatementKind::ConstEvalCounter
            | StatementKind::Nop => todo!(),
        }
    }

    pub fn codegen_rvalue(&self, rvalue: &Rvalue<'tcx>) -> Expr {
        debug!(rvalue=?rvalue, "codegen_rvalue");
        match rvalue {
            Rvalue::Use(operand) => self.codegen_operand(operand),
            Rvalue::BinaryOp(binop, box (lhs, rhs)) => self.codegen_binary_op(binop, lhs, rhs),
            _ => todo!(),
        }
    }

    pub fn codegen_binary_op(
        &self,
        binop: &BinOp,
        lhs: &Operand<'tcx>,
        rhs: &Operand<'tcx>,
    ) -> Expr {
        match binop {
            BinOp::Eq => Expr::BinaryOp {
                op: BinaryOp::Eq,
                left: Box::new(self.codegen_operand(lhs)),
                right: Box::new(self.codegen_operand(rhs)),
            },
            _ => todo!(),
        }
    }

    pub fn codegen_terminator(
        &self,
        local_decls: &LocalDecls<'tcx>,
        term: &Terminator<'tcx>,
    ) -> Stmt {
        let _trace_span = debug_span!("CodegenTerminator", statement = ?term.kind).entered();
        debug!("handling terminator {:?}", term);
        match &term.kind {
            TerminatorKind::Call { func, args, destination, target, .. } => self.codegen_funcall(
                local_decls,
                func,
                args,
                destination,
                target,
                term.source_info.span,
            ),
            TerminatorKind::Return => Stmt::Return,
            _ => todo!(),
        }
    }

    pub fn codegen_funcall(
        &self,
        local_decls: &LocalDecls<'tcx>,
        func: &Operand<'tcx>,
        args: &[Operand<'tcx>],
        destination: &Place<'tcx>,
        target: &Option<BasicBlock>,
        span: Span,
    ) -> Stmt {
        debug!(?func, ?args, ?destination, ?span, "codegen_funcall");
        let fargs = self.codegen_funcall_args(local_decls, args);
        let funct = self.operand_ty(local_decls, func);
        // TODO: Only hooks are handled currently
        match &funct.kind() {
            ty::FnDef(defid, substs) => {
                let instance =
                    Instance::expect_resolve(self.tcx, ty::ParamEnv::reveal_all(), *defid, substs);

                if let Some(hk) = self.hooks.hook_applies(self.tcx, instance) {
                    return hk.handle(self, instance, fargs, *destination, *target, Some(span));
                }
                todo!()
            }
            _ => todo!(),
        }
    }

    pub fn codegen_funcall_args(
        &self,
        local_decls: &LocalDecls<'tcx>,
        args: &[Operand<'tcx>],
    ) -> Vec<Expr> {
        debug!(?args, "codegen_funcall_args");
        args.iter()
            .filter_map(|o| {
                let ty = self.operand_ty(local_decls, o);
                // TODO: handle non-primitive types
                if ty.is_primitive() {
                    return Some(self.codegen_operand(o));
                }
                None
            })
            .collect()
    }

    pub fn codegen_operand(&self, o: &Operand<'tcx>) -> Expr {
        trace!(operand=?o, "codegen_operand");
        // A MIR operand is either a constant (literal or `const` declaration)
        // or a place (being moved or copied for this operation).
        // An "operand" in MIR is the argument to an "Rvalue" (and is also used
        // by some statements.)
        match o {
            Operand::Copy(place) | Operand::Move(place) => self.codegen_place(place),
            Operand::Constant(c) => self.codegen_constant(c),
        }
    }

    pub fn codegen_place(&self, place: &Place<'tcx>) -> Expr {
        debug!(place=?place, "codegen_place");
        debug!(place.local=?place.local, "codegen_place");
        debug!(place.projection=?place.projection, "codegen_place");
        self.codegen_local(place.local)
    }

    pub fn codegen_local(&self, local: Local) -> Expr {
        // TODO: handle function definitions
        Expr::Symbol { name: format!("{local:?}") }
    }

    pub fn codegen_constant(&self, c: &Constant<'tcx>) -> Expr {
        trace!(constant=?c, "codegen_constant");
        // TODO: monomorphize
        match c.literal {
            ConstantKind::Val(val, ty) => self.codegen_constant_value(val, ty),
            _ => todo!(),
        }
    }

    pub fn codegen_constant_value(&self, val: ConstValue<'tcx>, ty: Ty<'tcx>) -> Expr {
        debug!(val=?val, "codegen_constant_value");
        match val {
            ConstValue::Scalar(s) => self.codegen_scalar(s, ty),
            _ => todo!(),
        }
    }

    pub fn codegen_scalar(&self, s: Scalar, ty: Ty<'tcx>) -> Expr {
        match (s, ty.kind()) {
            (Scalar::Int(_), ty::Bool) => Expr::Literal(Literal::Bool(s.to_bool().unwrap())),
            (Scalar::Int(_), ty::Int(it)) => match it {
                IntTy::I8 => Expr::Literal(Literal::Int(s.to_i8().unwrap().into())),
                IntTy::I16 => Expr::Literal(Literal::Int(s.to_i16().unwrap().into())),
                IntTy::I32 => Expr::Literal(Literal::Int(s.to_i32().unwrap().into())),
                IntTy::I64 => Expr::Literal(Literal::Int(s.to_i64().unwrap().into())),
                IntTy::I128 => Expr::Literal(Literal::Int(s.to_i128().unwrap().into())),
                IntTy::Isize => {
                    Expr::Literal(Literal::Int(s.to_target_isize(self).unwrap().into()))
                }
            },
            (Scalar::Int(_), ty::Uint(it)) => match it {
                UintTy::U8 => Expr::Literal(Literal::Int(s.to_u8().unwrap().into())),
                UintTy::U16 => Expr::Literal(Literal::Int(s.to_u16().unwrap().into())),
                UintTy::U32 => Expr::Literal(Literal::Int(s.to_u32().unwrap().into())),
                UintTy::U64 => Expr::Literal(Literal::Int(s.to_u64().unwrap().into())),
                UintTy::U128 => Expr::Literal(Literal::Int(s.to_u128().unwrap().into())),
                UintTy::Usize => {
                    Expr::Literal(Literal::Int(s.to_target_isize(self).unwrap().into()))
                }
            },
            _ => todo!(),
        }
    }

    pub fn write<T: Write>(&self, writer: &mut T) -> std::io::Result<()> {
        self.program.write_to(writer)?;
        Ok(())
    }

    pub fn operand_ty(&self, local_decls: &LocalDecls<'tcx>, o: &Operand<'tcx>) -> Ty<'tcx> {
        // TODO: monomorphize
        o.ty(local_decls, self.tcx)
    }
}

impl<'tcx> LayoutOfHelpers<'tcx> for BoogieCtx<'tcx> {
    type LayoutOfResult = TyAndLayout<'tcx>;

    fn handle_layout_err(&self, err: LayoutError<'tcx>, span: Span, ty: Ty<'tcx>) -> ! {
        span_bug!(span, "failed to get layout for `{}`: {}", ty, err)
    }
}

impl<'tcx> HasParamEnv<'tcx> for BoogieCtx<'tcx> {
    fn param_env(&self) -> ty::ParamEnv<'tcx> {
        ty::ParamEnv::reveal_all()
    }
}

impl<'tcx> HasTyCtxt<'tcx> for BoogieCtx<'tcx> {
    fn tcx(&self) -> TyCtxt<'tcx> {
        self.tcx
    }
}

impl<'tcx> HasDataLayout for BoogieCtx<'tcx> {
    fn data_layout(&self) -> &TargetDataLayout {
        self.tcx.data_layout()
    }
}

impl<'tcx> BoogieCtx<'tcx> {
    pub fn add_procedure(&mut self, procedure: Procedure) {
        self.program.add_procedure(procedure);
    }
}
