// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Implements the subcommand handling of the autoharness subcommand

use std::path::PathBuf;

use crate::args::list_args::Format;
use crate::args::{ValidateArgs, VerificationArgs, validate_std_path};
use clap::{Error, Parser, error::ErrorKind};
use kani_metadata::UnstableFeature;
use regex::Regex;

// TODO: It would be nice if we could borrow --exact here from VerificationArgs to differentiate between partial/exact matches,
// like --harnesses does. Sharing arguments with VerificationArgs doesn't work with our current structure, though.
#[derive(Debug, Parser)]
pub struct CommonAutoharnessArgs {
    /// Only create automatic harnesses for functions that match the given pattern.
    /// This option can be provided multiple times, which will verify functions matching any of the patterns.
    /// Kani considers a function to match the pattern if its fully qualified path contains PATTERN as a substring.
    /// Example: `--include-pattern foo` matches all functions whose fully qualified paths contain the substring "foo".
    #[arg(long = "include-pattern", num_args(1), value_name = "PATTERN")]
    pub include_pattern: Vec<String>,

    /// Only create automatic harnesses for functions that do not match the given pattern.
    /// This option can be provided multiple times, which will verify functions that do not match any of the patterns.
    /// Kani considers a function to match the pattern if its fully qualified path contains PATTERN as a substring.

    /// This option takes precedence over `--include-pattern`, i.e., Kani will first select all functions that match `--include-pattern`,
    /// then exclude those that match `--exclude-pattern.`
    /// Example: `--include-pattern foo --exclude-pattern foo::bar` creates automatic harnesses for all functions whose paths contain "foo" without "foo::bar".
    /// Example: `--include-pattern foo::bar --exclude-pattern foo` makes the `--include-pattern` a no-op, since the exclude pattern is a superset of the include pattern.
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
