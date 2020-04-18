//-
// Copyright 2017, 2018, 2019 The proptest developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use crate::std_facade::Box;
use core::u32;

#[cfg(feature = "std")]
use std::env;
#[cfg(feature = "std")]
use std::ffi::OsString;
#[cfg(feature = "std")]
use std::fmt;
#[cfg(feature = "std")]
use std::str::FromStr;

use crate::test_runner::result_cache::{noop_result_cache, ResultCache};
use crate::test_runner::rng::RngAlgorithm;
use crate::test_runner::FailurePersistence;
#[cfg(feature = "std")]
use crate::test_runner::FileFailurePersistence;

#[cfg(feature = "std")]
const CASES: &str = "PROPTEST_CASES";
#[cfg(feature = "std")]
const MAX_LOCAL_REJECTS: &str = "PROPTEST_MAX_LOCAL_REJECTS";
#[cfg(feature = "std")]
const MAX_GLOBAL_REJECTS: &str = "PROPTEST_MAX_GLOBAL_REJECTS";
#[cfg(feature = "std")]
const MAX_FLAT_MAP_REGENS: &str = "PROPTEST_MAX_FLAT_MAP_REGENS";
#[cfg(feature = "std")]
const MAX_SHRINK_TIME: &str = "PROPTEST_MAX_SHRINK_TIME";
#[cfg(feature = "std")]
const MAX_SHRINK_ITERS: &str = "PROPTEST_MAX_SHRINK_ITERS";
#[cfg(feature = "fork")]
const FORK: &str = "PROPTEST_FORK";
#[cfg(feature = "timeout")]
const TIMEOUT: &str = "PROPTEST_TIMEOUT";
#[cfg(feature = "std")]
const VERBOSE: &str = "PROPTEST_VERBOSE";
const RNG_ALGORITHM: &str = "PROPTEST_RNG_ALGORITHM";

#[cfg(feature = "std")]
fn contextualize_config(mut result: Config) -> Config {
    fn parse_or_warn<T: FromStr + fmt::Display>(
        src: &OsString,
        dst: &mut T,
        typ: &str,
        var: &str,
    ) {
        if let Some(src) = src.to_str() {
            if let Ok(value) = src.parse() {
                *dst = value;
            } else {
                eprintln!(
                    "proptest: The env-var {}={} can't be parsed as {}, \
                     using default of {}.",
                    var, src, typ, *dst
                );
            }
        } else {
            eprintln!(
                "proptest: The env-var {} is not valid, using \
                 default of {}.",
                var, *dst
            );
        }
    }

    result.failure_persistence =
        Some(Box::new(FileFailurePersistence::default()));
    for (var, value) in
        env::vars_os().filter_map(|(k, v)| k.into_string().ok().map(|k| (k, v)))
    {
        match var.as_str() {
            CASES => parse_or_warn(&value, &mut result.cases, "u32", CASES),
            MAX_LOCAL_REJECTS => parse_or_warn(
                &value,
                &mut result.max_local_rejects,
                "u32",
                MAX_LOCAL_REJECTS,
            ),
            MAX_GLOBAL_REJECTS => parse_or_warn(
                &value,
                &mut result.max_global_rejects,
                "u32",
                MAX_GLOBAL_REJECTS,
            ),
            MAX_FLAT_MAP_REGENS => parse_or_warn(
                &value,
                &mut result.max_flat_map_regens,
                "u32",
                MAX_FLAT_MAP_REGENS,
            ),
            #[cfg(feature = "fork")]
            FORK => parse_or_warn(&value, &mut result.fork, "bool", FORK),
            #[cfg(feature = "timeout")]
            TIMEOUT => {
                parse_or_warn(&value, &mut result.timeout, "timeout", TIMEOUT)
            }
            MAX_SHRINK_TIME => parse_or_warn(
                &value,
                &mut result.max_shrink_time,
                "u32",
                MAX_SHRINK_TIME,
            ),
            MAX_SHRINK_ITERS => parse_or_warn(
                &value,
                &mut result.max_shrink_iters,
                "u32",
                MAX_SHRINK_ITERS,
            ),
            VERBOSE => {
                parse_or_warn(&value, &mut result.verbose, "u32", VERBOSE)
            }
            RNG_ALGORITHM => parse_or_warn(
                &value,
                &mut result.rng_algorithm,
                "RngAlgorithm",
                RNG_ALGORITHM,
            ),

            _ => {
                if var.starts_with("PROPTEST_") {
                    eprintln!("proptest: Ignoring unknown env-var {}.", var);
                }
            }
        }
    }

    result
}

#[cfg(not(feature = "std"))]
fn contextualize_config(result: Config) -> Config {
    result
}

fn default_default_config() -> Config {
    Config {
        cases: 256,
        max_local_rejects: 65_536,
        max_global_rejects: 1024,
        max_flat_map_regens: 1_000_000,
        failure_persistence: None,
        source_file: None,
        test_name: None,
        #[cfg(feature = "fork")]
        fork: false,
        #[cfg(feature = "timeout")]
        timeout: 0,
        #[cfg(feature = "std")]
        max_shrink_time: 0,
        max_shrink_iters: u32::MAX,
        result_cache: noop_result_cache,
        #[cfg(feature = "std")]
        verbose: 0,
        rng_algorithm: RngAlgorithm::default(),
        _non_exhaustive: (),
    }
}

// The default config, computed by combining environment variables and
// defaults.
#[cfg(feature = "std")]
lazy_static! {
    static ref DEFAULT_CONFIG: Config =
        contextualize_config(default_default_config());
}

/// Configuration for how a proptest test should be run.
#[derive(Clone, Debug, PartialEq)]
pub struct Config {
    /// The number of successful test cases that must execute for the test as a
    /// whole to pass.
    ///
    /// This does not include implicitly-replayed persisted failing cases.
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

    /// Indicates whether and how to persist failed test results.
    ///
    /// When compiling with "std" feature (i.e. the standard library is available), the default
    /// is `Some(Box::new(FileFailurePersistence::SourceParallel("proptest-regressions")))`.
    ///
    /// Without the standard library, the default is `None`, and no persistence occurs.
    ///
    /// See the docs of [`FileFailurePersistence`](enum.FileFailurePersistence.html)
    /// and [`MapFailurePersistence`](struct.MapFailurePersistence.html) for more information.
    ///
    /// The default cannot currently be overridden by an environment variable.
    pub failure_persistence: Option<Box<dyn FailurePersistence>>,

    /// File location of the current test, relevant for persistence
    /// and debugging.
    ///
    /// Note the use of `&str` rather than `Path` to be compatible with
    /// `#![no_std]` use cases where `Path` is unavailable.
    ///
    /// See the docs of [`FileFailurePersistence`](enum.FileFailurePersistence.html)
    /// for more information on how it may be used for persistence.
    pub source_file: Option<&'static str>,

    /// The fully-qualified name of the test being run, as would be passed to
    /// the test executable to run just that test.
    ///
    /// This must be set if `fork` is `true`. Otherwise, it is unused. It is
    /// automatically set by `proptest!`.
    ///
    /// This must include the crate name at the beginning, as produced by
    /// `module_path!()`.
    pub test_name: Option<&'static str>,

    /// If true, tests are run in a subprocess.
    ///
    /// Forking allows proptest to work with tests which may fail by aborting
    /// the process, causing a segmentation fault, etc, but can be a lot slower
    /// in certain environments or when running a very large number of tests.
    ///
    /// For forking to work correctly, both the `Strategy` and the content of
    /// the test case itself must be deterministic.
    ///
    /// This requires the "fork" feature, enabled by default.
    ///
    /// The default is `false`, which can be overridden by setting the
    /// `PROPTEST_FORK` environment variable.
    #[cfg(feature = "fork")]
    pub fork: bool,

    /// If non-zero, tests are run in a subprocess and each generated case
    /// fails if it takes longer than this number of milliseconds.
    ///
    /// This implicitly enables forking, even if the `fork` field is `false`.
    ///
    /// The type here is plain `u32` (rather than
    /// `Option<std::time::Duration>`) for the sake of ergonomics.
    ///
    /// This requires the "timeout" feature, enabled by default.
    ///
    /// Setting a timeout to less than the time it takes the process to start
    /// up and initialise the first test case will cause the whole test to be
    /// aborted.
    ///
    /// The default is `0` (i.e., no timeout), which can be overridden by
    /// setting the `PROPTEST_TIMEOUT` environment variable.
    #[cfg(feature = "timeout")]
    pub timeout: u32,

    /// If non-zero, give up the shrinking process after this many milliseconds
    /// have elapsed since the start of the shrinking process.
    ///
    /// This will not cause currently running test cases to be interrupted.
    ///
    /// This configuration is only available when the `std` feature is enabled
    /// (which it is by default).
    ///
    /// The default is `0` (i.e., no limit), which can be overridden by setting
    /// the `PROPTEST_MAX_SHRINK_TIME` environment variable.
    #[cfg(feature = "std")]
    pub max_shrink_time: u32,

    /// Give up on shrinking if more than this number of iterations of the test
    /// code are run.
    ///
    /// Setting this to `std::u32::MAX` causes the actual limit to be four
    /// times the number of test cases.
    ///
    /// Setting this value to `0` disables shrinking altogether.
    ///
    /// Note that the type of this field will change in a future version of
    /// proptest to better accommodate its special values.
    ///
    /// The default is `std::u32::MAX`, which can be overridden by setting the
    /// `PROPTEST_MAX_SHRINK_ITERS` environment variable.
    pub max_shrink_iters: u32,

    /// A function to create new result caches.
    ///
    /// The default is to do no caching. The easiest way to enable caching is
    /// to set this field to `basic_result_cache` (though that is currently
    /// only available with the `std` feature).
    ///
    /// This is useful for strategies which have a tendency to produce
    /// duplicate values, or for tests where shrinking can take a very long
    /// time due to exploring the same output multiple times.
    ///
    /// When caching is enabled, generated values themselves are not stored, so
    /// this does not pose a risk of memory exhaustion for large test inputs
    /// unless using extraordinarily large test case counts.
    ///
    /// Caching incurs its own overhead, and may very well make your test run
    /// more slowly.
    pub result_cache: fn() -> Box<dyn ResultCache>,

    /// Set to non-zero values to cause proptest to emit human-targeted
    /// messages to stderr as it runs.
    ///
    /// Greater values cause greater amounts of logs to be emitted. The exact
    /// meaning of certain levels other than 0 is subject to change.
    ///
    /// - 0: No extra output.
    /// - 1: Log test failure messages.
    /// - 2: Trace low-level details.
    ///
    /// This is only available with the `std` feature (enabled by default)
    /// since on nostd proptest has no way to produce output.
    ///
    /// The default is `0`, which can be overridden by setting the
    /// `PROPTEST_VERBOSE` environment variable.
    #[cfg(feature = "std")]
    pub verbose: u32,

    /// The RNG algorithm to use when not using a user-provided RNG.
    ///
    /// The default is `RngAlgorithm::default()`, which can be overridden by
    /// setting the `PROPTEST_RNG_ALGORITHM` environment variable to one of the following:
    ///
    /// - `xs` — `RngAlgorithm::XorShift`
    /// - `cc` — `RngAlgorithm::ChaCha`
    pub rng_algorithm: RngAlgorithm,

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
    pub fn with_cases(cases: u32) -> Self {
        Self {
            cases,
            ..Config::default()
        }
    }

    /// Constructs a `Config` only differing from the `default()` in the
    /// source_file of the present test.
    ///
    /// This is simply a more concise alternative to using field-record update
    /// syntax:
    ///
    /// ```
    /// # use proptest::test_runner::Config;
    /// assert_eq!(
    ///     Config::with_source_file("computer/question"),
    ///     Config { source_file: Some("computer/question"), .. Config::default() }
    /// );
    /// ```
    pub fn with_source_file(source_file: &'static str) -> Self {
        Self {
            source_file: Some(source_file),
            ..Config::default()
        }
    }

    /// Constructs a `Config` only differing from the provided Config instance, `self`,
    /// in the source_file of the present test.
    ///
    /// This is simply a more concise alternative to using field-record update
    /// syntax:
    ///
    /// ```
    /// # use proptest::test_runner::Config;
    /// let a = Config::with_source_file("computer/question");
    /// let b = a.clone_with_source_file("answer/42");
    /// assert_eq!(
    ///     a,
    ///     Config { source_file: Some("computer/question"), .. Config::default() }
    /// );
    /// assert_eq!(
    ///     b,
    ///     Config { source_file: Some("answer/42"), .. Config::default() }
    /// );
    /// ```
    pub fn clone_with_source_file(&self, source_file: &'static str) -> Self {
        let mut result = self.clone();
        result.source_file = Some(source_file);
        result
    }

    /// Return whether this configuration implies forking.
    ///
    /// This method exists even if the "fork" feature is disabled, in which
    /// case it simply returns false.
    pub fn fork(&self) -> bool {
        self._fork() || self.timeout() > 0
    }

    #[cfg(feature = "fork")]
    fn _fork(&self) -> bool {
        self.fork
    }

    #[cfg(not(feature = "fork"))]
    fn _fork(&self) -> bool {
        false
    }

    /// Returns the configured timeout.
    ///
    /// This method exists even if the "timeout" feature is disabled, in which
    /// case it simply returns 0.
    #[cfg(feature = "timeout")]
    pub fn timeout(&self) -> u32 {
        self.timeout
    }

    /// Returns the configured timeout.
    ///
    /// This method exists even if the "timeout" feature is disabled, in which
    /// case it simply returns 0.
    #[cfg(not(feature = "timeout"))]
    pub fn timeout(&self) -> u32 {
        0
    }

    /// Returns the configured limit on shrinking iterations.
    ///
    /// This takes into account the special "automatic" behaviour.
    pub fn max_shrink_iters(&self) -> u32 {
        if u32::MAX == self.max_shrink_iters {
            self.cases.saturating_mul(4)
        } else {
            self.max_shrink_iters
        }
    }

    // Used by macros to force the config to be owned without depending on
    // certain traits being `use`d.
    #[allow(missing_docs)]
    #[doc(hidden)]
    pub fn __sugar_to_owned(&self) -> Self {
        self.clone()
    }
}

#[cfg(feature = "std")]
impl Default for Config {
    fn default() -> Self {
        DEFAULT_CONFIG.clone()
    }
}

#[cfg(not(feature = "std"))]
impl Default for Config {
    fn default() -> Self {
        default_default_config()
    }
}
