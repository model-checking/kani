use super::hooks::GotocHook;
use crate::codegen_cprover_gotoc::codegen::bb_label;
use crate::codegen_cprover_gotoc::GotocCtx;
use crate::kani_middle::attributes::matches_diagnostic as matches_function;
use cbmc::goto_program::{Expr, Stmt};
use rustc_middle::ty::TyCtxt;
use stable_mir::mir::mono::Instance;
use stable_mir::mir::{BasicBlockIdx, Place};
use stable_mir::ty::Span;

pub struct LoopInvariantBegin;

impl GotocHook for LoopInvariantBegin {
    fn hook_applies(&self, tcx: TyCtxt, instance: Instance) -> bool {
        matches_function(tcx, instance.def, "KaniLoopInvariantBegin")
    }

    fn handle(
        &self,
        gcx: &mut GotocCtx,
        _instance: Instance,
        fargs: Vec<Expr>,
        _assign_to: &Place,
        target: Option<BasicBlockIdx>,
        span: Span,
    ) -> Stmt {
        assert_eq!(fargs.len(), 0);
        let loc = gcx.codegen_span_stable(span);

        // Start to record loop invariant statement
        gcx.loop_contracts_ctx.enter_loop_invariant_block();

        Stmt::goto(bb_label(target.unwrap()), loc)
    }
}

pub struct LoopInvariantEnd;

impl GotocHook for LoopInvariantEnd {
    fn hook_applies(&self, tcx: TyCtxt, instance: Instance) -> bool {
        matches_function(tcx, instance.def, "KaniLoopInvariantEnd")
    }

    fn handle(
        &self,
        gcx: &mut GotocCtx,
        _instance: Instance,
        fargs: Vec<Expr>,
        _assign_to: &Place,
        target: Option<BasicBlockIdx>,
        span: Span,
    ) -> Stmt {
        assert_eq!(fargs.len(), 0);
        let loc = gcx.codegen_span_stable(span);

        // Stop to record loop invariant statement
        gcx.loop_contracts_ctx.exit_loop_invariant_block();

        Stmt::goto(bb_label(target.unwrap()), loc)
    }
}
