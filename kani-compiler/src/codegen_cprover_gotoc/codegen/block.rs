// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use crate::codegen_cprover_gotoc::GotocCtx;
use rustc_middle::mir::{BasicBlock, BasicBlockData};
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
                self.current_fn_mut().push_onto_block(tcode.with_label(label));
            }
            _ => {
                let stmt = &bbd.statements[0];
                let scode = self.codegen_statement(stmt);
                self.current_fn_mut().push_onto_block(scode.with_label(label));

                for s in &bbd.statements[1..] {
                    let stmt = self.codegen_statement(s);
                    self.current_fn_mut().push_onto_block(stmt);
                }
                let term = self.codegen_terminator(bbd.terminator());
                self.current_fn_mut().push_onto_block(term);
            }
        }
        self.current_fn_mut().reset_current_bb();
    }
}
