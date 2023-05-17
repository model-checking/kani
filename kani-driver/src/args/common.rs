// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Define arguments that should be common to all subcommands in Kani.
use crate::args::ValidateArgs;
use clap::{error::Error, error::ErrorKind, ValueEnum};

/// Common Kani arguments that we expect to be included in most subcommands.
#[derive(Debug, clap::Args)]
pub struct CommonArgs {
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

    /// Enable an unstable feature.
    #[arg(short = 'Z', num_args(1), value_name = "UNSTABLE_FEATURE")]
    pub unstable_features: Vec<UnstableFeatures>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum, strum_macros::Display)]
#[strum(serialize_all = "kebab-case")]
pub enum UnstableFeatures {
    /// Allow replacing certain items with stubs (mocks).
    /// See [RFC-0002](https://model-checking.github.io/kani/rfc/rfcs/0002-function-stubbing.html)
    Stubbing,
    /// Generate a C-like file equivalent to input program used for debugging purpose.
    GenC,
    /// Allow Kani to link against C code.
    CFfi,
}

impl ValidateArgs for CommonArgs {
    fn validate(&self) -> Result<(), Error> {
        if self.dry_run {
            return Err(Error::raw(
                ErrorKind::ValueValidation,
                "The `--dry-run` option is obsolete. Use --verbose instead.",
            ));
        }
        Ok(())
    }
}
