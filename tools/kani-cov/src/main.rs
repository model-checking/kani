// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Main module of `kani-cov`, containing its main function.

mod args;
mod coverage;
mod merge;
mod report;
mod summary;

use anyhow::Result;
use args::{Subcommand, validate_args};
use clap::Parser;

/// The main function of `kani-cov`.
/// First, we parse and validate the subcommand and arguments. Then, we call the
/// main function for the subcommand that had been specified.
fn main() -> Result<()> {
    let args = args::Args::parse();

    validate_args(&args)?;

    match args.command.unwrap() {
        Subcommand::Merge(merge_args) => merge::merge_main(&merge_args)?,
        Subcommand::Summary(summary_args) => summary::summary_main(&summary_args)?,
        Subcommand::Report(report_args) => report::report_main(&report_args)?,
    };

    Ok(())
}
