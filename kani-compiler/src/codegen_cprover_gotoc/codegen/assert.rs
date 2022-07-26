// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This module is the central location for handling assertions and assumptions in Kani.

use crate::codegen_cprover_gotoc::utils;
use crate::codegen_cprover_gotoc::GotocCtx;
use cbmc::goto_program::{BuiltinFn, Expr, Location, Stmt};
use rustc_span::Span;
use std::convert::AsRef;
use strum_macros::{AsRefStr, EnumString};

/// The Property Class enum stores all viable options for classifying asserts, cover assume and other related statements
#[derive(Debug, Clone, EnumString, AsRefStr)]
#[strum(serialize_all = "snake_case")]
pub enum PropertyClass {
    ArithmeticOverflow,
    Assume,
    Cover,
    /// Assertions and Panic that are not specific to Kani. In a concrete execution, we expect
    /// these assertions to be available to the user.
    /// E.g.: User assertions, compiler invariant checks
    Assertion,
    ExactDiv,
    ExpectFail,
    FiniteCheck,
    /// Checks added by Kani compiler to detect safety conditions violation.
    /// E.g., things that trigger UB or unstable behavior.
    SafetyCheck,
    /// Checks to ensure that Kani's code generation is correct.
    SanityCheck,
    Unimplemented,
    UnsupportedConstruct,
    Unreachable,
}

#[allow(dead_code)]
impl PropertyClass {
    pub fn as_str(&self) -> &str {
        self.as_ref()
    }
}

impl<'tcx> GotocCtx<'tcx> {
    /// Generates a CBMC assertion. Note: Does _NOT_ assume.
    pub fn codegen_assert(
        &self,
        cond: Expr,
        property_class: PropertyClass,
        message: &str,
        loc: Location,
    ) -> Stmt {
        assert!(cond.typ().is_bool());
        let property_name = property_class.as_str();
        Stmt::assert(cond, property_name, message, loc)
    }

    /// Generates a CBMC assertion, followed by an assumption of the same condition.
    pub fn codegen_assert_assume(
        &self,
        cond: Expr,
        property_class: PropertyClass,
        message: &str,
        loc: Location,
    ) -> Stmt {
        assert!(cond.typ().is_bool());
        let property_name = property_class.as_str();
        Stmt::block(
            vec![Stmt::assert(cond.clone(), property_name, message, loc), Stmt::assume(cond, loc)],
            loc,
        )
    }

    /// A shorthand for generating a CBMC assert-false. TODO: This should probably be eliminated!
    pub fn codegen_assert_false(
        &self,
        property_class: PropertyClass,
        message: &str,
        loc: Location,
    ) -> Stmt {
        // Convert Property Class to String
        let property_name = property_class.as_str();
        Stmt::assert_false(property_name, message, loc)
    }

    /// Kani hooks function calls to `panic` and calls this intead.
    pub fn codegen_panic(&self, span: Option<Span>, fargs: Vec<Expr>) -> Stmt {
        // CBMC requires that the argument to the assertion must be a string constant.
        // If there is one in the MIR, use it; otherwise, explain that we can't.
        assert!(!fargs.is_empty(), "Panic requires a string message");
        let msg = utils::extract_const_message(&fargs[0]).unwrap_or(String::from(
            "This is a placeholder message; Kani doesn't support message formatted at runtime",
        ));

        self.codegen_fatal_error(PropertyClass::Assertion, &msg, span)
    }

    /// Generate code for fatal error which should trigger an assertion failure and abort the
    /// execution.
    pub fn codegen_fatal_error(
        &self,
        property_class: PropertyClass,
        msg: &str,
        span: Option<Span>,
    ) -> Stmt {
        let loc = self.codegen_caller_span(&span);
        Stmt::block(
            vec![
                self.codegen_assert_false(property_class, msg, loc),
                BuiltinFn::Abort.call(vec![], loc).as_stmt(loc),
            ],
            loc,
        )
    }

    /// Generate code to cover the given condition at the current location
    pub fn codegen_cover(&self, cond: Expr, msg: &str, span: Option<Span>) -> Stmt {
        let loc = self.codegen_caller_span(&span);
        // Should use Stmt::cover, but currently this doesn't work with CBMC
        // unless it is run with '--cover cover' (see
        // https://github.com/diffblue/cbmc/issues/6613). So for now use
        // assert(!cond).
        self.codegen_assert(cond.not(), PropertyClass::Cover, msg, loc)
    }

    /// Generate code to cover the current location
    pub fn codegen_cover_loc(&self, msg: &str, span: Option<Span>) -> Stmt {
        self.codegen_cover(Expr::bool_true(), msg, span)
    }
}
