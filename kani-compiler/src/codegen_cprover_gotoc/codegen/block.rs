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
        let check_coverage = self.queries.check_coverage;
        match bbd.statements.len() {
            0 => {
                let term = bbd.terminator();
                if check_coverage {
                    let span = term.source_info.span;
                    let cover = self.codegen_coverage(span);
                    self.current_fn_mut().push_onto_block(cover.with_label(label));
                    let tcode = self.codegen_terminator(term);
                    self.current_fn_mut().push_onto_block(tcode);
                } else {
                    let tcode = self.codegen_terminator(term);
                    self.current_fn_mut().push_onto_block(tcode.with_label(label));
                }
            }
            _ => {
                let stmt = &bbd.statements[0];
                if check_coverage {
                    let span = stmt.source_info.span;
                    let cover = self.codegen_coverage(span);
                    self.current_fn_mut().push_onto_block(cover.with_label(label));
                    let scode = self.codegen_statement(stmt);
                    self.current_fn_mut().push_onto_block(scode);
                } else {
                    let scode = self.codegen_statement(stmt);
                    self.current_fn_mut().push_onto_block(scode.with_label(label));
                }

                for s in &bbd.statements[1..] {
                    if check_coverage {
                        let span = s.source_info.span;
                        // TODO: Push cover statement based on some TBD condition
                        let cover = self.codegen_coverage(span);
                        self.current_fn_mut().push_onto_block(cover);
                    }
                    let stmt = self.codegen_statement(s);
                    self.current_fn_mut().push_onto_block(stmt);
                }
                // TODO: test with division by zero or overflow that false
                // assumption will be in the middle of the basic block
                let term = bbd.terminator();
                if check_coverage {
                    let span = term.source_info.span;
                    let cover = self.codegen_coverage(span);
                    self.current_fn_mut().push_onto_block(cover);
                }
                let tcode = self.codegen_terminator(term);
                self.current_fn_mut().push_onto_block(tcode);
            }
        }
        self.current_fn_mut().reset_current_bb();
    }
}
