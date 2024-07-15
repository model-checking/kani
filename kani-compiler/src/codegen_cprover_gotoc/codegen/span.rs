// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! MIR Span related functions

use crate::codegen_cprover_gotoc::GotocCtx;
use cbmc::goto_program::Location;
use rustc_smir::rustc_internal;
use rustc_span::Span;
use stable_mir::ty::Span as SpanStable;

/// Pragma to prevent CBMC from generating automatic pointer checks.
const DISABLE_PTR_CHECK_PRAGMA: &str = "disable:pointer-check";

impl<'tcx> GotocCtx<'tcx> {
    pub fn codegen_span(&self, sp: &Span) -> Location {
        self.codegen_span_stable(rustc_internal::stable(sp))
    }

    pub fn codegen_span_stable(&self, sp: SpanStable) -> Location {
        // Attribute to mark functions as where automatic pointer checks should not be generated.
        let should_skip_ptr_checks_attr = vec![
            rustc_span::symbol::Symbol::intern("kanitool"),
            rustc_span::symbol::Symbol::intern("skip_ptr_checks"),
        ];
        let pragmas: &[&str] = {
            let should_skip_ptr_checks = self
                .current_fn
                .as_ref()
                .map(|current_fn| {
                    let instance = current_fn.instance();
                    self.tcx
                        .has_attrs_with_path(instance.def.def_id(), &should_skip_ptr_checks_attr)
                })
                .unwrap_or(false);
            if should_skip_ptr_checks { &[DISABLE_PTR_CHECK_PRAGMA] } else { &[] }
        };
        let loc = sp.get_lines();
        Location::new(
            sp.get_filename().to_string(),
            self.current_fn.as_ref().map(|x| x.readable_name().to_string()),
            loc.start_line,
            Some(loc.start_col),
            loc.end_line,
            Some(loc.end_col),
            pragmas,
        )
    }

    pub fn codegen_caller_span_stable(&self, sp: SpanStable) -> Location {
        self.codegen_caller_span(&rustc_internal::internal(self.tcx, sp))
    }

    /// Get the location of the caller. This will attempt to reach the macro caller.
    /// This function uses rustc_span methods designed to returns span for the macro which
    /// originally caused the expansion to happen.
    /// Note: The API stops backtracing at include! boundary.
    pub fn codegen_caller_span(&self, span: &Span) -> Location {
        let topmost = span.ctxt().outer_expn().expansion_cause().unwrap_or(*span);
        self.codegen_span(&topmost)
    }
}
