// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Implements the subcommand handling of the list subcommand

use std::path::PathBuf;

use crate::args::{ValidateArgs, VerificationArgs};
use clap::{Error, Parser, error::ErrorKind};
use kani_metadata::UnstableFeature;

// TODO add --function option to only verify functions matching the substring
// (akin to --harness).

/// Automatically verify functions in a crate.
#[derive(Debug, Parser)]
pub struct CargoAutoverifyArgs {
    #[command(flatten)]
    pub verify_opts: VerificationArgs,
}

/// Automatically verify functions in a file.
#[derive(Debug, Parser)]
pub struct StandaloneAutoverifyArgs {
    /// Rust crate's top file location.
    #[arg(required = true)]
    pub input: PathBuf,

    #[arg(long, hide = true)]
    pub crate_name: Option<String>,

    #[command(flatten)]
    pub verify_opts: VerificationArgs,
}

impl ValidateArgs for CargoAutoverifyArgs {
    fn validate(&self) -> Result<(), Error> {
        self.verify_opts.validate()?;
        if !self
            .verify_opts
            .common_args
            .unstable_features
            .contains(UnstableFeature::UnstableOptions)
        {
            return Err(Error::raw(
                ErrorKind::MissingRequiredArgument,
                format!(
                    "The `autoverify` subcommand is unstable and requires -Z {}",
                    UnstableFeature::UnstableOptions.to_string()
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
                "The autoverify subcommand does not support concrete playback",
            ));
        }

        Ok(())
    }
}

impl ValidateArgs for StandaloneAutoverifyArgs {
    fn validate(&self) -> Result<(), Error> {
        self.verify_opts.validate()?;
        if !self
            .verify_opts
            .common_args
            .unstable_features
            .contains(UnstableFeature::UnstableOptions)
        {
            return Err(Error::raw(
                ErrorKind::MissingRequiredArgument,
                format!(
                    "The `autoverify` subcommand is unstable and requires -Z {}",
                    UnstableFeature::UnstableOptions.to_string()
                ),
            ));
        }
        if !self.input.is_file() {
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
                "The autoverify subcommand does not support concrete playback",
            ));
        }

        Ok(())
    }
}
