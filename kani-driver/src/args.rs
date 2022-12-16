// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[cfg(feature = "unsound_experiments")]
use crate::unsound_experiments::UnsoundExperimentArgs;

use clap::{error::Error, error::ErrorKind, CommandFactory, Parser, ValueEnum};
use std::ffi::OsString;
use std::path::PathBuf;

// By default we configure CBMC to use 16 bits to represent the object bits in pointers.
const DEFAULT_OBJECT_BITS: u32 = 16;

#[derive(Debug, Parser)]
#[command(
    version,
    name = "kani",
    about = "Verify a single Rust crate. For more information, see https://github.com/model-checking/kani",
    args_override_self = true
)]
pub struct StandaloneArgs {
    /// Rust file to verify
    pub input: PathBuf,

    #[command(flatten)]
    pub common_opts: KaniArgs,
}

#[derive(Debug, Parser)]
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
    pub common_opts: KaniArgs,
}

// cargo-kani takes optional subcommands to request specialized behavior
#[derive(Debug, Parser)]
pub enum CargoKaniSubcommand {
    #[command(hide = true)]
    Assess,
}

// Common arguments for invoking Kani. This gets put into KaniContext, whereas
// anything above is "local" to "main"'s control flow.
#[derive(Debug, Parser)]
pub struct KaniArgs {
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
        requires("enable_unstable"),
        conflicts_with_all(&["visualize"]),
        ignore_case = true,
        value_enum
    )]
    pub concrete_playback: Option<ConcretePlaybackMode>,
    /// Keep temporary files generated throughout Kani process. This is already the default
    /// behavior for `cargo-kani`.
    #[arg(long, hide_short_help = true)]
    pub keep_temps: bool,

    /// Produce full debug information
    #[arg(long)]
    pub debug: bool,
    /// Produces no output, just an exit code and requested artifacts; overrides --verbose
    #[arg(long, short)]
    pub quiet: bool,
    /// Output processing stages and commands, along with minor debug information
    #[arg(long, short, default_value_if("debug", "true", Some("true")))]
    pub verbose: bool,
    /// Enable usage of unstable options
    #[arg(long, hide_short_help = true)]
    pub enable_unstable: bool,

    /// We no longer support dry-run. Use `--verbose` to see the commands being printed during
    /// Kani execution.
    #[arg(long, hide = true)]
    pub dry_run: bool,

    /// Generate C file equivalent to inputted program.
    /// This feature is unstable and it requires `--enable-unstable` to be used
    #[arg(long, hide_short_help = true, requires("enable_unstable"),
        conflicts_with_all(&["function", "legacy_linker"])
)]
    pub gen_c: bool,

    /// Directory for all generated artifacts.
    #[arg(long)]
    pub target_dir: Option<PathBuf>,

    /// Toggle between different styles of output
    #[arg(long, default_value = "regular", ignore_case = true, value_enum)]
    pub output_format: OutputFormat,

    #[command(flatten)]
    pub checks: CheckArgs,

    #[cfg(feature = "unsound_experiments")]
    #[command(flatten)]
    pub unsound_experiments: UnsoundExperimentArgs,

    /// Entry point for verification (symbol name).
    /// This is an unstable feature. Consider using --harness instead
    #[arg(long, hide = true, requires("enable_unstable"))]
    pub function: Option<String>,
    /// Entry point for verification (proof harness)
    #[arg(long, conflicts_with = "function")]
    pub harness: Option<String>,

    /// Link external C files referenced by Rust code.
    /// This is an experimental feature and requires `--enable-unstable` to be used
    #[arg(long, hide = true, requires("enable_unstable"), num_args(1..))]
    pub c_lib: Vec<PathBuf>,
    /// Enable test function verification. Only use this option when the entry point is a test function
    #[arg(long)]
    pub tests: bool,
    /// Kani will only compile the crate. No verification will be performed
    #[arg(long, hide_short_help = true)]
    pub only_codegen: bool,

    /// Disable the new MIR Linker. Using this option may result in missing symbols from the
    /// `std` library. See <https://github.com/model-checking/kani/issues/1213> for more details.
    #[arg(long, hide = true)]
    pub legacy_linker: bool,

    /// Enable the new MIR Linker. This is already the default option and it will be removed once
    /// the linker is stable.
    /// The MIR Linker affects how Kani prunes the code to be analyzed. It also fixes previous
    /// issues with missing `std` function definitions.
    /// See <https://model-checking.github.io/kani/rfc/rfcs/0001-mir-linker.html> for more details.
    #[arg(long, conflicts_with("legacy_linker"), hide = true)]
    pub mir_linker: bool,

    /// Compiles Kani harnesses in all features of all packages selected on the command-line.
    #[arg(long)]
    pub all_features: bool,
    /// Run Kani on all packages in the workspace.
    #[arg(long)]
    pub workspace: bool,
    /// Run Kani on the specified packages.
    #[arg(long, short, conflicts_with("workspace"), num_args(1..))]
    pub package: Vec<String>,

    /// Specify the value used for loop unwinding in CBMC
    #[arg(long)]
    pub default_unwind: Option<u32>,
    /// Specify the value used for loop unwinding for the specified harness in CBMC
    #[arg(long, requires("harness"))]
    pub unwind: Option<u32>,
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

    /// Execute CBMC's sanity checks to ensure the goto-program we generate is correct.
    #[arg(long, hide_short_help = true, requires("enable_unstable"))]
    pub run_sanity_checks: bool,

    /// Disable CBMC's slice formula which prevents values from being assigned to redundant variables in traces.
    #[arg(long, hide_short_help = true, requires("enable_unstable"))]
    pub no_slice_formula: bool,

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
        requires("harness"),
        conflicts_with("concrete_playback")
    )]
    pub enable_stubbing: bool,
}

impl KaniArgs {
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

#[derive(Debug, Parser)]
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

impl StandaloneArgs {
    pub fn validate(&self) {
        self.common_opts.validate::<Self>();
        self.valid_input()
            .or_else(|e| -> Result<(), ()> { e.format(&mut Self::command()).exit() })
            .unwrap()
    }

    fn valid_input(&self) -> Result<(), Error> {
        if !self.input.is_file() {
            Err(Error::raw(
                ErrorKind::InvalidValue,
                &format!(
                    "Invalid argument: Input invalid. `{}` is not a regular file.",
                    self.input.display()
                ),
            ))
        } else {
            Ok(())
        }
    }
}
impl CargoKaniArgs {
    pub fn validate(&self) {
        self.common_opts.validate::<Self>();
        // --assess requires --enable-unstable, but the subcommand needs manual checking
        if (matches!(self.command, Some(CargoKaniSubcommand::Assess)) || self.common_opts.assess)
            && !self.common_opts.enable_unstable
        {
            Self::command()
                .error(
                    ErrorKind::MissingRequiredArgument,
                    "Assess is unstable and requires 'cargo kani --enable-unstable assess'",
                )
                .exit()
        }
    }
}
impl KaniArgs {
    pub fn validate<T: Parser>(&self) {
        self.validate_inner()
            .or_else(|e| -> Result<(), ()> { e.format(&mut T::command()).exit() })
            .unwrap()
    }

    fn validate_inner(&self) -> Result<(), Error> {
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
                -Z randomize-layout{}",
                random_seed
            );
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
        if self.quiet && self.concrete_playback == Some(ConcretePlaybackMode::Print) {
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

        if self.dry_run {
            return Err(Error::raw(
                ErrorKind::ValueValidation,
                "The `--dry-run` option is obsolete. Use --verbose instead.",
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

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_arg_parsing() {
        let a = StandaloneArgs::parse_from(vec![
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
            StandaloneArgs::parse_from(vec!["kani", "file.rs", "--enable-unstable", "--cbmc-args"]);
        // no assertion: the above might fail if it fails to allow 0 args to cbmc-args
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
            check($args, true, |p| p.common_opts.$name)
        };
    }

    macro_rules! check_opt {
        ($args:expr, $require_unstable:expr, $name:ident, $expected:expr) => {
            check($args, $require_unstable, |p| p.common_opts.$name == $expected)
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
        let err = StandaloneArgs::parse_from(&args).common_opts.validate_inner().unwrap_err();
        assert_eq!(err.kind(), ErrorKind::ValueValidation);
    }

    /// Kani should fail if the argument given is not a file.
    #[test]
    fn check_invalid_input_fails() {
        let args = vec!["kani", "."];
        let err = StandaloneArgs::parse_from(&args).valid_input().unwrap_err();
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
        check_opt!(
            "--concrete-playback inplace",
            true,
            concrete_playback,
            Some(ConcretePlaybackMode::InPlace)
        );
        check_opt!(
            "--concrete-playback print",
            true,
            concrete_playback,
            Some(ConcretePlaybackMode::Print)
        );
    }

    /// Check if parsing the given argument string results in the given error.
    fn expect_validation_error(arg: &str, err: ErrorKind) {
        let args = StandaloneArgs::parse_from(arg.split_whitespace());
        assert_eq!(args.common_opts.validate_inner().unwrap_err().kind(), err);
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

        // `--enable-stubbing` cannot be called without `--harness`
        let err = parse_unstable_enabled("--enable-stubbing").unwrap_err();
        assert_eq!(err.kind(), ErrorKind::MissingRequiredArgument);

        // `--enable-stubbing` cannot be called with `--concrete-playback`
        let err =
            parse_unstable_enabled("--enable-stubbing --harness foo --concrete-playback=print")
                .unwrap_err();
        assert_eq!(err.kind(), ErrorKind::ArgumentConflict);
    }
}
