// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use crate::codegen_cprover_gotoc::GotocCtx;
use cbmc::goto_program::{Stmt, Expr};
use rustc_middle::mir::{BasicBlock, BasicBlockData, Statement, Terminator};
use tracing::debug;

impl<'tcx> GotocCtx<'tcx> {
    /// Generates Goto-C for a basic block.
    ///
    /// A MIR basic block consists of 0 or more statements followed by a terminator.
    ///
    /// This function does not return a value, but mutates state with
    /// `self.current_fn_mut().push_onto_block(...)`
    pub fn codegen_block(&mut self, bb: BasicBlock, bbd: &BasicBlockData<'tcx>) {
        debug!(?bb, "Codegen basicblock");
        self.current_fn_mut().set_current_bb(bb);
        let label: String = self.current_fn().find_label(&bb);
        // the first statement should be labelled. if there is no statements, then the
        // terminator should be labelled.
        match bbd.statements.len() {
            0 => {
                let term = bbd.terminator();
                let tcode = self.codegen_terminator(term);
                // let new_tcode = self.add_cover_term(tcode, term);
                self.current_fn_mut().push_onto_block(tcode.with_label(label));
            }
            _ => {
                let stmt = &bbd.statements[0];
                let scode = self.codegen_statement(stmt);
                let new_scode = self.add_cover_stmt(scode, stmt);
                self.current_fn_mut().push_onto_block(new_scode.with_label(label.clone()));

                for s in &bbd.statements[1..] {
                    let stmt = self.codegen_statement(s);
                    self.current_fn_mut().push_onto_block(stmt);
                }
                let term = bbd.terminator();
                let tcode = self.codegen_terminator(term);
                self.current_fn_mut().push_onto_block(tcode);
            }
        }
        self.current_fn_mut().reset_current_bb();
    }
    #[allow(dead_code)]
    fn add_cover_term(&mut self, stmt: Stmt, term: &Terminator<'tcx>) -> Stmt {
            let span = &term.source_info.span;
            let loc = self.codegen_span(&span);
            let stmts = vec![stmt];
            // let body = Stmt::block(stmts, loc);
            let cover = self.codegen_cover(Expr::c_true(), "cover_experiment", Some(*span));
            let mut new_stmts = stmts.clone();
            new_stmts.insert(0, cover);
            let body = Stmt::block(new_stmts, loc);
            body
    }

    fn add_cover_stmt(&mut self, stmt: Stmt, s: &Statement<'tcx>) -> Stmt {
        let span = &s.source_info.span;
        let loc = self.codegen_span(&span);
        let stmts = vec![stmt];
        // let body = Stmt::block(stmts, loc);
        let cover = self.codegen_cover(Expr::c_true(), "cover_experiment", Some(*span));
        let mut new_stmts = stmts.clone();
        new_stmts.insert(0, cover);
        let body = Stmt::block(new_stmts, loc);
        body
}
}
