#![cfg(feature = "unsound_experiments")]
use crate::codegen_cprover_gotoc::GotocCtx;
use crate::unwrap_or_return_codegen_unimplemented_stmt;
use cbmc::goto_program::{Expr, Location, Stmt};
use kani_queries::UserInput;
use rustc_middle::mir::Place;
use rustc_middle::ty::layout::LayoutOf;

impl<'tcx> GotocCtx<'tcx> {
    /// Codegens the an initalizer for variables without one.
    /// If the zero initilizer flag is set, does zero init (if possible).
    /// Otherwise, returns `None` which leaves the variable uninitilized.
    /// In CBMC, this translates to a NONDET value.
    pub fn codegen_default_initializer(&mut self, e: &Expr) -> Option<Expr> {
        if self.queries.get_unsound_experiments().lock().unwrap().zero_init_vars {
            Some(e.typ().zero_initializer(&self.symbol_table))
        } else {
            None
        }
    }

    /// From rustc doc: "This writes `uninit` bytes to the entire place."
    /// Our model of GotoC has a similar statement, which is later lowered
    /// to assigning a Nondet in CBMC, with a comment specifying that it
    /// corresponds to a Deinit.
    pub fn codegen_deinit(&mut self, place: &Place<'tcx>, loc: Location) -> Stmt {
        let dst_mir_ty = self.place_ty(place);
        let dst_type = self.codegen_ty(dst_mir_ty);
        let layout = self.layout_of(dst_mir_ty);
        let goto_place =
            unwrap_or_return_codegen_unimplemented_stmt!(self, self.codegen_place(place)).goto_expr;
        if layout.is_zst() || dst_type.sizeof_in_bits(&self.symbol_table) == 0 {
            // We ignore assignment for all zero size types
            Stmt::skip(loc)
        } else if self.queries.get_unsound_experiments().lock().unwrap().zero_init_vars {
            let init = goto_place.typ().zero_initializer(&self.symbol_table);
            goto_place.assign(init, loc)
        } else {
            goto_place.deinit(loc)
        }
    }
}
