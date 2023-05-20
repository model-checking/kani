// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This module contains arguments specific to the `cargo kani assess` subcommand.

use std::path::PathBuf;

use clap::Parser;

/// `cargo kani assess` subcommand arguments
#[derive(Default, Debug, Parser)]
pub struct AssessArgs {
    #[command(subcommand)]
    pub command: Option<AssessSubcommand>,

    /// Write Assess metadata (unstable file format) to the given file
    #[arg(long, hide = true)]
    pub emit_metadata: Option<PathBuf>,
}

/// `cargo kani assess` takes optional subcommands to request specialized behavior
#[derive(Debug, Parser)]
pub enum AssessSubcommand {
    /// Run assess on a directory containing multiple cargo projects, and aggregate the results
    Scan(ScanArgs),
}

/// `cargo kani assess scan` subcommand arguments
#[derive(Debug, Parser)]
pub struct ScanArgs {
    // TODO: When assess scan is invoked using `--existing-only`, it should check if the Kani version
    // from the existing metadata files matches the current version. Otherwise, the results may no
    // longer hold.
    /// Don't run assess on found packages, just re-analyze the results from a previous run
    #[arg(long, hide = true)]
    pub existing_only: bool,

    /// Only consider the packages named in the given file
    #[arg(long, hide = true)]
    pub filter_packages_file: Option<PathBuf>,

    /// Write Assess-Scan metadata (unstable file format) to the given file
    #[arg(long, hide = true)]
    pub emit_metadata: Option<PathBuf>,
}
