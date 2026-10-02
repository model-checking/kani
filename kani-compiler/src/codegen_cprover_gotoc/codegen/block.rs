// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use crate::codegen_cprover_gotoc::GotocCtx;
use rustc_public::mir::{BasicBlock, BasicBlockIdx, Body};
use std::collections::{HashMap, HashSet};
use tracing::debug;

pub fn bb_label(bb: BasicBlockIdx) -> String {
    format!("bb{bb}")
}

impl GotocCtx<'_, '_> {
    /// Generates Goto-C for a basic block.
    ///
    /// A MIR basic block consists of 0 or more statements followed by a terminator.
    ///
    /// This function does not return a value, but mutates state with
    /// `self.current_fn_mut().push_onto_block(...)`
    pub fn codegen_block(&mut self, bb: BasicBlockIdx, bbd: &BasicBlock) {
        debug!(?bb, "codegen_block");
        let clauses = self.current_fn_mut().loop_decreases_mut();
        if clauses.loop_heads.contains(&bb)
            && let Some(decreases) = clauses.pending.take()
        {
            // The binding of a decreases clause is right before its loop, and blocks are
            // generated in reverse postorder, so the clause belongs to the first loop head that
            // is generated after its binding. `LoopInvariantRegister` takes it from there.
            clauses.by_head.insert(bb, decreases);
        }
        let label = bb_label(bb);
        // the first statement should be labelled. if there is no statements, then the
        // terminator should be labelled.
        match bbd.statements.len() {
            0 => {
                let term = &bbd.terminator;
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
                let term = &bbd.terminator;
                let tcode = self.codegen_terminator(term);
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

/// The loop heads of `body`, i.e. the targets of its back edges, given its reachable basic
/// blocks in reverse postorder.
pub fn loop_heads(body: &Body, reverse_postorder: &[BasicBlockIdx]) -> HashSet<BasicBlockIdx> {
    let position: HashMap<BasicBlockIdx, usize> =
        reverse_postorder.iter().enumerate().map(|(position, bb)| (*bb, position)).collect();
    reverse_postorder
        .iter()
        .flat_map(|bb| {
            body.blocks[*bb]
                .terminator
                .successors()
                .into_iter()
                .filter(|succ| position[succ] <= position[bb])
                .collect::<Vec<_>>()
        })
        .collect()
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
