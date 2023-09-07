// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! MIR Span related functions

use crate::{codegen_cprover_gotoc::GotocCtx, kani_middle::SourceLocation};
use cbmc::goto_program::Location;
use rustc_middle::mir::{Local, VarDebugInfo, VarDebugInfoContents};
use rustc_span::Span;

impl<'tcx> GotocCtx<'tcx> {
    pub fn codegen_span(&self, sp: &Span) -> Location {
        let loc = SourceLocation::new(self.tcx, sp);
        Location::new(
            loc.filename,
            self.current_fn.as_ref().map(|x| x.readable_name().to_string()),
            loc.start_line,
            Some(loc.start_col),
            loc.end_line,
            Some(loc.end_col),
        )
    }

    /// Get the location of the caller. This will attempt to reach the macro caller.
    /// This function uses rustc_span methods designed to returns span for the macro which
    /// originally caused the expansion to happen.
    /// Note: The API stops backtracing at include! boundary.
    pub fn codegen_caller_span(&self, sp: &Option<Span>) -> Location {
        if let Some(span) = sp {
            let topmost = span.ctxt().outer_expn().expansion_cause().unwrap_or(*span);
            self.codegen_span(&topmost)
        } else {
            Location::none()
        }
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
