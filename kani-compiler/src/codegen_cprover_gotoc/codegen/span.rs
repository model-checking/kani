// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! MIR Span related functions

use crate::codegen_cprover_gotoc::GotocCtx;
use cbmc::goto_program::Location;
use rustc_middle::mir::{Local, VarDebugInfo, VarDebugInfoContents};
use rustc_smir::rustc_internal;
use rustc_span::Span;

impl<'tcx> GotocCtx<'tcx> {
    pub fn codegen_span(&self, sp: &Span) -> Location {
        self.stable_codegen_span(&rustc_internal::stable(sp))
    }

    pub fn stable_codegen_span(&self, sp: &stable_mir::ty::Span) -> Location {
        let loc = sp.get_lines();
        Location::new(
            sp.get_filename().to_string(),
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
