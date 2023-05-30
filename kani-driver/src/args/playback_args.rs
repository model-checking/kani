// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Implements the subcommand handling of the playback subcommand

use crate::args::cargo::CargoTestArgs;
use crate::args::common::UnstableFeatures;
use crate::args::{CommonArgs, ValidateArgs};
use clap::error::ErrorKind;
use clap::{Error, Parser, ValueEnum};
use std::path::PathBuf;

/// Execute concrete playback testcases of a local package.
#[derive(Debug, Parser)]
pub struct CargoPlaybackArgs {
    #[command(flatten)]
    pub playback: PlaybackArgs,

    /// Arguments to pass down to Cargo that are specific to tests.
    #[command(flatten)]
    pub cargo: CargoTestArgs,
}

/// Execute concrete playback testcases of a local crate.
#[derive(Debug, Parser)]
pub struct KaniPlaybackArgs {
    /// Rust crate's top file location.
    pub input: PathBuf,

    #[command(flatten)]
    pub playback: PlaybackArgs,
}

/// Playback subcommand arguments.
#[derive(Debug, clap::Args)]
pub struct PlaybackArgs {
    /// Common args always available to Kani subcommands.
    #[command(flatten)]
    pub common_opts: CommonArgs,

    /// Compile but don't actually run the tests.
    #[arg(long)]
    pub only_codegen: bool,

    // TODO: We should make this a common option to all subcommands.
    /// Control the subcommand output.
    #[arg(long, default_value = "human")]
    pub message_format: MessageFormat,

    /// Arguments to be passed to the test binary.
    #[arg(num_args(0..), last = true)]
    pub test_args: Vec<String>,
}

/// Message formats available for the subcommand.
#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum, strum_macros::Display)]
#[strum(serialize_all = "kebab-case")]
pub enum MessageFormat {
    /// Print diagnostic messages in a user friendly format.
    Human,
    /// Print diagnostic messages in JSON format.
    Json,
}

impl ValidateArgs for CargoPlaybackArgs {
    fn validate(&self) -> Result<(), Error> {
        self.playback.validate()?;
        self.cargo.validate()
    }
}

impl ValidateArgs for KaniPlaybackArgs {
    fn validate(&self) -> Result<(), Error> {
        self.playback.validate()?;
        if !self.input.is_file() {
            return Err(Error::raw(
                ErrorKind::InvalidValue,
                &format!(
                    "Invalid argument: Input invalid. `{}` is not a regular file.",
                    self.input.display()
                ),
            ));
        }
        Ok(())
    }
}

impl ValidateArgs for PlaybackArgs {
    fn validate(&self) -> Result<(), Error> {
        self.common_opts.validate()?;
        if !self.common_opts.unstable_features.contains(&UnstableFeatures::ConcretePlayback) {
            return Err(Error::raw(
                ErrorKind::MissingRequiredArgument,
                "The `playback` subcommand is unstable and requires `-Z concrete-playback` \
                to be used.",
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::*;

    #[test]
    fn check_cargo_parse_test_works() {
        let input = "playback -Z concrete-playback -- TEST_NAME".split_whitespace();
        let args = CargoPlaybackArgs::try_parse_from(input.clone()).unwrap();
        args.validate().unwrap();
        assert_eq!(args.playback.test_args, ["TEST_NAME"]);
        // The default value is human friendly.
        assert_eq!(args.playback.message_format, MessageFormat::Human);
    }

    #[test]
    fn check_cargo_parse_pkg_works() {
        let input = "playback -Z concrete-playback -p PKG_NAME".split_whitespace();
        let args = CargoPlaybackArgs::try_parse_from(input).unwrap();
        args.validate().unwrap();
        assert_eq!(&args.cargo.common.package, &["PKG_NAME"])
    }

    #[test]
    fn check_parse_format_works() {
        let input = "playback -Z concrete-playback --message-format=json".split_whitespace();
        let args = CargoPlaybackArgs::try_parse_from(input).unwrap();
        args.validate().unwrap();
        assert_eq!(args.playback.message_format, MessageFormat::Json)
    }

    #[test]
    fn check_kani_parse_test_works() {
        let input = "playback -Z concrete-playback input.rs -- TEST_NAME".split_whitespace();
        let args = KaniPlaybackArgs::try_parse_from(input).unwrap();
        // Don't validate this since we check if the input file exists.
        //args.validate().unwrap();
        assert_eq!(args.playback.test_args, ["TEST_NAME"]);
        assert_eq!(args.input, PathBuf::from("input.rs"));
        // The default value is human friendly.
        assert_eq!(args.playback.message_format, MessageFormat::Human);
    }

    #[test]
    fn check_kani_parse_extra_args() {
        let input = "playback -Z concrete-playback input.rs -- TEST_NAME --exact --nocapture"
            .split_whitespace();
        let args = KaniPlaybackArgs::try_parse_from(input).unwrap();
        assert_eq!(args.playback.test_args, ["TEST_NAME", "--exact", "--nocapture"])
    }

    #[test]
    fn check_kani_no_unstable_fails() {
        let input = "playback input.rs".split_whitespace();
        let args = KaniPlaybackArgs::try_parse_from(input).unwrap();
        let err = args.validate().unwrap_err();
        assert_eq!(err.kind(), ErrorKind::MissingRequiredArgument);
    }
}
