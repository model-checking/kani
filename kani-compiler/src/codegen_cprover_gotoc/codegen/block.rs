// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This file contains functions related to codegenning MIR blocks into gotoc

use crate::codegen_cprover_gotoc::GotocCtx;
use rustc_middle::mir::{BasicBlock, BasicBlockData};

impl<'tcx> GotocCtx<'tcx> {
    pub fn codegen_block(&mut self, bb: BasicBlock, bbd: &BasicBlockData<'tcx>) {
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
