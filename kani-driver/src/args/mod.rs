// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Module that define Kani's command line interface. This includes all subcommands.

pub mod autoharness_args;
pub mod cargo;
pub mod common;
pub mod list_args;
pub mod playback_args;
pub mod std_args;

use self::common::*;
use crate::args::cargo::CargoTargetArgs;
use crate::util::warning;
use cargo::CargoCommonArgs;
use clap::builder::{PossibleValue, TypedValueParser};
use clap::{ValueEnum, error::ContextKind, error::ContextValue, error::Error, error::ErrorKind};
use kani_metadata::CbmcSolver;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::Duration;
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

#[allow(dead_code)]
pub fn print_obsolete(verbosity: &CommonArgs, option: &str) {
    if !verbosity.quiet {
        warning(&format!(
            "The `{option}` option is obsolete. This option no longer has any effect and should be removed"
        ))
    }
}

#[allow(dead_code)]
pub fn print_deprecated(verbosity: &CommonArgs, option: &str, version: &str, alternative: &str) {
    if !verbosity.quiet {
        warning(&format!(
            "The `{option}` option has been deprecated since {version} and will be removed soon. \
            Consider {alternative} instead"
        ))
    }
}

/// First step in two-phase stabilization.
/// When an unstable option is first stabilized, print this warning that `-Z unstable-options` has no effect.
/// This warning should last for one release only; in the next Kani release, remove it.
pub fn print_stabilized_option_warning(verbosity: &CommonArgs, option: &str, version: &str) {
    if !verbosity.quiet {
        warning(&format!(
            "The `--{option}` option has been stable since {version} and no longer requires {} to enable. \
            Remove it unless it is needed for another unstable option.",
            UnstableFeature::UnstableOptions.as_argument_string(),
        ))
    }
}

/// First step in two-phase stabilization.
/// When an unstable feature is first stabilized, print this warning that `-Z {feature}` has no effect.
/// This warning should last for one release only; in the next Kani release, remove it.
pub fn print_stabilized_feature_warning(
    verbosity: &CommonArgs,
    feature: UnstableFeature,
    version: &str,
) {
    if !verbosity.quiet {
        warning(&format!(
            "The `{feature}` feature has been stable since {version} and no longer requires {} to enable",
            feature.as_argument_string()
        ))
    }
}

// By default we configure CBMC to use 16 bits to represent the object bits in pointers.
const DEFAULT_OBJECT_BITS: u32 = 16;

#[derive(Clone, Copy, Debug, PartialEq, Eq, strum_macros::EnumString)]
enum TimeUnit {
    #[strum(serialize = "s")]
    Seconds,
    #[strum(serialize = "m")]
    Minutes,
    #[strum(serialize = "h")]
    Hours,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Timeout {
    value: u32,
    unit: TimeUnit,
}

impl FromStr for Timeout {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let last_char = s.chars().last().unwrap();
        let (value_str, unit_str) = if last_char.is_ascii_digit() {
            // no suffix
            (s, "s")
        } else {
            s.split_at(s.len() - 1)
        };
        let value = value_str.parse::<u32>().map_err(|_| "Invalid timeout value")?;

        let unit = TimeUnit::from_str(unit_str).map_err(
            |_| "Invalid time unit. Use 's' for seconds, 'm' for minutes, or 'h' for hours",
        )?;

        Ok(Timeout { value, unit })
    }
}

impl From<Timeout> for Duration {
    fn from(timeout: Timeout) -> Self {
        match timeout.unit {
            TimeUnit::Seconds => Duration::from_secs(timeout.value as u64),
            TimeUnit::Minutes => Duration::from_secs(timeout.value as u64 * 60),
            TimeUnit::Hours => Duration::from_secs(timeout.value as u64 * 3600),
        }
    }
}

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

    #[arg(long, hide = true)]
    pub crate_name: Option<String>,
}

/// Kani takes optional subcommands to request specialized behavior.
/// When no subcommand is provided, there is an implied verification subcommand.
#[derive(Debug, clap::Subcommand)]
pub enum StandaloneSubcommand {
    /// Create and run harnesses automatically for eligible functions. Implies -Z function-contracts and -Z loop-contracts.
    Autoharness(Box<autoharness_args::StandaloneAutoharnessArgs>),
    /// List contracts and harnesses.
    List(Box<list_args::StandaloneListArgs>),
    /// Execute concrete playback testcases of a local crate.
    Playback(Box<playback_args::KaniPlaybackArgs>),
    /// Verify the rust standard library.
    VerifyStd(Box<std_args::VerifyStdArgs>),
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
    /// Create and run harnesses automatically for eligible functions. Implies -Z function-contracts and -Z loop-contracts.
    /// See https://model-checking.github.io/kani/reference/experimental/autoharness.html for documentation.
    Autoharness(Box<autoharness_args::CargoAutoharnessArgs>),

    /// List contracts and harnesses.
    List(Box<list_args::CargoListArgs>),

    /// Execute concrete playback testcases of a local package.
    Playback(Box<playback_args::CargoPlaybackArgs>),
}

// Common arguments for invoking Kani for verification purpose. This gets put into KaniContext,
// whereas anything above is "local" to "main"'s control flow.
// When adding an argument to this struct, make sure that it's in alphabetical order as displayed to the user when running --help.
#[derive(Debug, clap::Args)]
#[clap(next_help_heading = "Verification Options")]
pub struct VerificationArgs {
    /// Link external C files referenced by Rust code.
    /// This is an experimental feature and requires `-Z c-ffi` to be used
    #[arg(long, hide = true, num_args(1..))]
    pub c_lib: Vec<PathBuf>,

    /// Pass through directly to CBMC; must be the last flag.
    /// This feature is unstable and it requires `-Z unstable-options` to be used
    #[arg(
        long,
        allow_hyphen_values = true,
        num_args(0..)
    )]
    // consumes everything
    pub cbmc_args: Vec<OsString>,

    /// Generate concrete playback unit test.
    /// If value supplied is 'print', Kani prints the unit test to stdout.
    /// If value supplied is 'inplace', Kani automatically adds the unit test to your source code.
    /// This option does not work with `--output-format old`.
    #[arg(long, ignore_case = true, value_enum)]
    pub concrete_playback: Option<ConcretePlaybackMode>,

    /// Enable Kani coverage output alongside verification result
    #[arg(long, hide_short_help = true)]
    pub coverage: bool,

    /// Specify the value used for loop unwinding in CBMC
    #[arg(long)]
    pub default_unwind: Option<u32>,

    /// When specified, the harness filter will only match the exact fully qualified name of a harness
    #[arg(long, requires("harnesses"))]
    pub exact: bool,

    /// Enable extra pointer checks such as invalid pointers in relation operations and pointer
    /// arithmetic overflow.
    /// This feature is unstable and it may yield false counter examples. It requires
    /// `-Z unstable-options` to be used
    #[arg(long, hide_short_help = true)]
    pub extra_pointer_checks: bool,

    /// Stop the verification process as soon as one of the harnesses fails.
    #[arg(long)]
    pub fail_fast: bool,

    /// Force Kani to rebuild all packages before the verification.
    #[arg(long)]
    pub force_build: bool,

    /// Generate C file equivalent to inputted program for debug purpose.
    /// This feature is unstable, and it requires `-Z unstable-options` to be used
    #[arg(long, hide_short_help = true)]
    pub gen_c: bool,

    /// If specified, only run harnesses that match this filter. This option can be provided
    /// multiple times, which will run all tests matching any of the filters.
    /// If used with --exact, the harness filter will only match the exact fully qualified name of a harness.
    #[arg(long = "harness", num_args(1), value_name = "HARNESS_FILTER")]
    pub harnesses: Vec<String>,

    /// Timeout for each harness with optional suffix ('s': seconds, 'm': minutes, 'h': hours). Default is seconds. This option is experimental and requires `-Z unstable-options` to be used.
    #[arg(long)]
    pub harness_timeout: Option<Timeout>,

    /// Do not error out for crates containing `global_asm!`.
    /// This option may impact the soundness of the analysis and may cause false proofs and/or counterexamples
    #[arg(long, hide_short_help = true)]
    pub ignore_global_asm: bool,

    /// Number of threads to spawn to verify harnesses in parallel.
    /// Omit the flag entirely to run sequentially (i.e. one thread).
    /// Pass -j to run with the thread pool's default number of threads.
    /// Pass -j <N> to specify N threads.
    #[arg(short, long, hide_short_help = true)]
    pub jobs: Option<Option<usize>>,

    /// Keep temporary files generated throughout Kani process. This is already the default
    /// behavior for `cargo-kani`.
    #[arg(long, hide_short_help = true)]
    pub keep_temps: bool,

    /// Do not assert the function contracts of dependencies. Requires -Z function-contracts.
    #[arg(long, hide_short_help = true)]
    pub no_assert_contracts: bool,

    /// Turn off assertion reachability checks
    #[arg(long)]
    pub no_assertion_reach_checks: bool,

    /// Run Kani without codegen. Useful for quick feedback on whether the code would compile successfully (similar to `cargo check`).
    /// This feature is unstable and requires `-Z unstable-options` to be used
    #[arg(long, hide_short_help = true)]
    pub no_codegen: bool,

    /// Disable restricting the targets of virtual table function pointer calls
    #[arg(long, hide_short_help = true)]
    pub no_restrict_vtable: bool,

    /// Disable CBMC's slice formula which prevents values from being assigned to redundant variables in traces.
    #[arg(long, hide_short_help = true)]
    pub no_slice_formula: bool,

    /// Kani will only compile the crate. No verification will be performed
    #[arg(long, hide_short_help = true)]
    pub only_codegen: bool,

    /// Toggle between different styles of output
    #[arg(long, default_value = "regular", ignore_case = true, value_enum)]
    pub output_format: OutputFormat,

    /// Write verification results into per-harness files, rather than to stdout
    #[arg(long, hide_short_help = true)]
    pub output_into_files: bool,

    /// Print final LLBC for Lean backend. This requires the `-Z lean` option.
    #[arg(long, hide = true)]
    pub print_llbc: bool,

    /// Randomize the layout of structures. This option can help catching code that relies on
    /// a specific layout chosen by the compiler that is not guaranteed to be stable in the future.
    /// If a value is given, it will be used as the seed for randomization
    /// See the `-Z randomize-layout` and `-Z layout-seed` arguments of the rust compiler.
    #[arg(long)]
    pub randomize_layout: Option<Option<u64>>,

    /// Restrict the targets of virtual table function pointer calls.
    /// This feature is unstable and it requires `-Z restrict-vtable` to be used
    #[arg(long, hide = true, conflicts_with = "no_restrict_vtable")]
    pub restrict_vtable: bool,

    /// Execute CBMC's sanity checks to ensure the goto-program we generate is correct.
    #[arg(long, hide_short_help = true)]
    pub run_sanity_checks: bool,

    /// Specify the CBMC solver to use. Overrides the harness `solver` attribute.
    /// If no solver is specified (with --solver or harness attribute), Kani will use CaDiCaL.
    #[arg(long, value_parser = CbmcSolverValueParser::new(CbmcSolver::VARIANTS))]
    pub solver: Option<CbmcSolver>,

    /// Synthesize loop contracts for all loops.
    #[arg(
        long,
        hide_short_help = true,
        conflicts_with("unwind"),
        conflicts_with("default_unwind")
    )]
    pub synthesize_loop_contracts: bool,

    /// Directory for all generated artifacts.
    #[arg(long)]
    pub target_dir: Option<PathBuf>,

    /// Enable test function verification. Only use this option when the entry point is a test function
    #[arg(long)]
    pub tests: bool,

    /// Specify the value used for loop unwinding for the specified harness in CBMC
    #[arg(long, requires("harnesses"))]
    pub unwind: Option<u32>,

    /// Write the GotoC symbol table to a file in JSON format instead of goto binary format.
    #[arg(long, hide = true)]
    pub write_json_symtab: bool,

    #[command(flatten)]
    pub checks: CheckArgs,

    #[command(flatten)]
    pub common_args: CommonArgs,

    /// Arguments to pass down to Cargo
    #[command(flatten)]
    pub cargo: CargoCommonArgs,

    /// Arguments used to select Cargo target.
    #[command(flatten)]
    pub target: CargoTargetArgs,
}

impl VerificationArgs {
    pub fn restrict_vtable(&self) -> bool {
        self.common_args.unstable_features.contains(UnstableFeature::RestrictVtable)
            && !self.no_restrict_vtable
    }

    /// Assertion reachability checks should be disabled
    pub fn assertion_reach_checks(&self) -> bool {
        !self.no_assertion_reach_checks
    }

    /// Suppress our default value, if the user has supplied it explicitly in --cbmc-args
    pub fn cbmc_object_bits(&self) -> Option<u32> {
        if self.cbmc_args.contains(&OsString::from("--object-bits")) {
            None
        } else {
            Some(DEFAULT_OBJECT_BITS)
        }
    }

    /// Given an argument, warn if UnstableFeature::UnstableOptions is enabled.
    /// This is for cases where the option was previously unstable but has since been stabilized.
    pub fn check_unnecessary_unstable_option(&self, enabled: bool, option: &str) {
        fn stabilization_version(option: &str) -> Option<String> {
            match option {
                "jobs" => Some("0.63.0".to_string()),
                _ => None,
            }
        }
        let stabilization_version = stabilization_version(option);
        if let Some(version) = stabilization_version
            && enabled
            && self.common_args.unstable_features.contains(UnstableFeature::UnstableOptions)
        {
            print_stabilized_option_warning(&self.common_args, option, &version)
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

    /// Are experimental function contracts enabled?
    pub fn is_function_contracts_enabled(&self) -> bool {
        self.common_args.unstable_features.contains(UnstableFeature::FunctionContracts)
    }

    /// Is experimental stubbing enabled?
    pub fn is_stubbing_enabled(&self) -> bool {
        self.common_args.unstable_features.contains(UnstableFeature::Stubbing)
            || self.is_function_contracts_enabled()
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

#[derive(Debug, clap::Args)]
#[clap(next_help_heading = "Memory Checks")]
pub struct CheckArgs {
    // Rust argument parsers (/clap) don't have the convenient '--flag' and '--no-flag' boolean pairs, so approximate
    // We're put both here then create helper functions to "intepret"
    /// Turn on all default checks
    #[arg(long, hide = true)]
    pub default_checks: bool,
    /// Turn off all default checks
    #[arg(long)]
    pub no_default_checks: bool,

    /// Turn on default memory safety checks
    #[arg(long, hide = true)]
    pub memory_safety_checks: bool,
    /// Turn off default memory safety checks
    #[arg(long)]
    pub no_memory_safety_checks: bool,

    /// Turn on default overflow checks
    #[arg(long, hide = true)]
    pub overflow_checks: bool,
    /// Turn off default overflow checks
    #[arg(long)]
    pub no_overflow_checks: bool,

    /// Turn on undefined function checks
    #[arg(long, hide = true)]
    pub undefined_function_checks: bool,
    /// Turn off undefined function checks
    #[arg(long)]
    pub no_undefined_function_checks: bool,

    /// Turn on default unwinding checks
    #[arg(long, hide = true)]
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
    pub fn print_deprecated(&self, verbosity: &CommonArgs) {
        let deprecation_version = "0.63.0";
        let alternative = "omitting the argument, since this is already the default behavior";
        if self.default_checks {
            print_deprecated(verbosity, "--default-checks", deprecation_version, alternative);
        }
        if self.memory_safety_checks {
            print_deprecated(verbosity, "--memory-safety-checks", deprecation_version, alternative);
        }
        if self.overflow_checks {
            print_deprecated(verbosity, "--overflow-checks", deprecation_version, alternative);
        }
        if self.undefined_function_checks {
            print_deprecated(
                verbosity,
                "--undefined-function-checks",
                deprecation_version,
                alternative,
            );
        }
        if self.unwinding_checks {
            print_deprecated(verbosity, "--unwinding-checks", deprecation_version, alternative);
        }
    }
}

/// Utility function to error out on arguments that are invalid Cargo specific.
///
/// We currently define a bunch of cargo specific arguments as part of the overall arguments,
/// however, they are invalid in the Kani standalone usage. Explicitly check them for now.
/// TODO: Remove this as part of <https://github.com/model-checking/kani/issues/1831>
fn check_no_cargo_opt(is_set: bool, name: &str) -> Result<(), Error> {
    if is_set {
        Err(Error::raw(
            ErrorKind::UnknownArgument,
            format!("argument `{name}` cannot be used with standalone Kani."),
        ))
    } else {
        Ok(())
    }
}

impl ValidateArgs for StandaloneArgs {
    fn validate(&self) -> Result<(), Error> {
        self.verify_opts.validate()?;

        match &self.command {
            Some(StandaloneSubcommand::VerifyStd(args)) => args.validate()?,
            Some(StandaloneSubcommand::List(args)) => args.validate()?,
            Some(StandaloneSubcommand::Autoharness(args)) => args.validate()?,
            // TODO: Invoke PlaybackArgs::validate()
            None | Some(StandaloneSubcommand::Playback(..)) => {}
        };

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
        if let Some(input) = &self.input
            && !input.is_file()
        {
            return Err(Error::raw(
                ErrorKind::InvalidValue,
                format!(
                    "Invalid argument: Input invalid. `{}` is not a regular file.",
                    input.display()
                ),
            ));
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
            CargoKaniSubcommand::Autoharness(autoharness) => autoharness.validate(),
            CargoKaniSubcommand::Playback(playback) => playback.validate(),
            CargoKaniSubcommand::List(list) => list.validate(),
        }
    }
}

impl ValidateArgs for CargoKaniArgs {
    fn validate(&self) -> Result<(), Error> {
        self.verify_opts.validate()?;
        self.command.validate()?;
        Ok(())
    }
}

impl ValidateArgs for VerificationArgs {
    fn validate(&self) -> Result<(), Error> {
        self.common_args.validate()?;

        // check_unstable() calls: for each unstable option, check that the requisite unstable feature is provided.
        let unstable = || -> Result<(), Error> {
            self.common_args.check_unstable(
                self.concrete_playback.is_some(),
                "concrete-playback",
                UnstableFeature::ConcretePlayback,
            )?;

            self.common_args.check_unstable(
                !self.c_lib.is_empty(),
                "c-lib",
                UnstableFeature::CFfi,
            )?;

            self.common_args.check_unstable(
                self.gen_c,
                "gen-c",
                UnstableFeature::UnstableOptions,
            )?;

            self.common_args.check_unstable(
                !self.cbmc_args.is_empty(),
                "cbmc-args",
                UnstableFeature::UnstableOptions,
            )?;

            self.common_args.check_unstable(
                self.no_codegen,
                "no-codegen",
                UnstableFeature::UnstableOptions,
            )?;

            self.common_args.check_unstable(
                self.extra_pointer_checks,
                "extra-pointer-checks",
                UnstableFeature::UnstableOptions,
            )?;

            self.common_args.check_unstable(
                self.ignore_global_asm,
                "ignore-asm",
                UnstableFeature::UnstableOptions,
            )?;

            self.common_args.check_unstable(
                self.run_sanity_checks,
                "run-sanity-checks",
                UnstableFeature::UnstableOptions,
            )?;

            self.common_args.check_unstable(
                self.no_slice_formula,
                "no-slice-formula",
                UnstableFeature::UnstableOptions,
            )?;

            self.common_args.check_unstable(
                self.synthesize_loop_contracts,
                "synthesize-loop-contracts",
                UnstableFeature::UnstableOptions,
            )?;

            self.common_args.check_unstable(
                self.no_restrict_vtable,
                "no-restrict-vtable",
                UnstableFeature::RestrictVtable,
            )?;

            self.common_args.check_unstable(
                self.coverage,
                "coverage",
                UnstableFeature::SourceCoverage,
            )?;
            self.common_args.check_unstable(
                self.output_into_files,
                "output-into-files",
                UnstableFeature::UnstableOptions,
            )?;
            self.common_args.check_unstable(
                self.print_llbc,
                "print-llbc",
                UnstableFeature::Lean,
            )?;
            self.common_args.check_unstable(
                self.harness_timeout.is_some(),
                "harness-timeout",
                UnstableFeature::UnstableOptions,
            )?;
            self.common_args.check_unstable(
                self.no_assert_contracts,
                "no-assert",
                UnstableFeature::FunctionContracts,
            )?;

            Ok(())
        };

        // Check for argument conflicts.
        let conflicting_options = || -> Result<(), Error> {
            let extra_unwind =
                self.cbmc_args.iter().any(|s| s.to_str().unwrap().starts_with("--unwind"));
            let natives_unwind = self.default_unwind.is_some() || self.unwind.is_some();

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
                    "Invalid flag: --function is not supported in Kani.",
                ));
            }
            if self.common_args.quiet && self.concrete_playback == Some(ConcretePlaybackMode::Print)
            {
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
            // TODO: error out for other CBMC-backend-specific arguments
            if self.common_args.unstable_features.contains(UnstableFeature::Lean)
                && !self.cbmc_args.is_empty()
            {
                return Err(Error::raw(
                    ErrorKind::ArgumentConflict,
                    format!(
                        "Conflicting options: --cbmc-args cannot be used with {}.",
                        UnstableFeature::Lean.as_argument_string()
                    ),
                ));
            }

            Ok(())
        };

        // Check for any deprecated/obsolete options, or providing an unstable flag that has since been stabilized.
        let deprecated_stabilized_obsolete = || -> Result<(), Error> {
            self.checks.print_deprecated(&self.common_args);
            self.check_unnecessary_unstable_option(self.jobs.is_some(), "jobs");

            if self.write_json_symtab {
                return Err(Error::raw(
                    ErrorKind::ValueValidation,
                    "The `--write-json-symtab` option is obsolete.",
                ));
            }

            if self.restrict_vtable {
                return Err(Error::raw(
                    ErrorKind::ValueValidation,
                    format!(
                        "The restrict-vtable option is obsolete. Use `{}` instead.",
                        UnstableFeature::RestrictVtable.as_argument_string()
                    ),
                ));
            }

            Ok(())
        };

        unstable()?;
        conflicting_options()?;
        deprecated_stabilized_obsolete()?;

        // Bespoke validations that don't fit into any of the categories above.
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

        if let Some(out_dir) = &self.target_dir
            && out_dir.exists()
            && !out_dir.is_dir()
        {
            return Err(Error::raw(
                ErrorKind::InvalidValue,
                format!(
                    "Invalid argument: `--target-dir` argument `{}` is not a directory",
                    out_dir.display()
                ),
            ));
        }

        Ok(())
    }
}

pub(crate) fn validate_std_path(std_path: &Path) -> Result<(), Error> {
    if !std_path.exists() {
        Err(Error::raw(
            ErrorKind::InvalidValue,
            format!(
                "Invalid argument: `<STD_PATH>` argument `{}` does not exist",
                std_path.display()
            ),
        ))
    } else if !std_path.is_dir() {
        Err(Error::raw(
            ErrorKind::InvalidValue,
            format!(
                "Invalid argument: `<STD_PATH>` argument `{}` is not a directory",
                std_path.display()
            ),
        ))
    } else {
        let full_path = std_path.canonicalize()?;
        let dir = full_path.file_stem().unwrap();
        if dir != "library" {
            Err(Error::raw(
                ErrorKind::InvalidValue,
                format!(
                    "Invalid argument: Expected `<STD_PATH>` to point to the `library` folder \
                containing the standard library crates.\n\
                Found `{}` folder instead",
                    dir.to_string_lossy()
                ),
            ))
        } else {
            Ok(())
        }
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
            "-Zunstable-options",
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
            "-Zunstable-options",
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
        assert_eq!(result.unwrap_err().kind(), ErrorKind::ArgumentConflict);
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

    fn check(args: &str, feature: Option<UnstableFeature>, pred: fn(StandaloneArgs) -> bool) {
        let mut res = parse_unstable_disabled(args);
        if let Some(unstable) = feature {
            // Should fail without -Z unstable-options.
            assert_eq!(res.unwrap_err().kind(), ErrorKind::MissingRequiredArgument);
            // Should succeed with -Z unstable-options.
            res = parse_unstable_enabled(args, unstable);
        }
        assert!(res.is_ok());
        assert!(pred(res.unwrap()));
    }

    macro_rules! check_unstable_flag {
        ($args:expr, $name:ident) => {
            check($args, Some(UnstableFeature::UnstableOptions), |p| p.verify_opts.$name)
        };
    }

    macro_rules! check_opt {
        ($args:expr, $require_unstable:expr, $name:ident, $expected:expr) => {
            check($args, $require_unstable, |p| p.verify_opts.$name == $expected)
        };
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
        let parse_res = StandaloneArgs::try_parse_from(args.split(' '))?;
        parse_res.verify_opts.validate()?;
        Ok(parse_res)
    }

    fn parse_unstable_enabled(
        args: &str,
        unstable: UnstableFeature,
    ) -> Result<StandaloneArgs, Error> {
        let args = format!("kani -Z {unstable} file.rs {args}");
        let parse_res = StandaloneArgs::try_parse_from(args.split(' '))?;
        parse_res.verify_opts.validate()?;
        Ok(parse_res)
    }

    #[test]
    fn check_restrict_vtable_unstable() {
        let res = parse_unstable_enabled("--output-format=terse", UnstableFeature::RestrictVtable)
            .unwrap();
        assert!(res.verify_opts.restrict_vtable());

        let res = parse_unstable_enabled("--no-restrict-vtable", UnstableFeature::RestrictVtable)
            .unwrap();
        assert!(!res.verify_opts.restrict_vtable());
    }

    #[test]
    fn check_restrict_cbmc_args() {
        check_opt!(
            "--cbmc-args --json-ui",
            Some(UnstableFeature::UnstableOptions),
            cbmc_args,
            vec!["--json-ui"]
        );
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
            "kani --concrete-playback=print --quiet -Z concrete-playback test.rs",
            ErrorKind::ArgumentConflict,
        );
        expect_validation_error(
            "kani --concrete-playback=inplace --output-format=old -Z concrete-playback test.rs",
            ErrorKind::ArgumentConflict,
        );
    }

    #[test]
    fn check_enable_stubbing() {
        let res = parse_unstable_disabled("--harness foo").unwrap();
        assert!(!res.verify_opts.is_stubbing_enabled());

        let res = parse_unstable_disabled("--harness foo -Z stubbing").unwrap();
        assert!(res.verify_opts.is_stubbing_enabled());

        // `-Z stubbing` can now be called with concrete playback.
        let res = parse_unstable_disabled(
            "--harness foo --concrete-playback=print -Z concrete-playback -Z stubbing",
        )
        .unwrap();
        // Note that `res.validate()` fails because input file does not exist.
        assert!(matches!(res.verify_opts.validate(), Ok(())));
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

    #[test]
    fn check_cbmc_args_lean_backend() {
        let args = "kani input.rs -Z lean -Z unstable-options --cbmc-args --object-bits 10"
            .split_whitespace();
        let err = StandaloneArgs::try_parse_from(args).unwrap().validate().unwrap_err();
        assert_eq!(err.kind(), ErrorKind::ArgumentConflict);
    }

    #[test]
    fn check_no_assert_contracts() {
        let args = "kani input.rs --no-assert-contracts".split_whitespace();
        let err = StandaloneArgs::try_parse_from(args).unwrap().validate().unwrap_err();
        assert_eq!(err.kind(), ErrorKind::MissingRequiredArgument);
    }
}
