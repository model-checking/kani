// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Modifications Copyright Kani Contributors
// See GitHub history for details.

pub use self::Mode::*;

use std::fmt;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use crate::util::PathBufExt;
use test::ColorConfig;

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub enum Mode {
    Kani,
    KaniFixme,
    CargoKani,
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
            CargoKani => "cargo-kani",
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

    /// Allow us to run the regression with the mir linker enabled by default. For that, set
    /// `RUSTFLAGS=--cfg=mir_linker` while compiling `compiletest`.
    /// Remove this as part of <https://github.com/model-checking/kani/issues/1677>
    pub mir_linker: bool,
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

/// Generates a unique name for the test, such as `testname.revision`.
pub fn output_testname_unique(testpaths: &TestPaths, revision: Option<&str>) -> PathBuf {
    PathBuf::from(&testpaths.file.file_stem().unwrap()).with_extra_extension(revision.unwrap_or(""))
}

/// Absolute path to the directory where all output for the given
/// test/revision should reside. Example:
///   /path/to/build/host-triple/test/ui/relative/testname.revision/
pub fn output_base_dir(config: &Config, testpaths: &TestPaths, revision: Option<&str>) -> PathBuf {
    output_relative_path(config, &testpaths.relative_dir)
        .join(output_testname_unique(testpaths, revision))
}

/// Absolute path to the base filename used as output for the given
/// test/revision. Example:
///   /path/to/build/host-triple/test/ui/relative/testname.revision.mode/testname
pub fn output_base_name(config: &Config, testpaths: &TestPaths, revision: Option<&str>) -> PathBuf {
    output_base_dir(config, testpaths, revision).join(testpaths.file.file_stem().unwrap())
}
