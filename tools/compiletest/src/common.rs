// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Modifications Copyright Kani Contributors
// See GitHub history for details.

pub use self::Mode::*;

use std::fmt;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use std::time::Duration;
use test::test::TestTimeOptions;
use test::ColorConfig;

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub enum Mode {
    Kani,
    KaniFixme,
    CargoCoverage,
    CargoKani,
    CargoKaniTest, // `cargo kani --tests`. This is temporary and should be removed when s2n-quic moves --tests to `Cargo.toml`.
    CoverageBased,
    Exec,
    Expected,
    Stub,
}

impl FromStr for Mode {
    type Err = ();
    fn from_str(s: &str) -> Result<Mode, ()> {
        match s {
            "kani" => Ok(Kani),
            "kani-fixme" => Ok(KaniFixme),
            "cargo-kani" => Ok(CargoKani),
            "cargo-kani-test" => Ok(CargoKaniTest),
            "coverage-based" => Ok(CoverageBased),
            "cargo-coverage" => Ok(CargoCoverage),
            "exec" => Ok(Exec),
            "expected" => Ok(Expected),
            "stub-tests" => Ok(Stub),
            _ => Err(()),
        }
    }
}

impl fmt::Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match *self {
            Kani => "kani",
            KaniFixme => "kani-fixme",
            CargoCoverage => "cargo-coverage",
            CargoKani => "cargo-kani",
            CargoKaniTest => "cargo-kani-test",
            CoverageBased => "coverage-based",
            Exec => "exec",
            Expected => "expected",
            Stub => "stub-tests",
        };
        fmt::Display::fmt(s, f)
    }
}

/// Step at which Kani test should fail.
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd)]
pub enum KaniFailStep {
    /// Kani panics before the codegen step (up to MIR generation). This step
    /// runs the same checks on the test code as `cargo check` including syntax,
    /// type, name resolution, and borrow checks.
    Check,
    /// Kani panics at the codegen step because the test code uses unimplemented
    /// and/or unsupported features.
    Codegen,
    /// Kani panics after the codegen step because of verification failures or
    /// other CBMC errors.
    Verify,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd)]
pub enum FailMode {
    Check,
    Build,
    Run,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PanicStrategy {
    Unwind,
    Abort,
}

/// Configuration for compiletest
#[derive(Debug, Clone)]
pub struct Config {
    /// The directory containing the tests to run
    pub src_base: PathBuf,

    /// The directory where programs should be built
    pub build_base: PathBuf,

    /// The test mode, e.g. ui or debuginfo.
    pub mode: Mode,

    /// The test suite (essentially which directory is running, but without the
    /// directory prefix such as tests/)
    pub suite: String,

    /// Run ignored tests
    pub run_ignored: bool,

    /// Only run tests that match these filters
    pub filters: Vec<String>,

    /// Exactly match the filter, rather than a substring
    pub filter_exact: bool,

    /// Write out a parseable log of tests that were run
    pub logfile: Option<PathBuf>,

    /// Target system to be tested
    pub target: String,

    /// Host triple for the compiler being invoked
    pub host: String,

    /// Explain what's going on
    pub verbose: bool,

    /// Print one character per test instead of one line
    pub quiet: bool,

    /// Whether to use colors in test.
    pub color: ColorConfig,

    /// The default Rust edition
    pub edition: Option<String>,

    /// Whether to rerun tests even if the inputs are unchanged.
    pub force_rerun: bool,

    /// Timeout duration for each test.
    pub timeout: Option<Duration>,

    /// Whether we will abort execution when a failure occurs.
    /// When set to false, this will execute the entire test suite regardless of any failure.
    pub fail_fast: bool,

    /// Whether we will run the tests or not.
    pub dry_run: bool,

    /// Whether we should update expected tests when there is a mismatch. This is helpful for
    /// updating multiple tests. Users should still manually edit the files after to only keep
    /// relevant expectations.
    pub fix_expected: bool,

    /// Whether we should measure and limit the time of a test.
    pub time_opts: Option<TestTimeOptions>,

    /// Extra arguments to be passed to Kani in this regression.
    /// Note that there is no validation done whether these flags conflict with existing flags.
    /// For example, one could add `--kani-flag=--only-codegen` to only compile all tests.
    pub extra_args: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct TestPaths {
    pub file: PathBuf,         // e.g., compile-test/foo/bar/baz.rs
    pub relative_dir: PathBuf, // e.g., foo/bar
}

/// Absolute path to the directory where all output for all tests in the given
/// `relative_dir` group should reside. Example:
///   /path/to/build/host-triple/test/ui/relative/
/// This is created early when tests are collected to avoid race conditions.
pub fn output_relative_path(config: &Config, relative_dir: &Path) -> PathBuf {
    config.build_base.join(relative_dir)
}

/// Generates a unique name for the test.
pub fn output_testname_unique(testpaths: &TestPaths) -> PathBuf {
    PathBuf::from(&testpaths.file.file_stem().unwrap())
}

/// Absolute path to the directory where all output for the given
/// test should reside. Example:
///   /path/to/build/host-triple/test/ui/relative/testname/
pub fn output_base_dir(config: &Config, testpaths: &TestPaths) -> PathBuf {
    output_relative_path(config, &testpaths.relative_dir).join(output_testname_unique(testpaths))
}

/// Absolute path to the base filename used as output for the given
/// test. Example:
///   /path/to/build/host-triple/test/ui/relative/testname.mode/testname
pub fn output_base_name(config: &Config, testpaths: &TestPaths) -> PathBuf {
    output_base_dir(config, testpaths).join(testpaths.file.file_stem().unwrap())
}
