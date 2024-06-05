// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Implements the `verify-std` subcommand handling.

use crate::args::VerificationArgs;
use clap::Parser;
use std::path::PathBuf;

/// Verify a local version of the Rust standard library.
///
/// This is an **unstable option** and it the standard library version must be compatible with
/// Kani's toolchain version.
#[derive(Debug, Parser)]
pub struct VerifyStdArgs {
    /// The path to the standard library folder.
    pub std_path: PathBuf,

    #[command(flatten)]
    pub verify_opts: VerificationArgs,
}
