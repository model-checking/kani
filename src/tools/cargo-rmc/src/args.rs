// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "rmc",
    about = "Verify a single Rust file. For more information, see https://github.com/model-checking/rmc"
)]
pub struct StandaloneArgs {
    /// Rust file to verify
    #[structopt(parse(from_os_str))]
    pub input: PathBuf,

    #[structopt(flatten)]
    pub common_opts: RmcArgs,
}

#[derive(Debug, StructOpt)]
#[structopt(
    name = "cargo-rmc",
    about = "Verify a Rust crate. For more information, see https://github.com/model-checking/rmc"
)]
pub struct CargoRmcArgs {
    #[structopt(flatten)]
    pub common_opts: RmcArgs,
}

// Common arguments for invoking RMC. This gets put into RmcContext, whereas
// anything above is "local" to "main"'s control flow.
#[derive(Debug, StructOpt)]
pub struct RmcArgs {
    /// Generate visualizer report to <target-dir>/report/html/index.html
    #[structopt(long)]
    pub visualize: bool,
    /// Keep temporary files generated throughout RMC process
    #[structopt(long)]
    pub keep_temps: bool,
}
