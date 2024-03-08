// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use crate::codegen_cprover_gotoc::GotocCtx;
use stable_mir::mir::{BasicBlock, BasicBlockIdx, Body};
use std::collections::HashSet;
use tracing::debug;

pub fn bb_label(bb: BasicBlockIdx) -> String {
    format!("bb{bb}")
}

impl<'tcx> GotocCtx<'tcx> {
    /// Generates Goto-C for a basic block.
    ///
    /// A MIR basic block consists of 0 or more statements followed by a terminator.
    ///
    /// This function does not return a value, but mutates state with
    /// `self.current_fn_mut().push_onto_block(...)`
    pub fn codegen_block(&mut self, bb: BasicBlockIdx, bbd: &BasicBlock) {
        debug!(?bb, "codegen_block");
        let label = bb_label(bb);
        let check_coverage = self.queries.args().check_coverage;
        // the first statement should be labelled. if there is no statements, then the
        // terminator should be labelled.
        match bbd.statements.len() {
            0 => {
                let term = &bbd.terminator;
                let tcode = self.codegen_terminator(term);
                // When checking coverage, the `coverage` check should be
                // labelled instead.
                if check_coverage {
                    let span = term.span;
                    let cover = self.codegen_coverage(span);
                    self.current_fn_mut().push_onto_block(cover.with_label(label));
                    self.current_fn_mut().push_onto_block(tcode);
                } else {
                    self.current_fn_mut().push_onto_block(tcode.with_label(label));
                }
            }
            _ => {
                let stmt = &bbd.statements[0];
                let scode = self.codegen_statement(stmt);
                // When checking coverage, the `coverage` check should be
                // labelled instead.
                if check_coverage {
                    let span = stmt.span;
                    let cover = self.codegen_coverage(span);
                    self.current_fn_mut().push_onto_block(cover.with_label(label));
                    self.current_fn_mut().push_onto_block(scode);
                } else {
                    self.current_fn_mut().push_onto_block(scode.with_label(label));
                }

                for s in &bbd.statements[1..] {
                    if check_coverage {
                        let span = s.span;
                        let cover = self.codegen_coverage(span);
                        self.current_fn_mut().push_onto_block(cover);
                    }
                    let stmt = self.codegen_statement(s);
                    self.current_fn_mut().push_onto_block(stmt);
                }
                let term = &bbd.terminator;
                if check_coverage {
                    let span = term.span;
                    let cover = self.codegen_coverage(span);
                    self.current_fn_mut().push_onto_block(cover);
                }
                let tcode = self.codegen_terminator(term);
                self.current_fn_mut().push_onto_block(tcode);
            }
        }
    }
}

pub fn reverse_postorder(body: &Body) -> impl Iterator<Item = BasicBlockIdx> {
    postorder(body, 0, &mut HashSet::with_capacity(body.blocks.len())).into_iter().rev()
}

fn postorder(
    body: &Body,
    bb: BasicBlockIdx,
    visited: &mut HashSet<BasicBlockIdx>,
) -> Vec<BasicBlockIdx> {
    if visited.contains(&bb) {
        return vec![];
    }
    visited.insert(bb);

    let mut result = vec![];
    for succ in body.blocks[bb].terminator.successors() {
        result.append(&mut postorder(body, succ, visited));
    }
    result.push(bb);
    result
}
