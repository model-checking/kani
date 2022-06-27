// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This file contains the code that acts as a wrapper to create the new assert and related statements
use crate::codegen_cprover_gotoc::GotocCtx;
use cbmc::goto_program::{Expr, Location, Stmt};
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
}
