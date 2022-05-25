// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This file contains the code that acts as a wrapper to create the new assert and related statements
use crate::codegen_cprover_gotoc::GotocCtx;
use cbmc::goto_program::{Expr, Location, Stmt};

/// The Property Class enum stores all viable options for classifying asserts, cover assume and other related statements
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum PropertyClass {
    ArithmeticOverflow,
    AssertFalse,
    Assume,
    Cover,
    CustomProperty(String),
    DefaultAssertion,
    ExactDiv,
    ExpectFail,
    FiniteCheck,
    /// Checks added by Kani compiler to detect UB or unstable behavior.
    KaniCheck,
    PointerOffset,
    SanityCheck,
    Unimplemented,
    UnsupportedConstruct,
    Unreachable,
}

#[allow(dead_code)]
impl PropertyClass {
    pub fn as_str(&self) -> &str {
        match self {
            PropertyClass::ArithmeticOverflow => "arithmetic_overflow",
            PropertyClass::AssertFalse => "assert_false",
            PropertyClass::Assume => "assume",
            PropertyClass::Cover => "coverage_check",
            PropertyClass::CustomProperty(property_string) => property_string.as_str(),
            PropertyClass::DefaultAssertion => "assertion",
            PropertyClass::ExactDiv => "exact_div",
            PropertyClass::ExpectFail => "expect_fail",
            PropertyClass::FiniteCheck => "finite_check",
            PropertyClass::KaniCheck => "kani_check",
            PropertyClass::PointerOffset => "pointer_offset",
            PropertyClass::SanityCheck => "sanity_check",
            PropertyClass::Unimplemented => "unimplemented",
            PropertyClass::Unreachable => "unreachable",
            PropertyClass::UnsupportedConstruct => "unsupported_construct",
        }
    }

    pub fn from_str(input: &str) -> PropertyClass {
        match input {
            "arithmetic_overflow" => PropertyClass::ArithmeticOverflow,
            "assert_false" => PropertyClass::AssertFalse,
            "assume" => PropertyClass::Assume,
            "assertion" => PropertyClass::DefaultAssertion,
            "coverage_check" => PropertyClass::Cover,
            "exact_div" => PropertyClass::ExactDiv,
            "expect_fail" => PropertyClass::ExpectFail,
            "finite_check" => PropertyClass::FiniteCheck,
            "kani_check" => PropertyClass::KaniCheck,
            "pointer_offset" => PropertyClass::PointerOffset,
            "sanity_check" => PropertyClass::SanityCheck,
            "unimplemented" => PropertyClass::Unimplemented,
            "unreachable" => PropertyClass::Unreachable,
            "unsupported_construct" => PropertyClass::UnsupportedConstruct,
            _ => PropertyClass::CustomProperty(input.to_owned()),
        }
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
