// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
use crate::args::CommonArgs;
use clap::Parser;

#[derive(Debug, Parser)]
pub struct CargoCoverageArgs {
    #[command(flatten)]
    pub coverage: CoverageArgs,
}

#[derive(Debug, clap::Args)]
pub struct CoverageArgs {
    /// Common args always available to Kani subcommands.
    #[command(flatten)]
    pub common_opts: CommonArgs,
}
