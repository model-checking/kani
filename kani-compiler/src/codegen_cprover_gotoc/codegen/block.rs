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

        // record the seen bbidx if loop contracts enabled
        if self.loop_contracts_ctx.loop_contracts_enabled() {
            self.loop_contracts_ctx.add_new_seen_bbidx(bb);
        }

        // the first statement should be labelled. if there is no statements, then the
        // terminator should be labelled.
        match bbd.statements.len() {
            0 => {
                let term = &bbd.terminator;
                let tcode = if self.loop_contracts_ctx.loop_contracts_enabled() {
                    let codegen_result = self.codegen_terminator(term);
                    self.loop_contracts_ctx.push_onto_block(codegen_result)
                } else {
                    self.codegen_terminator(term)
                };

                self.current_fn_mut().push_onto_block(tcode.with_label(label));
            }
            _ => {
                let stmt = &bbd.statements[0];
                let scode = if self.loop_contracts_ctx.loop_contracts_enabled() {
                    let codegen_result = self.codegen_statement(stmt);
                    self.loop_contracts_ctx.push_onto_block(codegen_result)
                } else {
                    self.codegen_statement(stmt)
                };

                self.current_fn_mut().push_onto_block(scode.with_label(label));

                for s in &bbd.statements[1..] {
                    let stmt = if self.loop_contracts_ctx.loop_contracts_enabled() {
                        let codegen_result = self.codegen_statement(s);
                        self.loop_contracts_ctx.push_onto_block(codegen_result)
                    } else {
                        self.codegen_statement(s)
                    };
                    self.current_fn_mut().push_onto_block(stmt);
                }
                let term = &bbd.terminator;

                let tcode = if self.loop_contracts_ctx.loop_contracts_enabled() {
                    let codegen_result = self.codegen_terminator(term);
                    self.loop_contracts_ctx.push_onto_block(codegen_result)
                } else {
                    self.codegen_terminator(term)
                };

                self.current_fn_mut().push_onto_block(tcode);
            }
        }
    }
}

/// Iterate over the basic blocks in reverse post-order.
///
/// The `reverse_postorder` function used before was internal to the compiler and reflected the
/// internal body representation.
///
/// As we introduce transformations on the top of SMIR body, there will be not guarantee of a
/// 1:1 relationship between basic blocks from internal body and monomorphic body from StableMIR.
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
