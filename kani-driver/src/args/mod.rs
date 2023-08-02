// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Module that define Kani's command line interface. This includes all subcommands.

pub mod assess_args;
pub mod cargo;
pub mod common;
pub mod playback_args;

pub use assess_args::*;

use self::common::*;
use crate::args::cargo::CargoTargetArgs;
use crate::util::warning;
use cargo::CargoCommonArgs;
use clap::builder::{PossibleValue, TypedValueParser};
use clap::{error::ContextKind, error::ContextValue, error::Error, error::ErrorKind, ValueEnum};
use kani_metadata::CbmcSolver;
use std::ffi::OsString;
use std::path::PathBuf;
use std::str::FromStr;
use strum::VariantNames;

/// Trait used to perform extra validation after parsing.
pub trait ValidateArgs {
    /// Perform post-parsing validation but do not abort.
    fn validate(&self) -> Result<(), Error>;
}

/// Validate a set of arguments and ensure they are in a valid state.
/// This method will abort execution with a user friendly error message if the state is invalid.
pub fn check_is_valid<T>(command: &T)
where
    T: clap::Parser + ValidateArgs,
{
    command
        .validate()
        .or_else(|e| -> Result<(), ()> { e.format(&mut T::command()).exit() })
        .unwrap()
}

pub fn print_obsolete(verbosity: &CommonArgs, option: &str) {
    if !verbosity.quiet {
        warning(&format!(
            "The `{option}` option is obsolete. This option no longer has any effect and should be removed"
        ))
    }
}

pub fn print_deprecated(verbosity: &CommonArgs, option: &str, alternative: &str) {
    if !verbosity.quiet {
        warning(&format!(
            "The `{option}` option is deprecated. This option will be removed soon. \
            Consider using `{alternative}` instead"
        ))
    }
}

// By default we configure CBMC to use 16 bits to represent the object bits in pointers.
const DEFAULT_OBJECT_BITS: u32 = 16;

#[derive(Debug, clap::Parser)]
#[command(
    version,
    name = "kani",
    about = "Verify a single Rust crate. For more information, see https://github.com/model-checking/kani",
    args_override_self = true,
    subcommand_negates_reqs = true,
    subcommand_precedence_over_arg = true,
    args_conflicts_with_subcommands = true
)]
pub struct StandaloneArgs {
    /// Rust file to verify
    #[arg(required = true)]
    pub input: Option<PathBuf>,

    #[command(flatten)]
    pub verify_opts: VerificationArgs,

    #[command(subcommand)]
    pub command: Option<StandaloneSubcommand>,
}

/// Kani takes optional subcommands to request specialized behavior.
/// When no subcommand is provided, there is an implied verification subcommand.
#[derive(Debug, clap::Subcommand)]
pub enum StandaloneSubcommand {
    /// Execute concrete playback testcases of a local crate.
    Playback(Box<playback_args::KaniPlaybackArgs>),
}

#[derive(Debug, clap::Parser)]
#[command(
    version,
    name = "cargo-kani",
    about = "Verify a Rust crate. For more information, see https://github.com/model-checking/kani",
    args_override_self = true
)]
pub struct CargoKaniArgs {
    #[command(subcommand)]
    pub command: Option<CargoKaniSubcommand>,

    #[command(flatten)]
    pub verify_opts: VerificationArgs,
}

/// cargo-kani takes optional subcommands to request specialized behavior
#[derive(Debug, clap::Subcommand)]
pub enum CargoKaniSubcommand {
    #[command(hide = true)]
    Assess(Box<crate::assess::AssessArgs>),

    /// Execute concrete playback testcases of a local package.
    Playback(Box<playback_args::CargoPlaybackArgs>),
}

// Common arguments for invoking Kani for verification purpose. This gets put into KaniContext,
// whereas anything above is "local" to "main"'s control flow.
#[derive(Debug, clap::Args)]
pub struct VerificationArgs {
    /// Temporary option to trigger assess mode for out test suite
    /// where we are able to add options but not subcommands
    #[arg(long, hide = true, requires("enable_unstable"))]
    pub assess: bool,

    /// Generate visualizer report to `<target-dir>/report/html/index.html`
    #[arg(long)]
    pub visualize: bool,
    /// Generate concrete playback unit test.
    /// If value supplied is 'print', Kani prints the unit test to stdout.
    /// If value supplied is 'inplace', Kani automatically adds the unit test to your source code.
    /// This option does not work with `--output-format old`.
    #[arg(
        long,
        conflicts_with_all(&["visualize"]),
        ignore_case = true,
        value_enum
    )]
    pub concrete_playback: Option<ConcretePlaybackMode>,
    /// Keep temporary files generated throughout Kani process. This is already the default
    /// behavior for `cargo-kani`.
    #[arg(long, hide_short_help = true)]
    pub keep_temps: bool,

    /// Generate C file equivalent to inputted program.
    /// This feature is unstable and it requires `--enable-unstable` to be used
    #[arg(long, hide_short_help = true, requires("enable_unstable"),
        conflicts_with_all(&["function"]))]
    pub gen_c: bool,

    /// Directory for all generated artifacts.
    #[arg(long)]
    pub target_dir: Option<PathBuf>,

    /// Force Kani to rebuild all packages before the verification.
    #[arg(long)]
    pub force_build: bool,

    /// Toggle between different styles of output
    #[arg(long, default_value = "regular", ignore_case = true, value_enum)]
    pub output_format: OutputFormat,

    #[command(flatten)]
    pub checks: CheckArgs,

    /// Entry point for verification (symbol name).
    /// This is an unstable feature. Consider using --harness instead
    #[arg(long, hide = true, requires("enable_unstable"))]
    pub function: Option<String>,
    /// If specified, only run harnesses that match this filter. This option can be provided
    /// multiple times, which will run all tests matching any of the filters.
    /// If used with --exact, the harness filter will only match the exact fully qualified name of a harness.
    #[arg(
        long = "harness",
        conflicts_with = "function",
        num_args(1),
        value_name = "HARNESS_FILTER"
    )]
    pub harnesses: Vec<String>,

    /// When specified, the harness filter will only match the exact fully qualified name of a harness
    #[arg(long, requires("harnesses"))]
    pub exact: bool,

    /// Link external C files referenced by Rust code.
    /// This is an experimental feature and requires `-Z c-ffi` to be used
    #[arg(long, hide = true, num_args(1..))]
    pub c_lib: Vec<PathBuf>,
    /// Enable test function verification. Only use this option when the entry point is a test function
    #[arg(long)]
    pub tests: bool,
    /// Kani will only compile the crate. No verification will be performed
    #[arg(long, hide_short_help = true)]
    pub only_codegen: bool,

    /// Deprecated flag. This is a no-op since we no longer support the legacy linker and
    /// it will be removed in a future Kani release.
    #[arg(long, hide = true, conflicts_with("mir_linker"))]
    pub legacy_linker: bool,
    /// Deprecated flag. This is a no-op since we no longer support any other linker.
    #[arg(long, hide = true)]
    pub mir_linker: bool,

    /// Specify the value used for loop unwinding in CBMC
    #[arg(long)]
    pub default_unwind: Option<u32>,
    /// Specify the value used for loop unwinding for the specified harness in CBMC
    #[arg(long, requires("harnesses"))]
    pub unwind: Option<u32>,
    /// Specify the CBMC solver to use. Overrides the harness `solver` attribute.
    /// If no solver is specified (with --solver or harness attribute), Kani will use CaDiCal.
    #[arg(long, value_parser = CbmcSolverValueParser::new(CbmcSolver::VARIANTS))]
    pub solver: Option<CbmcSolver>,
    /// Pass through directly to CBMC; must be the last flag.
    /// This feature is unstable and it requires `--enable_unstable` to be used
    #[arg(
        long,
        allow_hyphen_values = true,
        requires("enable_unstable"),
        num_args(0..)
    )]
    // consumes everything
    pub cbmc_args: Vec<OsString>,

    /// Number of parallel jobs, defaults to 1
    #[arg(short, long, hide = true, requires("enable_unstable"))]
    pub jobs: Option<Option<usize>>,

    // Hide option till https://github.com/model-checking/kani/issues/697 is
    // fixed.
    /// Use abstractions for the standard library.
    /// This is an experimental feature and requires `--enable-unstable` to be used
    #[arg(long, hide = true, requires("enable_unstable"))]
    pub use_abs: bool,
    // Hide option till https://github.com/model-checking/kani/issues/697 is
    // fixed.
    /// Choose abstraction for modules of standard library if available
    #[arg(long, default_value = "std", ignore_case = true, hide = true, value_enum)]
    pub abs_type: AbstractionType,

    /// Enable extra pointer checks such as invalid pointers in relation operations and pointer
    /// arithmetic overflow.
    /// This feature is unstable and it may yield false counter examples. It requires
    /// `--enable-unstable` to be used
    #[arg(long, hide_short_help = true, requires("enable_unstable"))]
    pub extra_pointer_checks: bool,

    /// Restrict the targets of virtual table function pointer calls.
    /// This feature is unstable and it requires `--enable-unstable` to be used
    #[arg(long, hide_short_help = true, requires("enable_unstable"))]
    pub restrict_vtable: bool,
    /// Disable restricting the targets of virtual table function pointer calls
    #[arg(long, hide_short_help = true)]
    pub no_restrict_vtable: bool,
    /// Turn off assertion reachability checks
    #[arg(long)]
    pub no_assertion_reach_checks: bool,

    /// Do not error out for crates containing `global_asm!`.
    /// This option may impact the soundness of the analysis and may cause false proofs and/or counterexamples
    #[arg(long, hide_short_help = true, requires("enable_unstable"))]
    pub ignore_global_asm: bool,

    /// Write the GotoC symbol table to a file in JSON format instead of goto binary format.
    #[arg(long, hide_short_help = true)]
    pub write_json_symtab: bool,

    /// Execute CBMC's sanity checks to ensure the goto-program we generate is correct.
    #[arg(long, hide_short_help = true, requires("enable_unstable"))]
    pub run_sanity_checks: bool,

    /// Disable CBMC's slice formula which prevents values from being assigned to redundant variables in traces.
    #[arg(long, hide_short_help = true, requires("enable_unstable"))]
    pub no_slice_formula: bool,

    /// Synthesize loop contracts for all loops.
    #[arg(
        long,
        hide_short_help = true,
        requires("enable_unstable"),
        conflicts_with("unwind"),
        conflicts_with("default_unwind")
    )]
    pub synthesize_loop_contracts: bool,

    /// Randomize the layout of structures. This option can help catching code that relies on
    /// a specific layout chosen by the compiler that is not guaranteed to be stable in the future.
    /// If a value is given, it will be used as the seed for randomization
    /// See the `-Z randomize-layout` and `-Z layout-seed` arguments of the rust compiler.
    #[arg(long)]
    pub randomize_layout: Option<Option<u64>>,

    /// Enable the stubbing of functions and methods.
    // TODO: Stubbing should in principle work with concrete playback.
    // <https://github.com/model-checking/kani/issues/1842>
    #[arg(
        long,
        hide_short_help = true,
        requires("enable_unstable"),
        conflicts_with("concrete_playback")
    )]
    pub enable_stubbing: bool,

    /// Enable Kani coverage output alongside verification result
    #[arg(long, hide_short_help = true)]
    pub coverage: bool,

    /// Arguments to pass down to Cargo
    #[command(flatten)]
    pub cargo: CargoCommonArgs,

    /// Arguments used to select Cargo target.
    #[command(flatten)]
    pub target: CargoTargetArgs,

    #[command(flatten)]
    pub common_args: CommonArgs,
}

impl VerificationArgs {
    pub fn restrict_vtable(&self) -> bool {
        self.restrict_vtable
        // if we flip the default, this will become: !self.no_restrict_vtable
    }

    /// Assertion reachability checks should be disabled when running with --visualize
    pub fn assertion_reach_checks(&self) -> bool {
        !self.no_assertion_reach_checks && !self.visualize
    }

    /// Suppress our default value, if the user has supplied it explicitly in --cbmc-args
    pub fn cbmc_object_bits(&self) -> Option<u32> {
        if self.cbmc_args.contains(&OsString::from("--object-bits")) {
            None
        } else {
            Some(DEFAULT_OBJECT_BITS)
        }
    }

    /// Computes how many threads should be used to verify harnesses.
    pub fn jobs(&self) -> Option<usize> {
        match self.jobs {
            None => Some(1),          // no argument, default 1
            Some(None) => None,       // -j
            Some(Some(x)) => Some(x), // -j=x
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum ConcretePlaybackMode {
    Print,
    // Otherwise clap will default to `in-place`
    #[value(name = "inplace")]
    InPlace,
}

#[derive(Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum OutputFormat {
    Regular,
    Terse,
    Old,
}

#[derive(Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum AbstractionType {
    Std,
    Kani,
    // Clap defaults to `c-ffi`
    CFfi,
    // Clap defaults to `no-back`
    NoBack,
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
#[cfg(test)]
impl AbstractionType {
    pub fn variants() -> Vec<&'static str> {
        vec!["std", "kani", "c-ffi", "no-back"]
    }
}

#[derive(Debug, clap::Args)]
pub struct CheckArgs {
    // Rust argument parsers (/clap) don't have the convenient '--flag' and '--no-flag' boolean pairs, so approximate
    // We're put both here then create helper functions to "intepret"
    /// Turn on all default checks
    #[arg(long)]
    pub default_checks: bool,
    /// Turn off all default checks
    #[arg(long)]
    pub no_default_checks: bool,

    /// Turn on default memory safety checks
    #[arg(long)]
    pub memory_safety_checks: bool,
    /// Turn off default memory safety checks
    #[arg(long)]
    pub no_memory_safety_checks: bool,

    /// Turn on default overflow checks
    #[arg(long)]
    pub overflow_checks: bool,
    /// Turn off default overflow checks
    #[arg(long)]
    pub no_overflow_checks: bool,

    /// Turn on undefined function checks
    #[arg(long)]
    pub undefined_function_checks: bool,
    /// Turn off undefined function checks
    #[arg(long)]
    pub no_undefined_function_checks: bool,

    /// Turn on default unwinding checks
    #[arg(long)]
    pub unwinding_checks: bool,
    /// Turn off default unwinding checks
    #[arg(long)]
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

/// Utility function to error out on arguments that are invalid Cargo specific.
///
/// We currently define a bunch of cargo specific arguments as part of the overall arguments,
/// however, they are invalid in the Kani standalone usage. Explicitly check them for now.
/// TODO: Remove this as part of https://github.com/model-checking/kani/issues/1831
fn check_no_cargo_opt(is_set: bool, name: &str) -> Result<(), Error> {
    if is_set {
        Err(Error::raw(
            ErrorKind::UnknownArgument,
            &format!("argument `{}` cannot be used with standalone Kani.", name),
        ))
    } else {
        Ok(())
    }
}

impl ValidateArgs for StandaloneArgs {
    fn validate(&self) -> Result<(), Error> {
        self.verify_opts.validate()?;
        // Cargo target arguments.
        check_no_cargo_opt(self.verify_opts.target.bins, "--bins")?;
        check_no_cargo_opt(self.verify_opts.target.lib, "--lib")?;
        check_no_cargo_opt(!self.verify_opts.target.bin.is_empty(), "--bin")?;
        // Cargo common arguments.
        check_no_cargo_opt(self.verify_opts.cargo.all_features, "--all-features")?;
        check_no_cargo_opt(self.verify_opts.cargo.no_default_features, "--no-default-features")?;
        check_no_cargo_opt(!self.verify_opts.cargo.features().is_empty(), "--features / -F")?;
        check_no_cargo_opt(!self.verify_opts.cargo.package.is_empty(), "--package / -p")?;
        check_no_cargo_opt(!self.verify_opts.cargo.exclude.is_empty(), "--exclude")?;
        check_no_cargo_opt(self.verify_opts.cargo.workspace, "--workspace")?;
        check_no_cargo_opt(self.verify_opts.cargo.manifest_path.is_some(), "--manifest-path")?;
        if let Some(input) = &self.input {
            if !input.is_file() {
                return Err(Error::raw(
                    ErrorKind::InvalidValue,
                    &format!(
                        "Invalid argument: Input invalid. `{}` is not a regular file.",
                        input.display()
                    ),
                ));
            }
        }
        Ok(())
    }
}

impl<T> ValidateArgs for Option<T>
where
    T: ValidateArgs,
{
    fn validate(&self) -> Result<(), Error> {
        self.as_ref().map_or(Ok(()), |inner| inner.validate())
    }
}

impl ValidateArgs for CargoKaniSubcommand {
    fn validate(&self) -> Result<(), Error> {
        match self {
            // Assess doesn't implement validation yet.
            CargoKaniSubcommand::Assess(_) => Ok(()),
            CargoKaniSubcommand::Playback(playback) => playback.validate(),
        }
    }
}

impl ValidateArgs for CargoKaniArgs {
    fn validate(&self) -> Result<(), Error> {
        self.verify_opts.validate()?;
        self.command.validate()?;
        // --assess requires --enable-unstable, but the subcommand needs manual checking
        if (matches!(self.command, Some(CargoKaniSubcommand::Assess(_))) || self.verify_opts.assess)
            && !self.verify_opts.common_args.enable_unstable
        {
            return Err(Error::raw(
                ErrorKind::MissingRequiredArgument,
                "Assess is unstable and requires 'cargo kani --enable-unstable assess'",
            ));
        }
        Ok(())
    }
}

impl ValidateArgs for VerificationArgs {
    fn validate(&self) -> Result<(), Error> {
        self.common_args.validate()?;
        let extra_unwind =
            self.cbmc_args.iter().any(|s| s.to_str().unwrap().starts_with("--unwind"));
        let natives_unwind = self.default_unwind.is_some() || self.unwind.is_some();

        if self.randomize_layout.is_some() && self.concrete_playback.is_some() {
            let random_seed = if let Some(seed) = self.randomize_layout.unwrap() {
                format!(" -Z layout-seed={seed}")
            } else {
                String::new()
            };

            println!(
                "Using concrete playback with --randomize-layout.\n\
                The produced tests will have to be played with the same rustc arguments:\n\
                -Z randomize-layout{random_seed}"
            );
        }

        if self.visualize && !self.common_args.enable_unstable {
            return Err(Error::raw(
                ErrorKind::MissingRequiredArgument,
                "Missing argument: --visualize now requires --enable-unstable
                    due to open issues involving incorrect results.",
            ));
        }

        if self.mir_linker {
            print_obsolete(&self.common_args, "--mir-linker");
        }

        if self.legacy_linker {
            print_obsolete(&self.common_args, "--legacy-linker");
        }

        // TODO: these conflicting flags reflect what's necessary to pass current tests unmodified.
        // We should consider improving the error messages slightly in a later pull request.
        if natives_unwind && extra_unwind {
            return Err(Error::raw(
                ErrorKind::ArgumentConflict,
                "Conflicting flags: unwind flags provided to kani and in --cbmc-args.",
            ));
        }
        if self.cbmc_args.contains(&OsString::from("--function")) {
            return Err(Error::raw(
                ErrorKind::ArgumentConflict,
                "Invalid flag: --function should be provided to Kani directly, not via --cbmc-args.",
            ));
        }
        if self.common_args.quiet && self.concrete_playback == Some(ConcretePlaybackMode::Print) {
            return Err(Error::raw(
                ErrorKind::ArgumentConflict,
                "Conflicting options: --concrete-playback=print and --quiet.",
            ));
        }
        if self.concrete_playback.is_some() && self.output_format == OutputFormat::Old {
            return Err(Error::raw(
                ErrorKind::ArgumentConflict,
                "Conflicting options: --concrete-playback isn't compatible with \
                --output-format=old.",
            ));
        }
        if self.concrete_playback.is_some() && self.jobs() != Some(1) {
            // Concrete playback currently embeds a lot of assumptions about the order in which harnesses get called.
            return Err(Error::raw(
                ErrorKind::ArgumentConflict,
                "Conflicting options: --concrete-playback isn't compatible with --jobs.",
            ));
        }
        if self.jobs.is_some() && self.output_format != OutputFormat::Terse {
            // More verbose output formats make it hard to interpret output right now when run in parallel.
            // This can be removed when we change up how results are printed.
            return Err(Error::raw(
                ErrorKind::ArgumentConflict,
                "Conflicting options: --jobs requires `--output-format=terse`",
            ));
        }
        if let Some(out_dir) = &self.target_dir {
            if out_dir.exists() && !out_dir.is_dir() {
                return Err(Error::raw(
                    ErrorKind::InvalidValue,
                    &format!(
                        "Invalid argument: `--target-dir` argument `{}` is not a directory",
                        out_dir.display()
                    ),
                ));
            }
        }

        if self.concrete_playback.is_some()
            && !self.common_args.unstable_features.contains(&UnstableFeatures::ConcretePlayback)
        {
            if self.common_args.enable_unstable {
                print_deprecated(&self.common_args, "--enable-unstable", "-Z concrete-playback");
            } else {
                return Err(Error::raw(
                    ErrorKind::MissingRequiredArgument,
                    "The `--concrete-playback` argument is unstable and requires `-Z \
                concrete-playback` to be used.",
                ));
            }
        }

        if !self.c_lib.is_empty()
            && !self.common_args.unstable_features.contains(&UnstableFeatures::CFfi)
        {
            if self.common_args.enable_unstable {
                print_deprecated(&self.common_args, "`--enable-unstable`", "-Z c-ffi");
            } else {
                return Err(Error::raw(
                    ErrorKind::MissingRequiredArgument,
                    "The `--c-lib` argument is unstable and requires `-Z c-ffi` to enable \
                unstable C-FFI support.",
                ));
            }
        }

        if self.coverage
            && !self.common_args.unstable_features.contains(&UnstableFeatures::LineCoverage)
        {
            return Err(Error::raw(
                ErrorKind::MissingRequiredArgument,
                "The `--coverage` argument is unstable and requires `-Z \
            line-coverage` to be used.",
            ));
        }

        Ok(())
    }
}

/// clap parser for `CbmcSolver`
#[derive(Clone, Debug)]
pub struct CbmcSolverValueParser(Vec<PossibleValue>);

impl CbmcSolverValueParser {
    pub fn new(values: impl Into<CbmcSolverValueParser>) -> Self {
        values.into()
    }
}

impl TypedValueParser for CbmcSolverValueParser {
    type Value = CbmcSolver;

    fn parse_ref(
        &self,
        cmd: &clap::builder::Command,
        arg: Option<&clap::builder::Arg>,
        value: &std::ffi::OsStr,
    ) -> Result<Self::Value, clap::error::Error> {
        let value = value.to_str().unwrap();
        // `value` is one of the possible `CbmcSolver` values or `bin=<binary>`
        let segments: Vec<&str> = value.split('=').collect();

        let mut err = clap::Error::new(ErrorKind::InvalidValue).with_cmd(cmd);
        err.insert(ContextKind::InvalidArg, ContextValue::String(arg.unwrap().to_string()));
        err.insert(ContextKind::InvalidValue, ContextValue::String(value.to_string()));

        if segments.len() == 2 {
            if segments[0] != "bin" {
                return Err(err);
            }
            return Ok(CbmcSolver::Binary(segments[1].into()));
        } else if segments.len() == 1 {
            let solver = CbmcSolver::from_str(value);
            return solver.map_err(|_| err);
        }
        Err(err)
    }

    /// Used for the help message
    fn possible_values(&self) -> Option<Box<dyn Iterator<Item = PossibleValue> + '_>> {
        Some(Box::new(self.0.iter().cloned()))
    }
}

impl<I, T> From<I> for CbmcSolverValueParser
where
    I: IntoIterator<Item = T>,
    T: Into<PossibleValue>,
{
    fn from(values: I) -> Self {
        Self(values.into_iter().map(|t| t.into()).collect())
    }
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::*;

    #[test]
    fn check_arg_parsing() {
        let a = StandaloneArgs::try_parse_from(vec![
            "kani",
            "file.rs",
            "--enable-unstable",
            "--cbmc-args",
            "--multiple",
            "args",
            "--here",
        ])
        .unwrap();
        assert_eq!(a.verify_opts.cbmc_args, vec!["--multiple", "args", "--here"]);
        let _b = StandaloneArgs::try_parse_from(vec![
            "kani",
            "file.rs",
            "--enable-unstable",
            "--cbmc-args",
        ])
        .unwrap();
        // no assertion: the above might fail if it fails to allow 0 args to cbmc-args
    }

    /// Ensure users can pass multiple harnesses options and that the value is accumulated.
    #[test]
    fn check_multiple_harnesses() {
        let args =
            StandaloneArgs::try_parse_from("kani input.rs --harness a --harness b".split(" "))
                .unwrap();
        assert_eq!(args.verify_opts.harnesses, vec!["a".to_owned(), "b".to_owned()]);
    }

    #[test]
    fn check_multiple_harnesses_without_flag_fail() {
        let result = StandaloneArgs::try_parse_from(
            "kani input.rs --harness harness_1 harness_2".split(" "),
        );
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), ErrorKind::UnknownArgument);
    }

    #[test]
    fn check_multiple_packages() {
        // accepts repeated:
        let a = CargoKaniArgs::try_parse_from(vec!["cargo-kani", "-p", "a", "-p", "b"]).unwrap();
        assert_eq!(a.verify_opts.cargo.package, vec!["a".to_owned(), "b".to_owned()]);
        let b = CargoKaniArgs::try_parse_from(vec![
            "cargo-kani",
            "-p",
            "a", // no -p
            "b",
        ]);
        // BUG: should not accept sequential:
        // Related: https://github.com/model-checking/kani/issues/2025
        // This assert should ideally return an error, and the assertion should instead be assert!(b.is_err())
        assert!(b.is_ok());
    }

    fn check(args: &str, require_unstable: bool, pred: fn(StandaloneArgs) -> bool) {
        let mut res = parse_unstable_disabled(&args);
        if require_unstable {
            // Should fail without --enable-unstable.
            assert_eq!(res.unwrap_err().kind(), ErrorKind::MissingRequiredArgument);
            // Should succeed with --enable-unstable.
            res = parse_unstable_enabled(&args);
        }
        assert!(res.is_ok());
        assert!(pred(res.unwrap()));
    }

    macro_rules! check_unstable_flag {
        ($args:expr, $name:ident) => {
            check($args, true, |p| p.verify_opts.$name)
        };
    }

    macro_rules! check_opt {
        ($args:expr, $require_unstable:expr, $name:ident, $expected:expr) => {
            check($args, $require_unstable, |p| p.verify_opts.$name == $expected)
        };
    }

    #[test]
    fn check_abs_type() {
        // Since we manually implemented this, consistency check it
        for t in AbstractionType::variants() {
            assert_eq!(t, format!("{}", AbstractionType::from_str(t, false).unwrap()));
        }
        check_opt!("--abs-type std", false, abs_type, AbstractionType::Std);
        check_opt!("--abs-type kani", false, abs_type, AbstractionType::Kani);
        check_opt!("--abs-type c-ffi", false, abs_type, AbstractionType::CFfi);
        check_opt!("--abs-type no-back", false, abs_type, AbstractionType::NoBack);
    }

    #[test]
    fn check_dry_run_fails() {
        // We don't support --dry-run anymore but we print a friendly reminder for now.
        let args = vec!["kani", "file.rs", "--dry-run"];
        let err =
            StandaloneArgs::try_parse_from(&args).unwrap().verify_opts.validate().unwrap_err();
        assert_eq!(err.kind(), ErrorKind::ValueValidation);
    }

    /// Kani should fail if the argument given is not a file.
    #[test]
    fn check_invalid_input_fails() {
        let args = vec!["kani", "."];
        let err = StandaloneArgs::try_parse_from(&args).unwrap().validate().unwrap_err();
        assert_eq!(err.kind(), ErrorKind::InvalidValue);
    }

    #[test]
    fn check_unwind_conflicts() {
        // --unwind cannot be called without --harness
        let args = vec!["kani", "file.rs", "--unwind", "3"];
        let err = StandaloneArgs::try_parse_from(args).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::MissingRequiredArgument);
    }

    fn parse_unstable_disabled(args: &str) -> Result<StandaloneArgs, Error> {
        let args = format!("kani file.rs {args}");
        StandaloneArgs::try_parse_from(args.split(' '))
    }

    fn parse_unstable_enabled(args: &str) -> Result<StandaloneArgs, Error> {
        let args = format!("kani --enable-unstable file.rs {args}");
        StandaloneArgs::try_parse_from(args.split(' '))
    }

    #[test]
    fn check_abs_unstable() {
        check_unstable_flag!("--use-abs", use_abs);
    }

    #[test]
    fn check_restrict_vtable_unstable() {
        check_unstable_flag!("--restrict-vtable", restrict_vtable);
    }

    #[test]
    fn check_restrict_cbmc_args() {
        check_opt!("--cbmc-args --json-ui", true, cbmc_args, vec!["--json-ui"]);
    }

    #[test]
    fn check_disable_slicing_unstable() {
        check_unstable_flag!("--no-slice-formula", no_slice_formula);
    }

    #[test]
    fn check_concrete_playback_unstable() {
        let check = |input: &str| {
            let args = input.split_whitespace();
            let result = StandaloneArgs::try_parse_from(args).unwrap().validate();
            assert!(result.is_err());

            let kind = result.unwrap_err().kind();
            assert!(matches!(kind, ErrorKind::MissingRequiredArgument), "Found {kind:?}");
        };

        check("kani file.rs --concrete-playback=inplace");
        check("kani file.rs --concrete-playback=print");
    }

    /// Check if parsing the given argument string results in the given error.
    fn expect_validation_error(arg: &str, err: ErrorKind) {
        let args = StandaloneArgs::try_parse_from(arg.split_whitespace()).unwrap();
        assert_eq!(args.verify_opts.validate().unwrap_err().kind(), err);
    }

    #[test]
    fn check_concrete_playback_conflicts() {
        expect_validation_error(
            "kani --concrete-playback=print --quiet --enable-unstable test.rs",
            ErrorKind::ArgumentConflict,
        );
        expect_validation_error(
            "kani --concrete-playback=inplace --output-format=old --enable-unstable test.rs",
            ErrorKind::ArgumentConflict,
        );
    }

    #[test]
    fn check_enable_stubbing() {
        check_unstable_flag!("--enable-stubbing --harness foo", enable_stubbing);

        check_unstable_flag!("--enable-stubbing", enable_stubbing);

        // `--enable-stubbing` cannot be called with `--concrete-playback`
        let err =
            parse_unstable_enabled("--enable-stubbing --harness foo --concrete-playback=print")
                .unwrap_err();
        assert_eq!(err.kind(), ErrorKind::ArgumentConflict);
    }

    #[test]
    fn check_features_parsing() {
        fn parse(args: &[&str]) -> Vec<String> {
            CargoKaniArgs::try_parse_from(args).unwrap().verify_opts.cargo.features()
        }

        // spaces, commas, multiple repeated args, all ok
        assert_eq!(parse(&["kani", "--features", "a b c"]), ["a", "b", "c"]);
        assert_eq!(parse(&["kani", "--features", "a,b,c"]), ["a", "b", "c"]);
        assert_eq!(parse(&["kani", "--features", "a", "--features", "b,c"]), ["a", "b", "c"]);
        assert_eq!(parse(&["kani", "--features", "a b", "-Fc"]), ["a", "b", "c"]);
    }

    #[test]
    fn check_kani_playback() {
        let input = "kani playback file.rs -- dummy".split_whitespace();
        let args = StandaloneArgs::try_parse_from(input).unwrap();
        assert_eq!(args.input, None);
        assert!(matches!(args.command, Some(StandaloneSubcommand::Playback(..))));
    }

    #[test]
    fn check_standalone_does_not_accept_cargo_opts() {
        fn check_invalid_args<'a, I>(args: I)
        where
            I: IntoIterator<Item = &'a str>,
        {
            let err = StandaloneArgs::try_parse_from(args).unwrap().validate().unwrap_err();
            assert_eq!(err.kind(), ErrorKind::UnknownArgument)
        }

        check_invalid_args("kani input.rs --bins".split_whitespace());
        check_invalid_args("kani input.rs --bin Binary".split_whitespace());
        check_invalid_args("kani input.rs --lib".split_whitespace());

        check_invalid_args("kani input.rs --all-features".split_whitespace());
        check_invalid_args("kani input.rs --no-default-features".split_whitespace());
        check_invalid_args("kani input.rs --features feat".split_whitespace());
        check_invalid_args("kani input.rs --manifest-path pkg/Cargo.toml".split_whitespace());
        check_invalid_args("kani input.rs --workspace".split_whitespace());
        check_invalid_args("kani input.rs --package foo".split_whitespace());
        check_invalid_args("kani input.rs --exclude bar --workspace".split_whitespace());
    }
}
