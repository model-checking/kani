//-
// Copyright 2017 Jason Lingle
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! State and functions for running proptest tests.
//!
//! You do not normally need to access things in this module directly except
//! when implementing new low-level strategies.

use std::collections::BTreeMap;
use std::env;
use std::ffi::OsString;
use std::fmt;
use std::panic::{self, AssertUnwindSafe};
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering::SeqCst;

use rand::{self, XorShiftRng};

use strategy::*;

/// The default config, computed by combining environment variables and
/// defaults.
lazy_static! {
    static ref DEFAULT_CONFIG: Config = {
        let mut result = Config {
            cases: 256,
            max_local_rejects: 65536,
            max_global_rejects: 1024,
            max_flat_map_regens: 1_000_000,
            _non_exhaustive: (),
        };

        fn parse_or_warn(dst: &mut u32, value: OsString, var: &str) {
            if let Some(value) = value.to_str() {
                if let Ok(value) = value.parse() {
                    *dst = value;
                } else {
                    eprintln!(
                        "proptest: The env-var {}={} can't be parsed as u32, \
                         using default of {}.", var, value, *dst);
                }
            } else {
                eprintln!(
                    "proptest: The env-var {} is not valid, using \
                     default of {}.", var, *dst);
            }
        }

        for (var, value) in env::vars_os() {
            if let Some(var) = var.to_str() {
                match var {
                    "PROPTEST_CASES" => parse_or_warn(
                        &mut result.cases, value, "PROPTEST_CASES"),
                    "PROPTEST_MAX_LOCAL_REJECTS" => parse_or_warn(
                        &mut result.max_local_rejects, value,
                        "PROPTEST_MAX_LOCAL_REJECTS"),
                    "PROPTEST_MAX_GLOBAL_REJECTS" => parse_or_warn(
                        &mut result.max_global_rejects, value,
                        "PROPTEST_MAX_GLOBAL_REJECTS"),
                    "PROPTEST_MAX_FLAT_MAP_REGENS" => parse_or_warn(
                        &mut result.max_flat_map_regens, value,
                        "PROPTEST_MAX_FLAT_MAP_REGENS"),
                    _ => if var.starts_with("PROPTEST_") {
                        eprintln!("proptest: Ignoring unknown env-var {}.",
                                  var);
                    },
                }
            }
        }

        result
    };
}

/// Configuration for how a proptest test should be run.
#[derive(Clone, Debug, PartialEq)]
pub struct Config {
    /// The number of successful test cases that must execute for the test as a
    /// whole to pass.
    ///
    /// The default is 256, which can be overridden by setting the
    /// `PROPTEST_CASES` environment variable.
    pub cases: u32,
    /// The maximum number of individual inputs that may be rejected before the
    /// test as a whole aborts.
    ///
    /// The default is 65536, which can be overridden by setting the
    /// `PROPTEST_MAX_LOCAL_REJECTS` environment variable.
    pub max_local_rejects: u32,
    /// The maximum number of combined inputs that may be rejected before the
    /// test as a whole aborts.
    ///
    /// The default is 1024, which can be overridden by setting the
    /// `PROPTEST_MAX_GLOBAL_REJECTS` environment variable.
    pub max_global_rejects: u32,
    /// The maximum number of times all `Flatten` combinators will attempt to
    /// regenerate values. This puts a limit on the worst-case exponential
    /// explosion that can happen with nested `Flatten`s.
    ///
    /// The default is 1_000_000, which can be overridden by setting the
    /// `PROPTEST_MAX_FLAT_MAP_REGENS` environment variable.
    pub max_flat_map_regens: u32,
    // Needs to be public so FRU syntax can be used.
    #[doc(hidden)]
    pub _non_exhaustive: (),
}

impl Config {
    /// Constructs a `Config` only differing from the `default()` in the
    /// number of test cases required to pass the test successfully.
    ///
    /// This is simply a more concise alternative to using field-record update
    /// syntax:
    ///
    /// ```
    /// # use proptest::test_runner::Config;
    /// assert_eq!(
    ///     Config::with_cases(42),
    ///     Config { cases: 42, .. Config::default() }
    /// );
    /// ```
    pub fn with_cases(n: u32) -> Self {
        Self {
            cases: n,
            .. Config::default()
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        DEFAULT_CONFIG.clone()
    }
}

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
    ///
    /// The string gives the location and context of the rejection, and
    /// should be suitable for formatting like `Foo did X at {whence}`.
    Reject(Rejection),
    /// The code under test failed the test.
    ///
    /// The string should indicate the location of the failure, but may
    /// generally be any string.
    Fail(Rejection),
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
    pub fn reject<R: Into<Rejection>>(reason: R) -> Self {
        TestCaseError::Reject(reason.into())
    }

    /// The code under test failed the test.
    ///
    /// The string should indicate the location of the failure, but may
    /// generally be any string.
    pub fn fail<R: Into<Rejection>>(reason: R) -> Self {
        TestCaseError::Fail(reason.into())
    }
}

/// Short-hand for `Err(TestCaseError::reject(..))`.
pub fn reject_case<R: Into<Rejection>>(reason: R) -> TestCaseResult {
    Err(TestCaseError::reject(reason))
}

/// Short-hand for `Err(TestCaseError::fail(..))`.
pub fn fail_case<R: Into<Rejection>>(reason: R) -> TestCaseResult {
    Err(TestCaseError::fail(reason))
}

impl fmt::Display for TestCaseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            TestCaseError::Reject(ref whence) =>
                write!(f, "Input rejected at {}", whence),
            TestCaseError::Fail(ref why) =>
                write!(f, "Case failed: {}", why),
        }
    }
}

impl<E : ::std::error::Error> From<E> for TestCaseError {
    fn from(cause: E) -> Self {
        TestCaseError::fail(cause.to_string())
    }
}

/// A failure state from running test cases for a single test.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TestError<T> {
    /// The test was aborted for the given reason, for example, due to too many
    /// inputs having been rejected.
    Abort(Rejection),
    /// A failing test case was found. The string indicates where and/or why
    /// the test failed. The `T` is the minimal input found to reproduce the
    /// failure.
    Fail(Rejection, T),
}

impl<T : fmt::Debug> fmt::Display for TestError<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            TestError::Abort(ref why) =>
                write!(f, "Test aborted: {}", why),
            TestError::Fail(ref why, ref what) =>
                write!(f, "Test failed: {}; minimal failing input: {:?}",
                       why, what),
        }
    }
}

impl<T : fmt::Debug> ::std::error::Error for TestError<T> {
    fn description(&self) -> &str {
        match *self {
            TestError::Abort(..) => "Abort",
            TestError::Fail(..) => "Fail",
        }
    }
}

type RejectionDetail = BTreeMap<Rejection, u32>;

/// State used when running a proptest test.
#[derive(Clone)]
pub struct TestRunner {
    config: Config,
    successes: u32,
    local_rejects: u32,
    global_rejects: u32,
    rng: XorShiftRng,
    flat_map_regens: Arc<AtomicUsize>,

    local_reject_detail: RejectionDetail,
    global_reject_detail: RejectionDetail,
}

impl fmt::Debug for TestRunner {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("TestRunner")
            .field("config", &self.config)
            .field("successes", &self.successes)
            .field("local_rejects", &self.local_rejects)
            .field("global_rejects", &self.global_rejects)
            .field("rng", &"<XorShiftRng>")
            .field("flat_map_regens", &self.flat_map_regens)
            .field("local_reject_detail", &self.local_reject_detail)
            .field("global_reject_detail", &self.global_reject_detail)
            .finish()
    }
}

impl fmt::Display for TestRunner {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "\tsuccesses: {}\n\
                   \tlocal rejects: {}\n",
               self.successes, self.local_rejects)?;
        for (whence, count) in &self.local_reject_detail {
            writeln!(f, "\t\t{} times at {}", count, whence)?;
        }
        writeln!(f, "\tglobal rejects: {}", self.global_rejects)?;
        for (whence, count) in &self.global_reject_detail {
            writeln!(f, "\t\t{} times at {}", count, whence)?;
        }

        Ok(())
    }
}

/// Equivalent to: `TestRunner::default(Config::default())`.
impl Default for TestRunner {
    fn default() -> Self {
        Self::new(Config::default())
    }
}

fn panic_guard<V, F>(case: &V, test: &F) -> TestCaseResult
where
    F: Fn(&V) -> TestCaseResult
{
    match panic::catch_unwind(AssertUnwindSafe(|| test(&case))) {
        Ok(r) => r,
        Err(what) => fail_case(
            what.downcast::<&'static str>().map(|s| reject(*s))
                .or_else(|what| what.downcast::<String>().map(|b| reject(*b)))
                .or_else(|what| what.downcast::<Box<str>>().map(|b| reject(*b)))
                .unwrap_or_else(|_| reject("<unknown panic value>"))),
    }
}

impl TestRunner {
    /// Create a fresh `TestRunner` with the given configuration.
    pub fn new(config: Config) -> Self {
        TestRunner {
            config: config,
            successes: 0,
            local_rejects: 0,
            global_rejects: 0,
            rng: rand::weak_rng(),
            flat_map_regens: Arc::new(AtomicUsize::new(0)),
            local_reject_detail: BTreeMap::new(),
            global_reject_detail: BTreeMap::new(),
        }
    }

    /// Create a fresh `TestRunner` with the same config and global counters as
    /// this one, but with local state reset and an independent `Rng`.
    pub(crate) fn partial_clone(&self) -> Self {
        TestRunner {
            config: self.config.clone(),
            successes: 0,
            local_rejects: 0,
            global_rejects: 0,
            rng: rand::weak_rng(),
            flat_map_regens: Arc::clone(&self.flat_map_regens),
            local_reject_detail: BTreeMap::new(),
            global_reject_detail: BTreeMap::new(),
        }
    }

    /// Returns the RNG for this test run.
    pub fn rng(&mut self) -> &mut XorShiftRng {
        &mut self.rng
    }

    /// Returns the configuration of this runner.
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Run test cases against `f`, choosing inputs via `strategy`.
    ///
    /// If any failure cases occur, try to find a minimal failure case and
    /// report that. If invoking `f` panics, the panic is turned into a
    /// `TestCaseError::Fail`.
    ///
    /// Returns success or failure indicating why the test as a whole failed.
    pub fn run<S : Strategy,
               F : Fn (&ValueFor<S>) -> TestCaseResult>
        (&mut self, strategy: &S, test: F)
         -> Result<(), TestError<ValueFor<S>>>
    {
        while self.successes < self.config.cases {
            let case = match strategy.new_value(self) {
                Ok(v) => v,
                Err(msg) => return Err(TestError::Abort(msg)),
            };
            if self.run_one(case, &test)? {
                self.successes += 1;
            }
        }

        Ok(())
    }

    /// Run one specific test case against this runner.
    ///
    /// If the test fails, finds the minimal failing test case. If the test
    /// does not fail, returns whether it succeeded or was filtered out.
    pub fn run_one<V : ValueTree,
                   F : Fn (&V::Value) -> TestCaseResult>
        (&mut self, mut case: V, test: F) -> Result<bool, TestError<V::Value>>
    {
        let curr = case.current();
        match panic_guard(&curr, &test) {
            Ok(_) => Ok(true),
            Err(TestCaseError::Fail(why)) => {
                let mut last_failure = (why, curr);

                if case.simplify() {
                    loop {
                        let curr = case.current();
                        let passed = match panic_guard(&curr, &test) {
                            // Rejections are effectively a pass here,
                            // since they indicate that any behaviour of
                            // the function under test is acceptable.
                            Ok(_) | Err(TestCaseError::Reject(..)) => true,

                            Err(TestCaseError::Fail(why)) => {
                                last_failure = (why, curr);
                                false
                            },
                        };

                        if passed {
                            if !case.complicate() {
                                break;
                            }
                        } else if !case.simplify() {
                            break;
                        }
                    }
                }

                Err(TestError::Fail(last_failure.0, last_failure.1))
            },
            Err(TestCaseError::Reject(whence)) => {
                self.reject_global(whence)?;
                Ok(false)
            },
        }
    }

    /// Update the state to account for a local rejection from `whence`, and
    /// return `Ok` if the caller should keep going or `Err` to abort.
    pub fn reject_local<R>(&mut self, whence: R) -> Result<(), Rejection>
    where
        R: Into<Rejection>
    {
        if self.local_rejects >= self.config.max_local_rejects {
            Err(reject("Too many local rejects"))
        } else {
            self.local_rejects += 1;
            Self::insert_or_increment(&mut self.local_reject_detail,
                whence.into());
            Ok(())
        }
    }

    /// Update the state to account for a global rejection from `whence`, and
    /// return `Ok` if the caller should keep going or `Err` to abort.
    fn reject_global<T>(&mut self, whence: Rejection) -> Result<(),TestError<T>> {
        if self.global_rejects >= self.config.max_global_rejects {
            Err(TestError::Abort(reject("Too many global rejects")))
        } else {
            self.global_rejects += 1;
            Self::insert_or_increment(&mut self.global_reject_detail, whence);
            Ok(())
        }
    }

    /// Insert 1 or increment the rejection detail at key for whence.
    fn insert_or_increment(into: &mut RejectionDetail, whence: Rejection) {
        use std::collections::btree_map::Entry::*;
        match into.entry(whence) {
            Occupied(oe) => { *oe.into_mut() += 1; },
            Vacant(ve)   => { ve.insert(1); },
        }
        /*
        // TODO: Replace with once and_modify is stable:
        into.entry(whence)
            .and_modify(|count| { *count += 1 })
            .or_insert(1);
        */
    }

    /// Increment the counter of flat map regenerations and return whether it
    /// is still under the configured limit.
    pub fn flat_map_regen(&self) -> bool {
        self.flat_map_regens.fetch_add(1, SeqCst) <
            self.config.max_flat_map_regens as usize
    }
}

#[cfg(test)]
mod test {
    use std::cell::Cell;

    use super::*;

    #[test]
    fn gives_up_after_too_many_rejections() {
        let config = Config::default();
        let mut runner = TestRunner::new(config.clone());
        let runs = Cell::new(0);
        let result = runner.run(&(0u32..), |_| {
            runs.set(runs.get() + 1);
            reject_case("reject")
        });
        match result {
            Err(TestError::Abort(_)) => (),
            e => panic!("Unexpected result: {:?}", e),
        }
        assert_eq!(config.max_global_rejects + 1, runs.get());
    }

    #[test]
    fn test_pass() {
        let mut runner = TestRunner::default();
        let result = runner.run(&(1u32..), |&v| { assert!(v > 0); Ok(()) });
        assert_eq!(Ok(()), result);
    }

    #[test]
    fn test_fail_via_result() {
        let mut runner = TestRunner::default();
        let result = runner.run(&(0u32..10u32), |&v| if v < 5 {
            Ok(())
        } else {
            fail_case("not less than 5")
        });

        assert_eq!(Err(TestError::Fail("not less than 5".into(), 5)), result);
    }

    #[test]
    fn test_fail_via_panic() {
        let mut runner = TestRunner::default();
        let result = runner.run(&(0u32..10u32), |&v| {
            assert!(v < 5, "not less than 5");
            Ok(())
        });
        assert_eq!(Err(TestError::Fail("not less than 5".into(), 5)), result);
    }
}
