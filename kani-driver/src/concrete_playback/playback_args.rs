// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Implements the subcommand handling of the playback subcommand

use crate::args::CargoArgs;
use clap::Parser;

#[derive(Default, Debug, Parser)]
/// Playback arguments to be used with cargo kani
pub struct CargoPlayback {
    #[command(flatten)]
    pub playback: PlaybackArgs,

    /// Arguments to pass down to Cargo
    #[command(flatten)]
    pub cargo: CargoArgs,
}

/// Playback arguments to be used with standalone kani
#[derive(Default, Debug, Parser)]
pub struct KaniPlayback {
    #[command(flatten)]
    pub playback: PlaybackArgs,

    /// Arguments to pass down to Cargo
    #[command(flatten)]
    pub cargo: CargoArgs,
}

/// Playback arguments
#[derive(Default, Debug, clap::Args)]
pub struct PlaybackArgs {
    /// Write Assess metadata (unstable file format) to the given file
    #[arg(long)]
    pub test: String,
}
