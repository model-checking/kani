// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use crate::codegen_cprover_gotoc::GotocCtx;
use cbmc::goto_program::{Stmt, Expr};
use rustc_middle::mir::{BasicBlock, BasicBlockData};
use rustc_span::Span;
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
                let span = term.source_info.span;
                // push cover stmt onto block with label
                // remove the next label
                let cover = self.cover_with_span(span);
                self.current_fn_mut().push_onto_block(cover.with_label(label));
                let tcode = self.codegen_terminator(term);
                self.current_fn_mut().push_onto_block(tcode);
            }
            _ => {
                let stmt = &bbd.statements[0];
                
                let span = stmt.source_info.span;
                let cover = self.cover_with_span(span);
                self.current_fn_mut().push_onto_block(cover.with_label(label));
                let scode = self.codegen_statement(stmt);
                self.current_fn_mut().push_onto_block(scode);

                for s in &bbd.statements[1..] {
                    let span = s.source_info.span;
                    // push cover statements
                    let cover = self.cover_with_span(span);
                    self.current_fn_mut().push_onto_block(cover);
                    let stmt = self.codegen_statement(s);
                    self.current_fn_mut().push_onto_block(stmt);
                }
                // push cover statements
                // test with division by zero or overflow
                // that false assumption will be in the middle of the basic block
                let term = bbd.terminator();
                let span = term.source_info.span;
                // push cover stmt onto block with label
                // remove the next label
                let cover = self.cover_with_span(span);
                self.current_fn_mut().push_onto_block(cover);
                let tcode = self.codegen_terminator(term);
                self.current_fn_mut().push_onto_block(tcode);
            }
        }
        self.current_fn_mut().reset_current_bb();
    }
    fn cover_with_span(&mut self, span: Span) -> Stmt {
            self.codegen_cover(Expr::c_true(), "cover_experiment", Some(span))
    }
}
