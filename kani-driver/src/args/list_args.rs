// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Implements the subcommand handling of the list subcommand

use std::path::PathBuf;

use crate::args::{CommonArgs, ValidateArgs, validate_std_path};
use clap::{Error, Parser, ValueEnum, error::ErrorKind};

/// List information relevant to verification
#[derive(Debug, Parser)]
pub struct CargoListArgs {
    #[command(flatten)]
    pub common_args: CommonArgs,

    /// Output format
    #[clap(long, default_value = "pretty")]
    pub format: Format,

    /// Write `kani-list.md` or `kani-list.json` to this directory instead of the current
    /// directory. Kani creates the directory if it does not exist.
    #[arg(long, value_name = "DIR")]
    pub output: Option<PathBuf>,
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

    /// Write `kani-list.md` or `kani-list.json` to this directory instead of the current
    /// directory. Kani creates the directory if it does not exist.
    #[arg(long, value_name = "DIR")]
    pub output: Option<PathBuf>,

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

        if self.format == Format::Pretty && self.common_args.quiet {
            return Err(Error::raw(
                ErrorKind::ArgumentConflict,
                "The `--quiet` flag is not compatible with the `pretty` format, since `pretty` prints to the terminal. Either specify a different format or don't pass `--quiet`.",
            ));
        }

        if self.format == Format::Pretty && self.output.is_some() {
            return Err(Error::raw(
                ErrorKind::ArgumentConflict,
                "The `--output` flag is not compatible with the `pretty` format, since `pretty` prints to the terminal. Either specify `--format markdown` or `--format json`, or don't pass `--output`.",
            ));
        }

        if let Some(output) = &self.output
            && output.exists()
            && !output.is_dir()
        {
            return Err(Error::raw(
                ErrorKind::InvalidValue,
                format!(
                    "Invalid argument: `--output` argument `{}` is not a directory",
                    output.display()
                ),
            ));
        }

        Ok(())
    }
}

impl ValidateArgs for StandaloneListArgs {
    fn validate(&self) -> Result<(), Error> {
        self.common_args.validate()?;

        if self.format == Format::Pretty && self.common_args.quiet {
            return Err(Error::raw(
                ErrorKind::ArgumentConflict,
                "The `--quiet` flag is not compatible with the `pretty` format, since `pretty` prints to the terminal. Either specify a different format or don't pass `--quiet`.",
            ));
        }

        if self.format == Format::Pretty && self.output.is_some() {
            return Err(Error::raw(
                ErrorKind::ArgumentConflict,
                "The `--output` flag is not compatible with the `pretty` format, since `pretty` prints to the terminal. Either specify `--format markdown` or `--format json`, or don't pass `--output`.",
            ));
        }

        if let Some(output) = &self.output
            && output.exists()
            && !output.is_dir()
        {
            return Err(Error::raw(
                ErrorKind::InvalidValue,
                format!(
                    "Invalid argument: `--output` argument `{}` is not a directory",
                    output.display()
                ),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn validate_cargo_list(args: &[&str]) -> Result<(), Error> {
        CargoListArgs::try_parse_from(args).unwrap().validate()
    }

    #[test]
    fn output_accepts_existing_and_missing_directories() {
        for format in ["json", "markdown"] {
            for dir in [".", "missing-dir"] {
                let args = ["list", "--format", format, "--output", dir];
                assert!(validate_cargo_list(&args).is_ok(), "{args:?}");
            }
        }
    }

    #[test]
    fn output_conflicts_with_pretty_format() {
        let err = validate_cargo_list(&["list", "--output", "."]).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::ArgumentConflict);
    }

    #[test]
    fn output_rejects_existing_file() {
        let file = tempfile::NamedTempFile::new().unwrap();
        let path = file.path().to_str().unwrap();
        let err = validate_cargo_list(&["list", "--format", "json", "--output", path]).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::InvalidValue);
    }
}
