// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This file contains a small parser for our build script.
use clap::{Args, Parser, Subcommand};

#[derive(Parser)]
#[clap(name = "build-kani")]
#[clap(about = "Builds Kani either for development or release.", long_about = None)]
pub struct ArgParser {
    #[clap(subcommand)]
    pub subcommand: Commands,
}

#[derive(Args, Debug, Eq, PartialEq)]
pub struct BuildDevParser {
    /// Arguments to be passed down to cargo when building cargo binaries.
    #[clap(value_name = "ARG", allow_hyphen_values = true)]
    pub args: Vec<String>,
}

#[derive(Args, Debug, Eq, PartialEq)]
pub struct BundleParser {
    /// String version
    #[clap(value_name = "VERSION", default_value(env!("CARGO_PKG_VERSION")))]
    pub version: String,
}

#[derive(Eq, PartialEq, Subcommand)]
pub enum Commands {
    /// Build kani binaries and sysroot for development.
    BuildDev(BuildDevParser),
    /// Build Kani's release bundle.
    Bundle(BundleParser),
}
