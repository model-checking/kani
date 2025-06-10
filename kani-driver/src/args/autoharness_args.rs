// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Implements the subcommand handling of the autoharness subcommand

use std::path::PathBuf;

use crate::args::list_args::Format;
use crate::args::{ValidateArgs, VerificationArgs, validate_std_path};
use crate::util::warning;
use clap::{Error, Parser, error::ErrorKind};
use kani_metadata::UnstableFeature;
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

    /// Run the `list` subcommand after generating the automatic harnesses. Note that this option implies --only-codegen.
    #[arg(long)]
    pub list: bool,

    /// The format of the `list` output. Requires --list.
    #[arg(long, default_value = "pretty", requires = "list")]
    pub format: Format,
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
