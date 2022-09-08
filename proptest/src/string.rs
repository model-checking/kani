//-
// Copyright 2017 Jason Lingle
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Modifications Copyright Kani Contributors
// See GitHub history for details.

//! Strategies for generating strings and byte strings from regular
//! expressions.

use crate::std_facade::{Box, Cow, String, ToOwned, Vec};
use core::fmt;
use core::mem;
use core::ops::RangeInclusive;
use core::u32;

// use regex_syntax::hir::{
//     self, Hir,
//     HirKind::*,
//     Literal::*,
//     RepetitionKind::{self, *},
//     RepetitionRange::*,
// };
// use regex_syntax::{Error as ParseError, Parser};

use crate::bool;
use crate::char;
use crate::collection::{size_range, vec, SizeRange};
use crate::strategy::*;
use crate::test_runner::*;

/// Wraps the regex that forms the `Strategy` for `String` so that a sensible
/// `Default` can be given. The default is a string of non-control characters.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StringParam(&'static str);

impl From<StringParam> for &'static str {
    fn from(x: StringParam) -> Self {
        x.0
    }
}

impl From<&'static str> for StringParam {
    fn from(x: &'static str) -> Self {
        StringParam(x)
    }
}

impl Default for StringParam {
    fn default() -> Self {
        StringParam("\\PC*")
    }
}

// quick_error! uses bare trait objects, so we enclose its invocation here in a
// module so the lint can be disabled just for it.
#[allow(bare_trait_objects)]
mod error_container {
    use super::*;

    quick_error! {
        /// Errors which may occur when preparing a regular expression for use with
        /// string generation.
        #[derive(Debug)]
        pub enum Error {
            /// The regex was syntactically valid, but contains elements not
            /// supported by proptest.
            UnsupportedRegex(message: &'static str) {
                display("{}", message)
            }
        }
    }
}

pub use self::error_container::Error;

opaque_strategy_wrapper! {
    /// Strategy which generates values (i.e., `String` or `Vec<u8>`) matching
    /// a regular expression.
    ///
    /// Created by various functions in this module.
    #[derive(Debug)]
    pub struct RegexGeneratorStrategy[<T>][where T : fmt::Debug]
        (SBoxedStrategy<T>) -> RegexGeneratorValueTree<T>;
    /// `ValueTree` corresponding to `RegexGeneratorStrategy`.
    pub struct RegexGeneratorValueTree[<T>][where T : fmt::Debug]
        (Box<dyn ValueTree<Value = T>>) -> T;
}

impl Strategy for str {
    type Tree = RegexGeneratorValueTree<String>;
    type Value = String;

    fn new_tree(&self, runner: &mut TestRunner) -> NewTree<Self> {
        string_regex(self).unwrap().new_tree(runner)
    }
}

type ParseResult<T> = Result<RegexGeneratorStrategy<T>, Error>;

#[doc(hidden)]
/// A type which knows how to produce a `Strategy` from a regular expression
/// generating the type.
///
/// This trait exists for the benefit of `#[proptest(regex = "...")]`.
/// It is semver exempt, so use at your own risk.
/// If you found a use for the trait beyond `Vec<u8>` and `String`,
/// please file an issue at https://github.com/AltSysrq/proptest.
pub trait StrategyFromRegex: Sized + fmt::Debug {
    type Strategy: Strategy<Value = Self>;

    /// Produce a strategy for `Self` from the `regex`.
    fn from_regex(regex: &str) -> Self::Strategy;
}

impl StrategyFromRegex for String {
    type Strategy = RegexGeneratorStrategy<Self>;

    fn from_regex(regex: &str) -> Self::Strategy {
        string_regex(regex).unwrap()
    }
}

impl StrategyFromRegex for Vec<u8> {
    type Strategy = RegexGeneratorStrategy<Self>;

    fn from_regex(regex: &str) -> Self::Strategy {
        bytes_regex(regex).unwrap()
    }
}

/// Creates a strategy which generates strings matching the given regular
/// expression.
///
/// If you don't need error handling and aren't limited by setup time, it is
/// also possible to directly use a `&str` as a strategy with the same effect.
pub fn string_regex(_: &str) -> ParseResult<String> {
    Err(Error::UnsupportedRegex("regex is not supported by Kani."))
}

/// Creates a strategy which generates byte strings matching the given regular
/// expression.
pub fn bytes_regex(_: &str) -> ParseResult<Vec<u8>> {
    Err(Error::UnsupportedRegex("regex is not supported by Kani."))
}
