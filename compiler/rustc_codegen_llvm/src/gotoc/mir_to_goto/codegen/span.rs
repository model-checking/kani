// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! MIR Span related functions

use crate::gotoc::cbmc::goto_program::Location;
use crate::gotoc::mir_to_goto::GotocCtx;
use rustc_middle::mir::{Local, VarDebugInfo, VarDebugInfoContents};
use rustc_span::Span;

impl<'tcx> GotocCtx<'tcx> {
    pub fn codegen_span(&self, sp: &Span) -> Location {
        let smap = self.tcx.sess.source_map();
        let lo = smap.lookup_char_pos(sp.lo());
        let line = lo.line;
        let col = 1 + lo.col_display;
        let filename0 = lo.file.name.prefer_remapped().to_string_lossy().to_string();
        let filename1 = match std::fs::canonicalize(filename0.clone()) {
            Ok(pathbuf) => pathbuf.to_str().unwrap().to_string(),
            Err(_) => filename0,
        };
        Location::new(
            filename1,
            self.current_fn.as_ref().map(|x| x.readable_name().to_string()),
            line,
            Some(col),
        )
    }

    pub fn codegen_span_option(&self, sp: Option<Span>) -> Location {
        sp.map_or(Location::none(), |x| self.codegen_span(&x))
    }

    pub fn find_debug_info(&self, l: &Local) -> Option<&VarDebugInfo<'tcx>> {
        self.current_fn().mir().var_debug_info.iter().find(|info| match info.value {
            VarDebugInfoContents::Place(p) => p.local == *l && p.projection.len() == 0,
            VarDebugInfoContents::Const(_) => false,
        })
    }
}
