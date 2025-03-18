// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Implements the `verify-std` subcommand handling.

use crate::args::{ValidateArgs, VerificationArgs, validate_std_path};
use clap::error::ErrorKind;
use clap::{Error, Parser};
use kani_metadata::UnstableFeature;
use std::path::PathBuf;

/// Verify a local version of the Rust standard library.
///
/// This is an **unstable option** and it the standard library version must be compatible with
/// Kani's toolchain version.
#[derive(Debug, Parser)]
pub struct VerifyStdArgs {
    /// The path to the folder containing the crates for the Rust standard library.
    /// Note that this directory must be named `library` as used in the Rust toolchain and
    /// repository.
    pub std_path: PathBuf,

    #[command(flatten)]
    pub verify_opts: VerificationArgs,
}

impl ValidateArgs for VerifyStdArgs {
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
                "The `verify-std` subcommand is unstable and requires -Z unstable-options",
            ));
        }

        validate_std_path(&self.std_path)
    }
}
