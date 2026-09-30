// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Implements the subcommand handling of the autoharness subcommand

use std::path::PathBuf;

use crate::args::list_args::Format;
use crate::args::{ValidateArgs, VerificationArgs, validate_std_path};
use crate::util::warning;
use clap::{Error, Parser, error::ErrorKind};
use kani_metadata::{
    AUTOHARNESS_BOUNDED_ARBITRARY_BOUND, AUTOHARNESS_SLICE_BOUND, AUTOHARNESS_STR_BOUND,
    UnstableFeature,
};
use regex::Regex;

#[derive(Debug, Parser)]
pub struct CommonAutoharnessArgs {
    /// Only create automatic harnesses for functions that match the given regular expression.
    #[arg(long = "include-pattern", num_args(1), value_name = "PATTERN")]
    pub include_pattern: Vec<String>,

    /// Only create automatic harnesses for functions that do not match the given regular expression pattern.
    /// This option takes precedence over `--include-pattern`, i.e., Kani will first select all functions that match `--include-pattern`,
    /// then exclude those that match `--exclude-pattern.`
    #[arg(long = "exclude-pattern", num_args(1), value_name = "PATTERN")]
    pub exclude_pattern: Vec<String>,

    /// Also create automatic harnesses for functions whose arguments require *bounded*
    /// nondeterministic values, e.g. slice references (`&[T]`, `&str`). Such harnesses are
    /// marked "(bounded)" in the output, and their verification results only hold up to the
    /// bounds; a bug that requires a larger input will not be found.
    #[arg(long)]
    pub bounded_arguments: bool,

    /// The maximum length for nondeterministic `&[T]`/`&mut [T]` arguments generated under
    /// `--bounded-arguments`. Verification results only hold up to this bound.
    #[arg(
        long,
        default_value_t = AUTOHARNESS_SLICE_BOUND,
        value_name = "N",
        requires = "bounded_arguments"
    )]
    pub slice_bound: u64,

    /// The maximum length, in bytes, for nondeterministic `&str` arguments generated under
    /// `--bounded-arguments`. Verification results only hold up to this bound.
    #[arg(
        long,
        default_value_t = AUTOHARNESS_STR_BOUND,
        value_name = "N",
        requires = "bounded_arguments"
    )]
    pub string_bound: u64,

    /// The bound for nondeterministic arguments whose type implements `BoundedArbitrary`
    /// (`Vec<T>`, `String`, or user types deriving it) generated under `--bounded-arguments`.
    /// Verification results only hold up to this bound.
    #[arg(
        long,
        default_value_t = AUTOHARNESS_BOUNDED_ARBITRARY_BOUND,
        value_name = "N",
        requires = "bounded_arguments"
    )]
    pub bounded_arbitrary_bound: u64,

    /// Generate nondeterministic values for types without an Arbitrary implementation through
    /// one of the type's own constructors with nondeterministic arguments: either an inlined
    /// assert-guarded representation constructor (unsafe/doc-hidden/*_unchecked) whose
    /// assertions filter the arguments, or a checked public constructor (assuming it succeeds).
    /// Such harnesses are marked "(ctor)" in the output, and their verification results only
    /// cover values reachable through that constructor; a bug that requires a different value
    /// will not be found.
    #[arg(long)]
    pub constructor_args: bool,

    /// Check that values returned by verified functions satisfy their type's *mined*
    /// invariants: assertions over the type's own fields that at least two of its methods
    /// state. Failures are reported as a distinct property class; since the mined predicate
    /// is heuristic, a failure means the returned value would trip the type's own
    /// assertions when used, which may or may not be a bug in the returning function.
    #[arg(long)]
    pub check_invariants: bool,

    /// Run the `list` subcommand after generating the automatic harnesses. Note that this option implies --only-codegen.
    #[arg(long)]
    pub list: bool,

    /// The format of the `list` output. Requires --list.
    #[arg(long, default_value = "pretty", requires = "list")]
    pub format: Format,
}

/// The bounds automatic harnesses use for arguments that can only be generated up to a bound,
/// c.f. `--bounded-arguments`.
#[derive(Debug, Clone, Copy)]
pub struct AutoharnessBounds {
    /// The maximum length for `&[T]`/`&mut [T]` arguments.
    pub slice: u64,
    /// The maximum length, in bytes, for `&str` arguments.
    pub string: u64,
    /// The bound for arguments whose type implements `BoundedArbitrary`.
    pub bounded_arbitrary: u64,
}

impl Default for AutoharnessBounds {
    fn default() -> Self {
        Self {
            slice: AUTOHARNESS_SLICE_BOUND,
            string: AUTOHARNESS_STR_BOUND,
            bounded_arbitrary: AUTOHARNESS_BOUNDED_ARBITRARY_BOUND,
        }
    }
}

impl CommonAutoharnessArgs {
    pub fn bounds(&self) -> AutoharnessBounds {
        AutoharnessBounds {
            slice: self.slice_bound,
            string: self.string_bound,
            bounded_arbitrary: self.bounded_arbitrary_bound,
        }
    }
}

/// Automatically verify functions in a crate.
#[derive(Debug, Parser)]
pub struct CargoAutoharnessArgs {
    #[command(flatten)]
    pub common_autoharness_args: CommonAutoharnessArgs,

    #[command(flatten)]
    pub verify_opts: VerificationArgs,
}

/// Automatically verify functions in a file.
#[derive(Debug, Parser)]
pub struct StandaloneAutoharnessArgs {
    /// Rust crate's top file location.
    #[arg(required = true)]
    pub input: PathBuf,

    #[arg(long, hide = true)]
    pub crate_name: Option<String>,

    #[command(flatten)]
    pub common_autoharness_args: CommonAutoharnessArgs,

    /// Pass this flag to run the `autoharness` subcommand on the standard library.
    /// Ensure that the provided `input` is the `library` folder.
    #[arg(long)]
    pub std: bool,

    #[command(flatten)]
    pub verify_opts: VerificationArgs,
}

impl ValidateArgs for CommonAutoharnessArgs {
    fn validate(&self) -> Result<(), Error> {
        // Error gracefully if a pattern contains whitespace, since rustc_driver argument will panic later if we try to pass this back,
        // c.f. https://github.com/model-checking/kani/issues/4046
        for pattern in self.include_pattern.iter().chain(self.exclude_pattern.iter()) {
            if pattern.contains(char::is_whitespace) {
                return Err(Error::raw(
                    ErrorKind::InvalidValue,
                    "The `--include-pattern` and `--exclude-pattern` options do not support patterns with whitespace. \
                        Use regular expression pattern flags (e.g., . to match any character) instead.",
                ));
            }
            if let Err(e) = Regex::new(pattern) {
                return Err(Error::raw(
                    ErrorKind::InvalidValue,
                    format!("invalid autoharness regular expression pattern: {e}"),
                ));
            }
        }

        for include_pattern in self.include_pattern.iter() {
            for exclude_pattern in self.exclude_pattern.iter() {
                // Check if include pattern contains exclude pattern
                // This catches cases like include="foo::bar" exclude="bar" or include="foo" exclude="foo"
                if include_pattern.contains(exclude_pattern) {
                    warning(&format!(
                        "Include pattern '{include_pattern}' contains exclude pattern '{exclude_pattern}'. \
                            This combination will never match any functions since all functions matching \
                            the include pattern will also match the exclude pattern, and the exclude pattern takes precedence."
                    ));
                }
            }
        }

        // A bound of 0 leaves no room for a value (and none for a `&CStr`'s terminating NUL), so it
        // is rejected here rather than reaching the models.
        for (option, bound) in [
            ("--slice-bound", self.slice_bound),
            ("--string-bound", self.string_bound),
            ("--bounded-arbitrary-bound", self.bounded_arbitrary_bound),
        ] {
            if bound == 0 {
                return Err(Error::raw(
                    ErrorKind::InvalidValue,
                    format!("`{option}` must be at least 1."),
                ));
            }
        }

        Ok(())
    }
}

impl ValidateArgs for CargoAutoharnessArgs {
    fn validate(&self) -> Result<(), Error> {
        self.common_autoharness_args.validate()?;
        self.verify_opts.validate()?;
        if !self.verify_opts.common_args.unstable_features.contains(UnstableFeature::Autoharness) {
            return Err(Error::raw(
                ErrorKind::MissingRequiredArgument,
                format!(
                    "The `autoharness` subcommand is unstable and requires -Z {}",
                    UnstableFeature::Autoharness
                ),
            ));
        }

        if self.common_autoharness_args.list
            && self.common_autoharness_args.format == Format::Pretty
            && self.verify_opts.common_args.quiet
        {
            return Err(Error::raw(
                ErrorKind::ArgumentConflict,
                "The `--quiet` flag is not compatible with the `pretty` format, since `pretty` prints to the terminal. Either specify a different format or don't pass `--quiet`.",
            ));
        }

        if self
            .verify_opts
            .common_args
            .unstable_features
            .contains(UnstableFeature::ConcretePlayback)
        {
            return Err(Error::raw(
                ErrorKind::ArgumentConflict,
                "The autoharness subcommand does not support concrete playback",
            ));
        }

        Ok(())
    }
}

impl ValidateArgs for StandaloneAutoharnessArgs {
    fn validate(&self) -> Result<(), Error> {
        self.common_autoharness_args.validate()?;
        self.verify_opts.validate()?;
        if !self.verify_opts.common_args.unstable_features.contains(UnstableFeature::Autoharness) {
            return Err(Error::raw(
                ErrorKind::MissingRequiredArgument,
                format!(
                    "The `autoharness` subcommand is unstable and requires -Z {}",
                    UnstableFeature::Autoharness
                ),
            ));
        }

        if self.common_autoharness_args.list
            && self.common_autoharness_args.format == Format::Pretty
            && self.verify_opts.common_args.quiet
        {
            return Err(Error::raw(
                ErrorKind::ArgumentConflict,
                "The `--quiet` flag is not compatible with the `pretty` format, since `pretty` prints to the terminal. Either specify a different format or don't pass `--quiet`.",
            ));
        }

        if self.std {
            validate_std_path(&self.input)?;
        } else if !self.input.is_file() {
            return Err(Error::raw(
                ErrorKind::InvalidValue,
                format!(
                    "Invalid argument: Input invalid. `{}` is not a regular file.",
                    self.input.display()
                ),
            ));
        }

        if self
            .verify_opts
            .common_args
            .unstable_features
            .contains(UnstableFeature::ConcretePlayback)
        {
            return Err(Error::raw(
                ErrorKind::ArgumentConflict,
                "The autoharness subcommand does not support concrete playback",
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The bound options only have an effect under `--bounded-arguments`, so passing one on its
    /// own is a mistake rather than a no-op and clap must say so.
    #[test]
    fn bound_options_require_bounded_arguments() {
        for opt in ["--slice-bound", "--string-bound", "--bounded-arbitrary-bound"] {
            let err = CommonAutoharnessArgs::try_parse_from(["autoharness", opt, "4"])
                .expect_err("expected the bound option to require --bounded-arguments");
            assert_eq!(err.kind(), ErrorKind::MissingRequiredArgument);
        }
    }

    #[test]
    fn bound_options_parse_alongside_bounded_arguments() {
        let args = CommonAutoharnessArgs::try_parse_from([
            "autoharness",
            "--bounded-arguments",
            "--slice-bound",
            "2",
        ])
        .expect("expected the bound option to parse with --bounded-arguments");
        assert_eq!(args.slice_bound, 2);
    }

    #[test]
    fn zero_bounds_are_rejected() {
        for opt in ["--slice-bound", "--string-bound", "--bounded-arbitrary-bound"] {
            let args = CommonAutoharnessArgs::try_parse_from([
                "autoharness",
                "--bounded-arguments",
                opt,
                "0",
            ])
            .expect("a bound of 0 parses; validation rejects it");
            let err = args.validate().expect_err("expected a bound of 0 to be rejected");
            assert_eq!(err.kind(), ErrorKind::InvalidValue);
        }
    }

    /// A default value must not stand in for the option being supplied, or every run without
    /// `--bounded-arguments` would fail to parse.
    #[test]
    fn bound_defaults_do_not_require_bounded_arguments() {
        let args = CommonAutoharnessArgs::try_parse_from(["autoharness"])
            .expect("defaults must parse without --bounded-arguments");
        assert_eq!(args.slice_bound, AUTOHARNESS_SLICE_BOUND);
        assert_eq!(args.string_bound, AUTOHARNESS_STR_BOUND);
        assert_eq!(args.bounded_arbitrary_bound, AUTOHARNESS_BOUNDED_ARBITRARY_BOUND);
    }
}
