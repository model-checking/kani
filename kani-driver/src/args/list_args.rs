// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Implements the subcommand handling of the list subcommand

use std::path::PathBuf;

use crate::args::{CommonArgs, ValidateArgs, print_stabilized_feature_warning, validate_std_path};
use clap::{Error, Parser, ValueEnum, error::ErrorKind};
use kani_metadata::UnstableFeature;

/// List information relevant to verification
#[derive(Debug, Parser)]
pub struct CargoListArgs {
    #[command(flatten)]
    pub common_args: CommonArgs,

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
    pub common_args: CommonArgs,

    /// Output format
    #[clap(long, default_value = "pretty")]
    pub format: Format,

    /// Pass this flag to run the `list` command on the standard library.
    /// Ensure that the provided `path` is the `library` folder.
    #[arg(long)]
    pub std: bool,
}

/// Output formats available for the subcommand.
#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum, strum_macros::Display)]
#[strum(serialize_all = "kebab-case")]
pub enum Format {
    /// Print output in human-readable format.
    Pretty,
    /// Write output to a Markdown file.
    Markdown,
    /// Write output to a JSON file.
    Json,
}

impl ValidateArgs for CargoListArgs {
    fn validate(&self) -> Result<(), Error> {
        self.common_args.validate()?;
        if self.common_args.unstable_features.contains(UnstableFeature::List) {
            print_stabilized_feature_warning(&self.common_args, UnstableFeature::List);
        }

        if self.format == Format::Pretty && self.common_args.quiet {
            return Err(Error::raw(
                ErrorKind::ArgumentConflict,
                "The `--quiet` flag is not compatible with the `pretty` format, since `pretty` prints to the terminal. Either specify a different format or don't pass `--quiet`.",
            ));
        }

        Ok(())
    }
}

impl ValidateArgs for StandaloneListArgs {
    fn validate(&self) -> Result<(), Error> {
        self.common_args.validate()?;
        if self.common_args.unstable_features.contains(UnstableFeature::List) {
            print_stabilized_feature_warning(&self.common_args, UnstableFeature::List);
        }

        if self.format == Format::Pretty && self.common_args.quiet {
            return Err(Error::raw(
                ErrorKind::ArgumentConflict,
                "The `--quiet` flag is not compatible with the `pretty` format, since `pretty` prints to the terminal. Either specify a different format or don't pass `--quiet`.",
            ));
        }

        if self.std {
            validate_std_path(&self.input)
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
