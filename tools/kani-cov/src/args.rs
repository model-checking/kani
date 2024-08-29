// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::path::PathBuf;

use anyhow::{bail, Result};
use clap::{arg, command};

use crate::{merge, summary, report};

#[derive(Debug, clap::Subcommand)]
pub enum Subcommand {
    Merge(MergeArgs),
    Summary(SummaryArgs),
    // Report(ReportArgs),
}

#[derive(Debug, clap::Parser)]
#[command(
    version,
    name = "kani-cov",
    about = "Process coverage information from Kani",
    args_override_self = true,
    subcommand_negates_reqs = true,
    subcommand_precedence_over_arg = true,
    args_conflicts_with_subcommands = true
)]
pub struct Args {
    #[command(subcommand)]
    pub command: Option<Subcommand>,
}

#[derive(Debug, clap::Args)]
pub struct MergeArgs {
    #[arg(long)]
    pub output: Option<PathBuf>,
    #[arg(required = true)]
    pub files: Vec<PathBuf>,
}

#[derive(Debug, clap::Args)]
pub struct SummaryArgs {
    #[arg(required = true)]
    pub profile: Vec<PathBuf>,
    #[arg(required = true)]
    pub output: Option<PathBuf>,
}

pub fn validate_args(args: &Args) -> Result<()> {
    if args.command.is_none() {
        bail!("subcommand needs to be specified")
    }

    match args.command.as_ref().unwrap() {
        Subcommand::Merge(merge_args) => merge::validate_merge_args(&merge_args)?,
        Subcommand::Summary(summary_args) => summary::validate_summary_args(&summary_args)?,
        // Subcommand::Report => report::validate_report_args(args)?,
    };

    Ok(())
}
