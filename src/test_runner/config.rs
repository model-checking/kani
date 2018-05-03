//-
// Copyright 2017, 2018 The proptest developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#[cfg(feature = "std")]
use std::env;
#[cfg(feature = "std")]
use std::ffi::OsString;

#[cfg(all(feature = "alloc", not(feature="std")))]
use alloc::boxed::Box;
#[cfg(feature = "std")]
use std::boxed::Box;

use test_runner::FailurePersistence;
#[cfg(feature = "std")]
use test_runner::FileFailurePersistence;

#[cfg(feature = "std")]
const CASES: &str = "PROPTEST_CASES";
#[cfg(feature = "std")]
const MAX_LOCAL_REJECTS: &str = "PROPTEST_MAX_LOCAL_REJECTS";
#[cfg(feature = "std")]
const MAX_GLOBAL_REJECTS: &str = "PROPTEST_MAX_GLOBAL_REJECTS";
#[cfg(feature = "std")]
const MAX_FLAT_MAP_REGENS: &str = "PROPTEST_MAX_FLAT_MAP_REGENS";

/// The default config, computed by combining environment variables and
/// defaults.
lazy_static! {
    static ref DEFAULT_CONFIG: Config = {
        let result = Config {
            cases: 256,
            max_local_rejects: 65_536,
            max_global_rejects: 1024,
            max_flat_map_regens: 1_000_000,
            failure_persistence: None,
            source_file: None,
            _non_exhaustive: (),
        };

        #[cfg(feature = "std")]
        fn contextualize_config(mut result: Config) -> Config {

            fn parse_or_warn(src: &OsString, dst: &mut u32, var: &str) {
                if let Some(src) = src.to_str() {
                    if let Ok(value) = src.parse() {
                        *dst = value;
                    } else {
                        eprintln!(
                            "proptest: The env-var {}={} can't be parsed as u32, \
                             using default of {}.", var, src, *dst);
                    }
                } else {
                    eprintln!(
                        "proptest: The env-var {} is not valid, using \
                         default of {}.", var, *dst);
                }
            }

            result.failure_persistence = Some(Box::new(FileFailurePersistence::default()));
            for (var, value) in env::vars_os() {
                if let Some(var) = var.to_str() {
                    match var {
                        CASES => parse_or_warn(&value,
                            &mut result.cases, CASES),
                        MAX_LOCAL_REJECTS => parse_or_warn(&value,
                            &mut result.max_local_rejects, MAX_LOCAL_REJECTS),
                        MAX_GLOBAL_REJECTS => parse_or_warn(&value,
                            &mut result.max_global_rejects, MAX_GLOBAL_REJECTS),
                        MAX_FLAT_MAP_REGENS => parse_or_warn(&value,
                            &mut result.max_flat_map_regens, MAX_FLAT_MAP_REGENS),
                        _ => if var.starts_with("PROPTEST_") {
                            eprintln!("proptest: Ignoring unknown env-var {}.",
                                      var);
                        },
                    }
                }
            }

            result
        }

        #[cfg(not(feature = "std"))]
        fn contextualize_config(result: Config) -> Config { result }

        contextualize_config(result)
    };
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
    ///
    ///
    pub failure_persistence: Option<Box<FailurePersistence>>,

    /// File location of the current test, relevant for persistence
    /// and debugging.
    ///
    /// Note the use of `&str` rather than `Path` to be compatible with
    /// `#![no_std] use cases where `Path` is unavailable.
    ///
    /// See the docs of [`FileFailurePersistence`](enum.FileFailurePersistence.html)
    /// for more information on how it may be used for persistence.
    pub source_file: Option<&'static str>,
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
        Self { cases, .. Config::default() }
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
        Self { source_file: Some(source_file), .. Config::default() }
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
}

impl Default for Config {
    fn default() -> Self {
        DEFAULT_CONFIG.clone()
    }
}
