// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! MIR Span related functions

use crate::codegen_cprover_gotoc::GotocCtx;
use cbmc::goto_program::Location;
use lazy_static::lazy_static;
use rustc_ast::Attribute;
use rustc_smir::rustc_internal;
use rustc_span::Span;
use stable_mir::ty::Span as SpanStable;
use std::collections::HashMap;

lazy_static! {
    /// Pragmas key-value store to prevent CBMC from generating automatic checks.
    /// This list is taken from https://github.com/diffblue/cbmc/blob/develop/regression/cbmc/pragma_cprover_enable_all/main.c.
    static ref PRAGMAS: HashMap<&'static str, &'static str> =
        [("bounds", "disable:bounds-check"),
         ("pointer", "disable:pointer-check"),
         ("div-by-zero", "disable:div-by-zero-check"),
         ("float-div-by-zero", "disable:float-div-by-zero-check"),
         ("enum-range", "disable:enum-range-check"),
         ("signed-overflow", "disable:signed-overflow-check"),
         ("unsigned-overflow", "disable:unsigned-overflow-check"),
         ("pointer-overflow", "disable:pointer-overflow-check"),
         ("float-overflow", "disable:float-overflow-check"),
         ("conversion", "disable:conversion-check"),
         ("undefined-shift", "disable:undefined-shift-check"),
         ("nan", "disable:nan-check"),
         ("pointer-primitive", "disable:pointer-primitive-check")].iter().copied().collect();
}

impl GotocCtx<'_> {
    pub fn codegen_span(&self, sp: &Span) -> Location {
        self.codegen_span_stable(rustc_internal::stable(sp))
    }

    pub fn codegen_span_stable(&self, sp: SpanStable) -> Location {
        // Attribute to mark functions as where automatic pointer checks should not be generated.
        let should_skip_ptr_checks_attr = vec![
            rustc_span::symbol::Symbol::intern("kanitool"),
            rustc_span::symbol::Symbol::intern("disable_checks"),
        ];
        let pragmas: &'static [&str] = {
            let disabled_checks: Vec<_> = self
                .current_fn
                .as_ref()
                .map(|current_fn| {
                    let instance = current_fn.instance();
                    self.tcx
                        .get_attrs_by_path(instance.def.def_id(), &should_skip_ptr_checks_attr)
                        .collect()
                })
                .unwrap_or_default();
            disabled_checks
                .iter()
                .map(|attr| {
                    let arg = parse_word(attr).expect(
                        "incorrect value passed to `disable_checks`, expected a single identifier",
                    );
                    *PRAGMAS.get(arg.as_str()).expect(format!(
                        "attempting to disable an unexisting check, the possible options are {:?}",
                        PRAGMAS.keys()
                    ).as_str())
                })
                .collect::<Vec<_>>()
                .leak() // This is to preserve `Location` being Copy, but could blow up the memory utilization of compiler. 
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

/// Extracts the single argument from the attribute provided as a string.
/// For example, `disable_checks(foo)` return `Some("foo")`
fn parse_word(attr: &Attribute) -> Option<String> {
    // Vector of meta items , that contain the arguments given the attribute
    let attr_args = attr.meta_item_list()?;
    // Only extracts one string ident as a string
    if attr_args.len() == 1 {
        attr_args[0].ident().map(|ident| ident.to_string())
    }
    // Return none if there are no attributes or if there's too many attributes
    else {
        None
    }
}
