// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use anyhow::bail;
use clap::{arg_enum, Error, ErrorKind};
use std::ffi::OsString;
use std::path::PathBuf;
use structopt::StructOpt;

// By default we configure CBMC to use 16 bits to represent the object bits in pointers.
const DEFAULT_OBJECT_BITS: u32 = 16;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "kani",
    about = "Verify a single Rust crate. For more information, see https://github.com/model-checking/kani",
    setting = structopt::clap::AppSettings::AllArgsOverrideSelf
)]
pub struct StandaloneArgs {
    /// Rust file to verify
    #[structopt(parse(from_os_str))]
    pub input: PathBuf,

    #[structopt(flatten)]
    pub common_opts: KaniArgs,
}

#[derive(Debug, StructOpt)]
#[structopt(
    name = "cargo-kani",
    about = "Verify a Rust crate. For more information, see https://github.com/model-checking/kani",
    setting = structopt::clap::AppSettings::AllArgsOverrideSelf
)]
pub struct CargoKaniArgs {
    #[structopt(flatten)]
    pub common_opts: KaniArgs,
}

// Common arguments for invoking Kani. This gets put into KaniContext, whereas
// anything above is "local" to "main"'s control flow.
#[derive(Debug, StructOpt)]
pub struct KaniArgs {
    /// Generate visualizer report to <target-dir>/report/html/index.html
    #[structopt(long)]
    pub visualize: bool,
    /// Keep temporary files generated throughout Kani process
    #[structopt(long, hidden_short_help(true))]
    pub keep_temps: bool,

    /// Produce full debug information
    #[structopt(long)]
    pub debug: bool,
    /// Produces no output, just an exit code and requested artifacts; overrides --verbose
    #[structopt(long, short)]
    pub quiet: bool,
    /// Output processing stages and commands, along with minor debug information
    #[structopt(long, short)]
    pub verbose: bool,
    /// Enable usage of unstable options
    #[structopt(long, hidden_short_help(true))]
    pub enable_unstable: bool,

    // Hide this since it depends on function that is a hidden option.
    /// Print commands instead of running them
    #[structopt(long, requires("function"), hidden(true))]
    pub dry_run: bool,
    /// Generate C file equivalent to inputted program.
    /// This feature is unstable and it requires `--enable-unstable` to be used
    #[structopt(long, hidden_short_help(true), requires("enable-unstable"))]
    pub gen_c: bool,

    // TODO: currently only cargo-kani pays attention to this.
    /// Directory for all generated artifacts. Only effective when running Kani with cargo
    #[structopt(long, parse(from_os_str))]
    pub target_dir: Option<PathBuf>,

    /// Toggle between different styles of output
    #[structopt(long, default_value = "regular", possible_values = &OutputFormat::variants(), case_insensitive = true)]
    pub output_format: OutputFormat,

    #[structopt(flatten)]
    pub checks: CheckArgs,

    /// Entry point for verification (symbol name).
    /// This is an unstable feature. Consider using --harness instead
    #[structopt(long, hidden = true, requires("enable-unstable"))]
    pub function: Option<String>,
    /// Entry point for verification (proof harness)
    // In a dry-run, we don't have kani-metadata.json to read, so we can't use this flag
    #[structopt(long, conflicts_with = "function", conflicts_with = "dry-run")]
    pub harness: Option<String>,

    /// Link external C files referenced by Rust code.
    /// This is an experimental feature and requires `--enable-unstable` to be used
    #[structopt(long, parse(from_os_str), hidden = true, requires("enable-unstable"))]
    pub c_lib: Vec<PathBuf>,
    /// Enable test function verification. Only use this option when the entry point is a test function
    #[structopt(long)]
    pub tests: bool,
    /// Kani will only compile the crate. No verification will be performed
    #[structopt(long, hidden_short_help(true))]
    pub only_codegen: bool,

    /// Specify the value used for loop unwinding in CBMC
    #[structopt(long)]
    pub default_unwind: Option<u32>,
    /// Specify the value used for loop unwinding for the specified harness in CBMC
    #[structopt(long, requires("harness"))]
    pub unwind: Option<u32>,
    /// Pass through directly to CBMC; must be the last flag.
    /// This feature is unstable and it requires `--enable-unstable` to be used
    #[structopt(long, allow_hyphen_values = true, min_values(0), requires("enable-unstable"))]
    // consumes everything
    pub cbmc_args: Vec<OsString>,

    // Hide option till https://github.com/model-checking/kani/issues/697 is
    // fixed.
    /// Use abstractions for the standard library.
    /// This is an experimental feature and requires `--enable-unstable` to be used
    #[structopt(long, hidden = true, requires("enable-unstable"))]
    pub use_abs: bool,
    // Hide option till https://github.com/model-checking/kani/issues/697 is
    // fixed.
    /// Choose abstraction for modules of standard library if available
    #[structopt(long, default_value = "std", possible_values = &AbstractionType::variants(),
    case_insensitive = true, hidden = true)]
    pub abs_type: AbstractionType,

    /// Enable extra pointer checks such as invalid pointers in relation operations and pointer
    /// arithmetic overflow.
    /// This feature is unstable and it may yield false counter examples. It requires
    /// `--enable-unstable` to be used
    #[structopt(long, hidden_short_help(true), requires("enable-unstable"))]
    pub extra_pointer_checks: bool,

    /// Restrict the targets of virtual table function pointer calls.
    /// This feature is unstable and it requires `--enable-unstable` to be used
    #[structopt(long, hidden_short_help(true), requires("enable-unstable"))]
    pub restrict_vtable: bool,
    /// Disable restricting the targets of virtual table function pointer calls
    #[structopt(long, hidden_short_help(true))]
    pub no_restrict_vtable: bool,
    /// Turn off assertion reachability checks
    #[structopt(long)]
    pub no_assertion_reach_checks: bool,

    /// Do not error out for crates containing `global_asm!`.
    /// This option may impact the soundness of the analysis and may cause false proofs and/or counterexamples
    #[structopt(long, hidden_short_help(true), requires("enable-unstable"))]
    pub ignore_global_asm: bool,

    /// Check if functions satisfy their contracts.
    #[structopt(long, hidden_short_help(true), requires("enable-unstable"))]
    pub enforce_contracts: bool,

    /// Replace functions with their contracts.
    #[structopt(long, hidden_short_help(true), requires("enable-unstable"))]
    pub replace_with_contracts: bool,
    /*
    The below is a "TODO list" of things not yet implemented from the kani_flags.py script.

        add_flag(group, "--gen-c-runnable", default=False, action=BooleanOptionalAction,
                 help="Generate C file equivalent to inputted program; "
                      "performs additional processing to produce valid C code "
                      "at the cost of some readability")
        */
}

impl KaniArgs {
    pub fn restrict_vtable(&self) -> bool {
        self.restrict_vtable
        // if we flip the default, this will become: !self.no_restrict_vtable
    }

    pub fn assertion_reach_checks(&self) -> bool {
        // Turn them off when visualizing an error trace.
        !self.no_assertion_reach_checks && !self.visualize
    }

    pub fn cbmc_object_bits(&self) -> Option<u32> {
        if self.cbmc_args.contains(&OsString::from("--object-bits")) {
            None
        } else {
            Some(DEFAULT_OBJECT_BITS)
        }
    }
}

arg_enum! {
    #[derive(Debug, PartialEq, Eq)]
    pub enum OutputFormat {
        Regular,
        Terse,
        Old,
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum AbstractionType {
    Std,
    Kani,
    CFfi,
    NoBack,
}
// We need customization to support dashes like 'no-back'
impl std::str::FromStr for AbstractionType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_string().to_lowercase().as_ref() {
            "std" => Ok(Self::Std),
            "kani" => Ok(Self::Kani),
            "c-ffi" => Ok(Self::CFfi),
            "no-back" => Ok(Self::NoBack),
            _ => bail!("Unknown abs_type {}", s),
        }
    }
}
impl std::fmt::Display for AbstractionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Std => f.write_str("std"),
            Self::Kani => f.write_str("kani"),
            Self::CFfi => f.write_str("c-ffi"),
            Self::NoBack => f.write_str("no-back"),
        }
    }
}
impl AbstractionType {
    pub fn variants() -> Vec<&'static str> {
        vec!["std", "kani", "c-ffi", "no-back"]
    }
}

#[derive(Debug, StructOpt)]
pub struct CheckArgs {
    // Rust argument parsers (/clap) don't have the convenient '--flag' and '--no-flag' boolean pairs, so approximate
    // We're put both here then create helper functions to "intepret"
    /// Turn on all default checks
    #[structopt(long)]
    pub default_checks: bool,
    /// Turn off all default checks
    #[structopt(long)]
    pub no_default_checks: bool,

    /// Turn on default memory safety checks
    #[structopt(long)]
    pub memory_safety_checks: bool,
    /// Turn off default memory safety checks
    #[structopt(long)]
    pub no_memory_safety_checks: bool,

    /// Turn on default overflow checks
    #[structopt(long)]
    pub overflow_checks: bool,
    /// Turn off default overflow checks
    #[structopt(long)]
    pub no_overflow_checks: bool,

    /// Turn on undefined function checks
    #[structopt(long)]
    pub undefined_function_checks: bool,
    /// Turn off undefined function checks
    #[structopt(long)]
    pub no_undefined_function_checks: bool,

    /// Turn on default unwinding checks
    #[structopt(long)]
    pub unwinding_checks: bool,
    /// Turn off default unwinding checks
    #[structopt(long)]
    pub no_unwinding_checks: bool,
}

impl CheckArgs {
    pub fn memory_safety_on(&self) -> bool {
        !self.no_default_checks && !self.no_memory_safety_checks || self.memory_safety_checks
    }
    pub fn overflow_on(&self) -> bool {
        !self.no_default_checks && !self.no_overflow_checks || self.overflow_checks
    }
    pub fn undefined_function_on(&self) -> bool {
        !self.no_default_checks && !self.no_undefined_function_checks
            || self.undefined_function_checks
    }
    pub fn unwinding_on(&self) -> bool {
        !self.no_default_checks && !self.no_unwinding_checks || self.unwinding_checks
    }
}

impl StandaloneArgs {
    pub fn validate(&self) {
        self.common_opts.validate();
    }
}
impl CargoKaniArgs {
    pub fn validate(&self) {
        self.common_opts.validate();
    }
}
impl KaniArgs {
    pub fn validate(&self) {
        let extra_unwind =
            self.cbmc_args.iter().any(|s| s.to_str().unwrap().starts_with("--unwind"));
        let natives_unwind = self.default_unwind.is_some() || self.unwind.is_some();

        // TODO: these conflicting flags reflect what's necessary to pass current tests unmodified.
        // We should consider improving the error messages slightly in a later pull request.
        if natives_unwind && extra_unwind {
            Error::with_description(
                "Conflicting flags: unwind flags provided to kani and in --cbmc-args.",
                ErrorKind::ArgumentConflict,
            )
            .exit();
        }

        if self.cbmc_args.contains(&OsString::from("--function")) {
            Error::with_description(
                "Invalid flag: --function should be provided to Kani directly, not via --cbmc-args.",
                ErrorKind::ArgumentConflict,
            )
            .exit();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;
    use clap::ArgMatches;

    #[test]
    fn check_arg_parsing() {
        let a = StandaloneArgs::from_iter(vec![
            "kani",
            "file.rs",
            "--enable-unstable",
            "--cbmc-args",
            "--multiple",
            "args",
            "--here",
        ]);
        assert_eq!(a.common_opts.cbmc_args, vec!["--multiple", "args", "--here"]);
        let _b =
            StandaloneArgs::from_iter(vec!["kani", "file.rs", "--enable-unstable", "--cbmc-args"]);
        // no assertion: the above might fail if it fails to allow 0 args to cbmc-args
    }

    #[test]
    fn check_abs_type() {
        // Since we manually implemented this, consistency check it
        for t in AbstractionType::variants() {
            assert_eq!(t, format!("{}", AbstractionType::from_str(t).unwrap()));
        }
    }

    #[test]
    fn check_dry_run_harness_conflicts() {
        // harness needs metadata which we don't have with dry-run
        let args = vec!["kani", "file.rs", "--dry-run", "--harness", "foo"];
        let app = StandaloneArgs::clap();
        let err = app.get_matches_from_safe(args).unwrap_err();
        assert_eq!(err.kind, ErrorKind::ArgumentConflict);
    }

    #[test]
    fn check_unwind_conflicts() {
        // --unwind cannot be called without --harness
        let args = vec!["kani", "file.rs", "--unwind", "3"];
        let app = StandaloneArgs::clap();
        let err = app.get_matches_from_safe(args).unwrap_err();
        assert_eq!(err.kind, ErrorKind::MissingRequiredArgument);
    }

    fn parse_unstable_disabled(args: &str) -> Result<ArgMatches<'_>, Error> {
        let args = format!("kani file.rs {}", args);
        let app = StandaloneArgs::clap();
        app.get_matches_from_safe(args.split(' '))
    }

    fn parse_unstable_enabled(args: &str) -> Result<ArgMatches<'_>, Error> {
        let args = format!("kani --enable-unstable file.rs {}", args);
        let app = StandaloneArgs::clap();
        app.get_matches_from_safe(args.split(' '))
    }

    fn check_unstable_flag(args: &str) {
        // Should fail without --enable-unstable.
        assert_eq!(
            parse_unstable_disabled(&args).unwrap_err().kind,
            ErrorKind::MissingRequiredArgument
        );

        // Should succeed with --enable-unstable.
        let result = parse_unstable_enabled(&args);
        assert!(result.is_ok());
        let flag = args.split(' ').next().unwrap();
        assert!(result.unwrap().is_present(&flag[2..]));
    }

    #[test]
    fn check_abs_unstable() {
        check_unstable_flag("--use-abs")
    }

    #[test]
    fn check_restrict_vtable_unstable() {
        check_unstable_flag("--restrict-vtable")
    }

    #[test]
    fn check_restrict_cbmc_args() {
        check_unstable_flag("--cbmc-args --json-ui")
    }
}
