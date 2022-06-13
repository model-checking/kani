// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Modifications Copyright Kani Contributors
// See GitHub history for details.
use std::convert::TryFrom;
use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;

use rustc_session::config::{ErrorOutputType, Externs};
use rustc_session::lint::Level;
use rustc_session::search_paths::SearchPath;
use rustc_span::edition::Edition;
use rustc_target::spec::TargetTriple;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum OutputFormat {
    Json,
    Html,
}

impl Default for OutputFormat {
    fn default() -> OutputFormat {
        OutputFormat::Html
    }
}

impl OutputFormat {}

impl TryFrom<&str> for OutputFormat {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "json" => Ok(OutputFormat::Json),
            "html" => Ok(OutputFormat::Html),
            _ => Err(format!("unknown output format `{}`", value)),
        }
    }
}

/// Configuration options for rustdoc.
#[derive(Clone)]
pub(crate) struct Options {
    // Basic options / Options passed directly to rustc
    /// The crate root or Markdown file to load.
    pub(crate) input: PathBuf,
    /// The name of the crate being documented.
    pub(crate) crate_name: Option<String>,
    /// Whether or not this is a proc-macro crate
    pub(crate) proc_macro_crate: bool,
    /// How to format errors and warnings.
    pub(crate) error_format: ErrorOutputType,
    /// Library search paths to hand to the compiler.
    pub(crate) libs: Vec<SearchPath>,
    /// Library search paths strings to hand to the compiler.
    pub(crate) lib_strs: Vec<String>,
    /// The list of external crates to link against.
    pub(crate) externs: Externs,
    /// The list of external crates strings to link against.
    pub(crate) extern_strs: Vec<String>,
    /// List of `cfg` flags to hand to the compiler. Always includes `rustdoc`.
    pub(crate) cfgs: Vec<String>,
    /// Codegen options strings to hand to the compiler.
    pub(crate) codegen_options_strs: Vec<String>,
    /// Debugging (`-Z`) options strings to pass to the compiler.
    pub(crate) debugging_opts_strs: Vec<String>,
    /// The target used to compile the crate against.
    pub(crate) target: TargetTriple,
    /// Edition used when reading the crate. Defaults to "2015". Also used by default when
    /// compiling doctests from the crate.
    pub(crate) edition: Edition,
    /// The path to the sysroot. Used during the compilation process.
    pub(crate) maybe_sysroot: Option<PathBuf>,
    /// Lint information passed over the command-line.
    pub(crate) lint_opts: Vec<(String, Level)>,
    /// Whether to ask rustc to describe the lints it knows.
    pub(crate) describe_lints: bool,
    /// What level to cap lints at.
    pub(crate) lint_cap: Option<Level>,

    // Options specific to running doctests
    /// Whether we should run doctests instead of generating docs.
    pub(crate) should_test: bool,
    /// List of arguments to pass to the test harness, if running tests.
    pub(crate) test_args: Vec<String>,
    /// The working directory in which to run tests.
    pub(crate) test_run_directory: Option<PathBuf>,
    /// Optional path to persist the doctest executables to, defaults to a
    /// temporary directory if not set.
    pub(crate) persist_doctests: Option<PathBuf>,
    /// Runtool to run doctests with
    pub(crate) runtool: Option<String>,
    /// Arguments to pass to the runtool
    pub(crate) runtool_args: Vec<String>,
    /// Whether to allow ignoring doctests on a per-target basis
    /// For example, using ignore-foo to ignore running the doctest on any target that
    /// contains "foo" as a substring
    pub(crate) enable_per_target_ignores: bool,
    /// Do not run doctests, compile them if should_test is active.
    pub(crate) no_run: bool,

    /// The path to a rustc-like binary to build tests with. If not set, we
    /// default to loading from `$sysroot/bin/rustc`.
    pub(crate) test_builder: Option<PathBuf>,

    // Options that affect the documentation process
    /// Whether to run the `calculate-doc-coverage` pass, which counts the number of public items
    /// with and without documentation.
    pub(crate) show_coverage: bool,

    // Options that alter generated documentation pages
    /// Crate version to note on the sidebar of generated docs.
    pub(crate) crate_version: Option<String>,
    /// Collected options specific to outputting final pages.
    pub(crate) render_options: RenderOptions,
    /// If this option is set to `true`, rustdoc will only run checks and not generate
    /// documentation.
    pub(crate) run_check: bool,
    /// Whether doctests should emit unused externs
    pub(crate) json_unused_externs: bool,
    /// Whether to skip capturing stdout and stderr of tests.
    pub(crate) nocapture: bool,
}

impl fmt::Debug for Options {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        struct FmtExterns<'a>(&'a Externs);

        impl<'a> fmt::Debug for FmtExterns<'a> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.debug_map().entries(self.0.iter()).finish()
            }
        }

        f.debug_struct("Options")
            .field("input", &self.input)
            .field("crate_name", &self.crate_name)
            .field("proc_macro_crate", &self.proc_macro_crate)
            .field("error_format", &self.error_format)
            .field("libs", &self.libs)
            .field("externs", &FmtExterns(&self.externs))
            .field("cfgs", &self.cfgs)
            .field("codegen_options", &"...")
            .field("debugging_options", &"...")
            .field("target", &self.target)
            .field("edition", &self.edition)
            .field("maybe_sysroot", &self.maybe_sysroot)
            .field("lint_opts", &self.lint_opts)
            .field("describe_lints", &self.describe_lints)
            .field("lint_cap", &self.lint_cap)
            .field("should_test", &self.should_test)
            .field("test_args", &self.test_args)
            .field("test_run_directory", &self.test_run_directory)
            .field("persist_doctests", &self.persist_doctests)
            .field("show_coverage", &self.show_coverage)
            .field("crate_version", &self.crate_version)
            .field("render_options", &self.render_options)
            .field("runtool", &self.runtool)
            .field("runtool_args", &self.runtool_args)
            .field("enable-per-target-ignores", &self.enable_per_target_ignores)
            .field("run_check", &self.run_check)
            .field("no_run", &self.no_run)
            .field("nocapture", &self.nocapture)
            .finish()
    }
}

/// Configuration options for the HTML page-creation process.
#[derive(Clone, Debug)]
pub(crate) struct RenderOptions {
    /// Document items that have lower than `pub` visibility.
    pub(crate) document_private: bool,
    /// Document items that have `doc(hidden)`.
    pub(crate) document_hidden: bool,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum EmitType {
    Unversioned,
    Toolchain,
    InvocationSpecific,
}

impl FromStr for EmitType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use EmitType::*;
        match s {
            "unversioned-shared-resources" => Ok(Unversioned),
            "toolchain-shared-resources" => Ok(Toolchain),
            "invocation-specific" => Ok(InvocationSpecific),
            _ => Err(()),
        }
    }
}

impl Options {}
