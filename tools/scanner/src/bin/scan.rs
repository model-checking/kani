// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Modifications Copyright Kani Contributors
// See GitHub history for details.

// This is a modified version of project-stable-mir `test-drive`
// <https://github.com/rust-lang/project-stable-mir/blob/8ec26c61/tools/test-drive/src/main.rs>

//! Provide a binary that can be used as a replacement to rustc.
//!
//! Besides executing the regular compilation, this binary will run a few static analyses.
//!
//! The result for each analysis will be stored in a file with the same prefix as an object file,
//! together with the name of the analysis.
//!
//! Look at each analysis documentation to see which files an analysis produces.

use scanner::run_all;
use std::process::ExitCode;

// ---- Arguments that should be parsed by the test-driver (w/ "scan" prefix)
/// Enable verbose mode.
const VERBOSE_ARG: &str = "--scan-verbose";

/// This is a wrapper that can be used to replace rustc.
fn main() -> ExitCode {
    let args = std::env::args();
    let (scan_args, rustc_args): (Vec<String>, _) = args.partition(|arg| arg.starts_with("--scan"));
    let verbose = scan_args.contains(&VERBOSE_ARG.to_string());
    run_all(rustc_args, verbose)
}
