// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Implements the subcommand handling of the list subcommand

use std::path::PathBuf;

use crate::args::ValidateArgs;
use clap::{error::ErrorKind, Error, Parser, ValueEnum};
use kani_metadata::UnstableFeature;

use super::VerificationArgs;

/// List information relevant to verification
#[derive(Debug, Parser)]
pub struct CargoListArgs {
    #[command(flatten)]
    pub verify_opts: VerificationArgs,

    /// Output format
    #[clap(long, default_value = "pretty")]
    pub format: Format,
}

/// List information relevant to verification
#[derive(Debug, Parser)]
pub struct StandaloneListArgs {
    /// Rust file to verify
    #[arg(required = true)]
    pub input: PathBuf,

    #[arg(long, hide = true)]
    pub crate_name: Option<String>,

    #[command(flatten)]
    pub verify_opts: VerificationArgs,

    /// Output format
    #[clap(long, default_value = "pretty")]
    pub format: Format,

    /// Pass this flag to run the `list` command on the standard library.
    /// Ensure that the provided `path` is the `library` folder.
    #[arg(long)]
    pub std: bool,
}

/// Message formats available for the subcommand.
#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum, strum_macros::Display)]
#[strum(serialize_all = "kebab-case")]
pub enum Format {
    /// Print diagnostic messages in a user friendly format.
    Pretty,
    /// Print diagnostic messages in JSON format.
    Json,
}

impl ValidateArgs for CargoListArgs {
    fn validate(&self) -> Result<(), Error> {
        self.verify_opts.validate()?;
        if !self.verify_opts.common_args.unstable_features.contains(UnstableFeature::List) {
            return Err(Error::raw(
                ErrorKind::MissingRequiredArgument,
                "The `list` subcommand is unstable and requires -Z list",
            ));
        }

        Ok(())
    }
}

impl ValidateArgs for StandaloneListArgs {
    fn validate(&self) -> Result<(), Error> {
        self.verify_opts.validate()?;
        if !self.verify_opts.common_args.unstable_features.contains(UnstableFeature::List) {
            return Err(Error::raw(
                ErrorKind::MissingRequiredArgument,
                "The `list` subcommand is unstable and requires -Z list",
            ));
        }

        if self.std {
            if !self.input.exists() {
                Err(Error::raw(
                    ErrorKind::InvalidValue,
                    format!(
                        "Invalid argument: `<input>` argument `{}` does not exist",
                        self.input.display()
                    ),
                ))
            } else if !self.input.is_dir() {
                Err(Error::raw(
                    ErrorKind::InvalidValue,
                    format!(
                        "Invalid argument: `<input>` argument `{}` is not a directory",
                        self.input.display()
                    ),
                ))
            } else {
                let full_path = self.input.canonicalize()?;
                let dir = full_path.file_stem().unwrap();
                if dir != "library" {
                    Err(Error::raw(
                        ErrorKind::InvalidValue,
                        format!(
                            "Invalid argument: Expected `<input>` to point to the `library` folder \
                        containing the standard library crates.\n\
                        Found `{}` folder instead",
                            dir.to_string_lossy()
                        ),
                    ))
                } else {
                    Ok(())
                }
            }
        } else if self.input.is_file() {
            Ok(())
        } else {
            Err(Error::raw(
                ErrorKind::InvalidValue,
                format!(
                    "Invalid argument: Input invalid. `{}` is not a regular file.",
                    self.input.display()
                ),
            ))
        }
    }
}
