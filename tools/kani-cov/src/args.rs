// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This module defines the data structures and validation logic for subcommands
//! and general arguments. Most of the implementation is done through clap.
//!
//! Note: Validation for subcommand-specific arguments is done in the module
//! associated with each subcommand.

use std::path::PathBuf;

use anyhow::{Result, bail};
use clap::{arg, command};

use crate::{merge, report, summary};

/// We define three subcommands:
///  * `merge` for merging raw Kani coverage results (AKA "kaniraw" files)
///  * `summary` for producing a summary containing coverage metrics
///  * `report` for generating human-readable coverage reports
/// 
/// As an example, let's assume we execute Kani with coverage enabled
/// ```sh
/// kani main.rs --coverage -Zsource-coverage
/// ```
/// the raw coverage results will be saved to a folder
/// ```sh
/// [info] Coverage results saved to /absolute/path/to/results/kanicov_2024-09-23_23-49
/// ```
/// 
/// We can aggregate those results with the `merge` subcommand
/// ```sh
/// kani-cov merge kanicov_2024-09-23_23-49/*kaniraw.json
/// ```
/// which by default produces a `default_kanicov.json` file.
/// 
/// Once we have both the "kanicov" file and the "kanimap" file, we are ready to
/// produce coverage metrics with the `summary` subcommand:
/// ```sh
/// kani-cov summary kanicov_2024-09-23_23-49/kanicov_2024-09-23_23-49_kanimap.json --profile default_kanicov.json
/// ```
/// 
/// We can also produce coverage reports with the `report` subcommand:
/// ```sh
/// kani-cov report kanicov_2024-09-23_23-49/kanicov_2024-09-23_23-49_kanimap.json --profile default_kanicov.json
/// ```
#[derive(Debug, clap::Subcommand)]
pub enum Subcommand {
    Merge(MergeArgs),
    Summary(SummaryArgs),
    Report(ReportArgs),
}

/// The main command.
/// Note: We use the same options as in Kani so that their option-parsing
/// behaviors (and issues due to them) are as similar as possible.
#[derive(Debug, clap::Parser)]
#[command(
    version,
    name = "kani-cov",
    about = "A tool to process coverage information from Kani",
    args_override_self = true,
    subcommand_negates_reqs = true,
    subcommand_precedence_over_arg = true,
    args_conflicts_with_subcommands = true
)]

/// General arguments
pub struct Args {
    #[command(subcommand)]
    pub command: Option<Subcommand>,
}

/// Arguments for the `merge` subcommand
#[derive(Debug, clap::Args)]
pub struct MergeArgs {
    #[arg(long)]
    pub output: Option<PathBuf>,
    #[arg(required = true)]
    pub files: Vec<PathBuf>,
}

/// Arguments for the `summary` subcommand
#[derive(Debug, clap::Args)]
pub struct SummaryArgs {
    // The path to the "kanimap" file
    #[arg(required = true)]
    pub mapfile: PathBuf,
    // The path to the "kanicov" file
    #[arg(long, required = true)]
    pub profile: PathBuf,
    // The format of the summary
    #[arg(long, short, value_parser = clap::value_parser!(SummaryFormat), default_value = "markdown")]
    pub format: SummaryFormat,
}

/// The format of the summary.
/// 
/// The default format is Markdown, but the CSV and JSON formats would be really
/// nice to have.
#[derive(Clone, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum SummaryFormat {
    Markdown,
    // Csv,
    // Json,
}

/// Arguments for the `report` subcommand
#[derive(Debug, clap::Args)]
pub struct ReportArgs {
    // The path to the "kanimap" file
    #[arg(required = true)]
    pub mapfile: PathBuf,
    // The path to the "kanicov" file
    #[arg(long, required = true)]
    pub profile: PathBuf,
    // The format of the report
    #[arg(long, short, value_parser = clap::value_parser!(ReportFormat), default_value = "terminal")]
    pub format: ReportFormat,
}

#[derive(Clone, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum ReportFormat {
    Terminal,
    Escapes,
}

/// Validate general arguments and delegate validation of command-specific
/// arguments.
pub fn validate_args(args: &Args) -> Result<()> {
    if args.command.is_none() {
        bail!("subcommand needs to be specified (`merge`, `summary` or `report`)")
    }

    match args.command.as_ref().unwrap() {
        Subcommand::Merge(merge_args) => merge::validate_merge_args(&merge_args)?,
        Subcommand::Summary(summary_args) => summary::validate_summary_args(&summary_args)?,
        Subcommand::Report(report_args) => report::validate_report_args(&report_args)?,
    };

    Ok(())
}
