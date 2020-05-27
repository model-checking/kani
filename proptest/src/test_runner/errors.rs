//-
// Copyright 2017, 2018 The proptest developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use crate::std_facade::fmt;

#[cfg(feature = "std")]
use std::string::ToString;

use crate::test_runner::Reason;

/// Errors which can be returned from test cases to indicate non-successful
/// completion.
///
/// Note that in spite of the name, `TestCaseError` is currently *not* an
/// instance of `Error`, since otherwise `impl<E : Error> From<E>` could not be
/// provided.
///
/// Any `Error` can be converted to a `TestCaseError`, which places
/// `Error::display()` into the `Fail` case.
#[derive(Debug, Clone)]
pub enum TestCaseError {
    /// The input was not valid for the test case. This does not count as a
    /// test failure (nor a success); rather, it simply signals to generate
    /// a new input and try again.
    Reject(Reason),
    /// The code under test failed the test.
    Fail(Reason),
}

/// Convenience for the type returned by test cases.
pub type TestCaseResult = Result<(), TestCaseError>;

impl TestCaseError {
    /// Rejects the generated test input as invalid for this test case. This
    /// does not count as a test failure (nor a success); rather, it simply
    /// signals to generate a new input and try again.
    ///
    /// The string gives the location and context of the rejection, and
    /// should be suitable for formatting like `Foo did X at {whence}`.
    pub fn reject(reason: impl Into<Reason>) -> Self {
        TestCaseError::Reject(reason.into())
    }

    /// The code under test failed the test.
    ///
    /// The string should indicate the location of the failure, but may
    /// generally be any string.
    pub fn fail(reason: impl Into<Reason>) -> Self {
        TestCaseError::Fail(reason.into())
    }
}

impl fmt::Display for TestCaseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            TestCaseError::Reject(ref whence) => {
                write!(f, "Input rejected at {}", whence)
            }
            TestCaseError::Fail(ref why) => write!(f, "Case failed: {}", why),
        }
    }
}

#[cfg(feature = "std")]
impl<E: ::std::error::Error> From<E> for TestCaseError {
    fn from(cause: E) -> Self {
        TestCaseError::fail(cause.to_string())
    }
}

/// A failure state from running test cases for a single test.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TestError<T> {
    /// The test was aborted for the given reason, for example, due to too many
    /// inputs having been rejected.
    Abort(Reason),
    /// A failing test case was found. The string indicates where and/or why
    /// the test failed. The `T` is the minimal input found to reproduce the
    /// failure.
    Fail(Reason, T),
}

impl<T: fmt::Debug> fmt::Display for TestError<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            TestError::Abort(ref why) => write!(f, "Test aborted: {}", why),
            TestError::Fail(ref why, ref what) => write!(
                f,
                "Test failed: {}; minimal failing input: {:?}",
                why, what
            ),
        }
    }
}

#[cfg(feature = "std")]
#[allow(deprecated)] // description()
impl<T: fmt::Debug> ::std::error::Error for TestError<T> {
    fn description(&self) -> &str {
        match *self {
            TestError::Abort(..) => "Abort",
            TestError::Fail(..) => "Fail",
        }
    }
}
