// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This file contains a small parser for our build script.
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[clap(name = "build-kani")]
#[clap(about = "Builds Kani either for development or release.", long_about = None)]
pub struct ArgParser {
    #[clap(subcommand)]
    pub subcommand: Commands,
}

#[derive(Eq, PartialEq, Subcommand)]
pub enum Commands {
    /// Build kani binaries and sysroot for development.
    BuildDev,
    /// Build Kani's release bundle.
    Bundle,
}
