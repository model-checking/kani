// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

mod coverage;
mod args;
mod merge;
mod summary;
mod report;

use args::{validate_args, Subcommand};
use clap::Parser;
use anyhow::Result;

fn main() -> Result<()> {
    let args = args::Args::parse();

    validate_args(&args)?;

    match args.command.unwrap() {
        Subcommand::Merge(merge_args) => merge::merge_main(&merge_args)?,
        Subcommand::Summary(summary_args) => summary::summary_main(&summary_args)?,
        // Subcommand::Report => report::report_main()?,
    };

    Ok(())
}
