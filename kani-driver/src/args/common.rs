// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Define arguments that should be common to all subcommands in Kani.
use crate::args::ValidateArgs;
use clap::{error::Error, error::ErrorKind};
pub use kani_metadata::{EnabledUnstableFeatures, UnstableFeature};

/// Common Kani arguments that we expect to be included in most subcommands.
#[derive(Debug, clap::Args)]
pub struct CommonArgs {
    /// Produce full debug information
    #[arg(long)]
    pub debug: bool,
    /// Produces no output, just an exit code and requested artifacts; overrides --verbose
    #[arg(long, short, conflicts_with_all(["debug", "verbose"]))]
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
    #[clap(flatten)]
    pub unstable_features: EnabledUnstableFeatures,
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

/// The verbosity level to be used in Kani.
pub trait Verbosity {
    /// Whether we should be quiet.
    fn quiet(&self) -> bool;
    /// Whether we should be verbose.
    /// Note that `debug() == true` must imply `verbose() == true`.
    fn verbose(&self) -> bool;
    /// Whether any verbosity was selected.
    fn is_set(&self) -> bool;
}

impl Verbosity for CommonArgs {
    fn quiet(&self) -> bool {
        self.quiet
    }

    fn verbose(&self) -> bool {
        self.verbose || self.debug
    }

    fn is_set(&self) -> bool {
        self.quiet || self.verbose || self.debug
    }
}
