// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::io::Write;

use crate::kani_queries::QueryDb;
use boogie_ast::boogie_program::{BinaryOp, BoogieProgram, Expr, Literal, Procedure, Stmt, Type};
use rustc_data_structures::fx::FxHashMap;
use rustc_middle::mir::interpret::Scalar;
use rustc_middle::mir::traversal::reverse_postorder;
use rustc_middle::mir::{
    BasicBlock, BasicBlockData, BinOp, Body, Const as mirConst, ConstOperand, ConstValue,
    HasLocalDecls, Local, Operand, Place, Rvalue, Statement, StatementKind, Terminator,
    TerminatorKind, VarDebugInfoContents,
};
use rustc_middle::span_bug;
use rustc_middle::ty::layout::{
    HasParamEnv, HasTyCtxt, LayoutError, LayoutOf, LayoutOfHelpers, TyAndLayout,
};
use rustc_middle::ty::{self, Instance, IntTy, Ty, TyCtxt, UintTy};
use rustc_span::Span;
use rustc_target::abi::{HasDataLayout, TargetDataLayout};
use std::collections::hash_map::Entry;
use tracing::{debug, debug_span, trace};

use super::kani_intrinsic::get_kani_intrinsic;

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
}

impl<'tcx> BoogieCtx<'tcx> {
    pub fn new(tcx: TyCtxt<'tcx>, queries: QueryDb) -> BoogieCtx<'tcx> {
        BoogieCtx { tcx, queries, program: BoogieProgram::new() }
    }

    /// Codegen a function into a Boogie procedure.
    /// Returns `None` if the function is a hook.
    pub fn codegen_function(&self, instance: Instance<'tcx>) -> Option<Procedure> {
        debug!(?instance, "boogie_codegen_function");
        if get_kani_intrinsic(self.tcx, instance).is_some() {
            debug!("skipping kani intrinsic `{instance}`");
            return None;
        }
        let fcx = FunctionCtx::new(self, instance);
        let mut decl = fcx.codegen_declare_variables();
        let body = fcx.codegen_body();
        decl.push(body);
        Some(Procedure::new(
            self.tcx.symbol_name(instance).name.to_string(),
            vec![],
            vec![],
            None,
            Stmt::Block { statements: decl },
        ))
    }

    pub fn add_procedure(&mut self, procedure: Procedure) {
        self.program.add_procedure(procedure);
    }

    /// Write the program to the given writer
    pub fn write<T: Write>(&self, writer: &mut T) -> std::io::Result<()> {
        self.program.write_to(writer)?;
        Ok(())
    }
}

pub(crate) struct FunctionCtx<'a, 'tcx> {
    bcx: &'a BoogieCtx<'tcx>,
    instance: Instance<'tcx>,
    mir: &'a Body<'tcx>,
    /// Maps from local to the name of the corresponding Boogie variable.
    local_names: FxHashMap<Local, String>,
}

impl<'a, 'tcx> FunctionCtx<'a, 'tcx> {
    pub fn new(bcx: &'a BoogieCtx<'tcx>, instance: Instance<'tcx>) -> FunctionCtx<'a, 'tcx> {
        // create names for all locals
        let mut local_names = FxHashMap::default();
        let mut name_occurrences: FxHashMap<String, usize> = FxHashMap::default();
        let mir = bcx.tcx.instance_mir(instance.def);
        let ldecls = mir.local_decls();
        for local in ldecls.indices() {
            let debug_info = mir.var_debug_info.iter().find(|info| match info.value {
                VarDebugInfoContents::Place(p) => p.local == local && p.projection.len() == 0,
                VarDebugInfoContents::Const(_) => false,
            });
            let name = if let Some(debug_info) = debug_info {
                let base_name = format!("{}", debug_info.name);
                let entry = name_occurrences.entry(base_name.clone());
                let name = match entry {
                    Entry::Occupied(mut o) => {
                        let occ = o.get_mut();
                        let index = *occ;
                        *occ += 1;
                        format!("{base_name}_{}", index)
                    }
                    Entry::Vacant(v) => {
                        v.insert(1);
                        base_name
                    }
                };
                name
            } else {
                format!("{local:?}")
            };
            local_names.insert(local, name);
        }
        Self { bcx, instance, mir, local_names }
    }

    fn codegen_declare_variables(&self) -> Vec<Stmt> {
        let ldecls = self.mir.local_decls();
        let decls: Vec<Stmt> = ldecls
            .indices()
            .filter_map(|lc| {
                let typ = self.instance.instantiate_mir_and_normalize_erasing_regions(
                    self.tcx(),
                    ty::ParamEnv::reveal_all(),
                    ty::EarlyBinder::bind(ldecls[lc].ty),
                );
                // skip ZSTs
                if self.layout_of(typ).is_zst() {
                    return None;
                }
                debug!(?lc, ?typ, "codegen_declare_variables");
                let name = self.local_name(lc).clone();
                let boogie_type = self.codegen_type(typ);
                Some(Stmt::Decl { name, typ: boogie_type })
            })
            .collect();
        decls
    }

    fn codegen_type(&self, ty: Ty<'tcx>) -> Type {
        trace!(typ=?ty, "codegen_type");
        match ty.kind() {
            ty::Bool => Type::Bool,
            ty::Int(_ity) => Type::Int, // TODO: use Bv
            _ => todo!(),
        }
    }

    fn codegen_body(&self) -> Stmt {
        let statements: Vec<Stmt> =
            reverse_postorder(self.mir).map(|(bb, bbd)| self.codegen_block(bb, bbd)).collect();
        Stmt::Block { statements }
    }

    fn codegen_block(&self, bb: BasicBlock, bbd: &BasicBlockData<'tcx>) -> Stmt {
        debug!(?bb, ?bbd, "codegen_block");
        // the first statement should be labelled. if there is no statements, then the
        // terminator should be labelled.
        let statements = match bbd.statements.len() {
            0 => {
                let term = bbd.terminator();
                let tcode = self.codegen_terminator(term);
                vec![tcode]
            }
            _ => {
                let mut statements: Vec<Stmt> =
                    bbd.statements.iter().map(|stmt| self.codegen_statement(stmt)).collect();

                let term = self.codegen_terminator(bbd.terminator());
                statements.push(term);
                statements
            }
        };
        Stmt::Block { statements }
    }

    fn codegen_statement(&self, stmt: &Statement<'tcx>) -> Stmt {
        match &stmt.kind {
            StatementKind::Assign(box (place, rvalue)) => {
                debug!(?place, ?rvalue, "codegen_statement");
                let rv = self.codegen_rvalue(rvalue);
                let place_name = self.local_name(place.local).clone();
                // assignment statement
                let asgn = Stmt::Assignment { target: place_name, value: rv.1 };
                // add it to other statements generated while creating the rvalue (if any)
                add_statement(rv.0, asgn)
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

    /// Codegen an rvalue. Returns the expression for the rvalue and an optional
    /// statement for any possible checks instrumented for the rvalue expression
    fn codegen_rvalue(&self, rvalue: &Rvalue<'tcx>) -> (Option<Stmt>, Expr) {
        debug!(rvalue=?rvalue, "codegen_rvalue");
        match rvalue {
            Rvalue::Use(operand) => (None, self.codegen_operand(operand)),
            Rvalue::BinaryOp(binop, box (lhs, rhs)) => self.codegen_binary_op(binop, lhs, rhs),
            _ => todo!(),
        }
    }

    fn codegen_binary_op(
        &self,
        binop: &BinOp,
        lhs: &Operand<'tcx>,
        rhs: &Operand<'tcx>,
    ) -> (Option<Stmt>, Expr) {
        let expr = match binop {
            BinOp::Eq => Expr::BinaryOp {
                op: BinaryOp::Eq,
                left: Box::new(self.codegen_operand(lhs)),
                right: Box::new(self.codegen_operand(rhs)),
            },
            _ => todo!(),
        };
        (None, expr)
    }

    fn codegen_terminator(&self, term: &Terminator<'tcx>) -> Stmt {
        let _trace_span = debug_span!("CodegenTerminator", statement = ?term.kind).entered();
        debug!("handling terminator {:?}", term);
        match &term.kind {
            TerminatorKind::Call { func, args, destination, target, .. } => {
                self.codegen_funcall(func, args, destination, target, term.source_info.span)
            }
            TerminatorKind::Return => Stmt::Return,
            _ => todo!(),
        }
    }

    fn codegen_funcall(
        &self,
        func: &Operand<'tcx>,
        args: &[Operand<'tcx>],
        destination: &Place<'tcx>,
        target: &Option<BasicBlock>,
        span: Span,
    ) -> Stmt {
        debug!(?func, ?args, ?destination, ?span, "codegen_funcall");
        let fargs = self.codegen_funcall_args(args);
        let funct = self.operand_ty(func);
        // TODO: Only Kani intrinsics are handled currently
        match &funct.kind() {
            ty::FnDef(defid, substs) => {
                let instance = Instance::expect_resolve(
                    self.tcx(),
                    ty::ParamEnv::reveal_all(),
                    *defid,
                    substs,
                );

                if let Some(intrinsic) = get_kani_intrinsic(self.tcx(), instance) {
                    return self.codegen_kani_intrinsic(
                        intrinsic,
                        instance,
                        fargs,
                        *destination,
                        *target,
                        Some(span),
                    );
                }
                todo!()
            }
            _ => todo!(),
        }
    }

    fn codegen_funcall_args(&self, args: &[Operand<'tcx>]) -> Vec<Expr> {
        debug!(?args, "codegen_funcall_args");
        args.iter()
            .filter_map(|o| {
                let ty = self.operand_ty(o);
                // TODO: handle non-primitive types
                if ty.is_primitive() {
                    return Some(self.codegen_operand(o));
                }
                // TODO: ignore non-primitive arguments for now (e.g. `msg`
                // argument of `kani::assert`)
                None
            })
            .collect()
    }

    fn codegen_operand(&self, o: &Operand<'tcx>) -> Expr {
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

    fn codegen_place(&self, place: &Place<'tcx>) -> Expr {
        debug!(place=?place, "codegen_place");
        debug!(place.local=?place.local, "codegen_place");
        debug!(place.projection=?place.projection, "codegen_place");
        self.codegen_local(place.local)
    }

    fn codegen_local(&self, local: Local) -> Expr {
        // TODO: handle function definitions
        Expr::Symbol { name: self.local_name(local).clone() }
    }

    fn local_name(&self, local: Local) -> &String {
        &self.local_names[&local]
    }

    fn codegen_constant(&self, c: &ConstOperand<'tcx>) -> Expr {
        trace!(constant=?c, "codegen_constant");
        // TODO: monomorphize
        match c.const_ {
            mirConst::Val(val, ty) => self.codegen_constant_value(val, ty),
            _ => todo!(),
        }
    }

    fn codegen_constant_value(&self, val: ConstValue<'tcx>, ty: Ty<'tcx>) -> Expr {
        debug!(val=?val, "codegen_constant_value");
        match val {
            ConstValue::Scalar(s) => self.codegen_scalar(s, ty),
            _ => todo!(),
        }
    }

    fn codegen_scalar(&self, s: Scalar, ty: Ty<'tcx>) -> Expr {
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

    fn operand_ty(&self, o: &Operand<'tcx>) -> Ty<'tcx> {
        // TODO: monomorphize
        o.ty(self.mir.local_decls(), self.bcx.tcx)
    }
}

impl<'a, 'tcx> LayoutOfHelpers<'tcx> for FunctionCtx<'a, 'tcx> {
    type LayoutOfResult = TyAndLayout<'tcx>;

    fn handle_layout_err(&self, err: LayoutError<'tcx>, span: Span, ty: Ty<'tcx>) -> ! {
        span_bug!(span, "failed to get layout for `{}`: {}", ty, err)
    }
}

impl<'a, 'tcx> HasParamEnv<'tcx> for FunctionCtx<'a, 'tcx> {
    fn param_env(&self) -> ty::ParamEnv<'tcx> {
        ty::ParamEnv::reveal_all()
    }
}

impl<'a, 'tcx> HasTyCtxt<'tcx> for FunctionCtx<'a, 'tcx> {
    fn tcx(&self) -> TyCtxt<'tcx> {
        self.bcx.tcx
    }
}

impl<'a, 'tcx> HasDataLayout for FunctionCtx<'a, 'tcx> {
    fn data_layout(&self) -> &TargetDataLayout {
        self.bcx.tcx.data_layout()
    }
}

/// Create a new statement that includes `s1` (if non-empty) and `s2`
fn add_statement(s1: Option<Stmt>, s2: Stmt) -> Stmt {
    match s1 {
        Some(s1) => match s1 {
            Stmt::Block { mut statements } => {
                statements.push(s2);
                Stmt::Block { statements }
            }
            _ => Stmt::Block { statements: vec![s1, s2] },
        },
        None => s2,
    }
}
