// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use super::hooks::GotocHook;
use crate::codegen_cprover_gotoc::codegen::bb_label;
use crate::codegen_cprover_gotoc::GotocCtx;
use crate::kani_middle::attributes::KaniAttributes;
use cbmc::goto_program::{CIntType, Expr, Stmt, Type};
use rustc_middle::ty::TyCtxt;
use rustc_span::Symbol;
use stable_mir::mir::mono::Instance;
use stable_mir::mir::{BasicBlockIdx, Place};
use stable_mir::ty::Span;

pub struct LoopInvariantRegister;

impl GotocHook for LoopInvariantRegister {
    fn hook_applies(&self, tcx: TyCtxt, instance: Instance) -> bool {
        KaniAttributes::for_instance(tcx, instance).fn_marker()
            == Some(Symbol::intern("kani_register_loop_contract"))
    }

    fn handle(
        &self,
        gcx: &mut GotocCtx,
        instance: Instance,
        fargs: Vec<Expr>,
        _assign_to: &Place,
        target: Option<BasicBlockIdx>,
        span: Span,
    ) -> Stmt {
        let loc = gcx.codegen_span_stable(span);
        let func_exp = gcx.codegen_func_expr(instance, loc);
        Stmt::goto(bb_label(target.unwrap()), loc)
            .with_loop_contracts(func_exp.call(fargs).cast_to(Type::CInteger(CIntType::Bool)))
    }
}
