// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This file contains a small parser for our build script.
use clap::{Args, Parser, Subcommand};
use kani_metadata::{SUPPORTED_TARGETS, supported_targets_list};

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
    /// Do not re-build Kani libraries. Only use this if you know there has been no changes to Kani
    /// libraries or the underlying Rust compiler.
    #[clap(long)]
    pub skip_libs: bool,
    /// Also build Kani's verification libraries for this target triple, so that
    /// `kani --target <TRIPLE> -Z unstable-options` can verify for it. May be repeated.
    /// Targets built earlier are rebuilt whether or not they are named again.
    #[clap(
        long = "lib-target",
        value_name = "TRIPLE",
        conflicts_with = "skip_libs",
        value_parser = parse_lib_target
    )]
    pub lib_targets: Vec<String>,
}

/// Refuse a target Kani has no machine model for before building anything. Otherwise the error is
/// kani-compiler's, from inside the library build, after the binaries have been rebuilt.
fn parse_lib_target(target: &str) -> Result<String, String> {
    if SUPPORTED_TARGETS.contains(&target) {
        Ok(target.to_string())
    } else {
        Err(format!("Kani can verify for {}", supported_targets_list()))
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Result<ArgParser, clap::Error> {
        ArgParser::try_parse_from(["build-kani", "build-dev"].iter().chain(args))
    }

    #[test]
    fn lib_target_must_be_supported() {
        let Commands::BuildDev(parsed) =
            parse(&["--lib-target", "riscv64gc-unknown-linux-gnu"]).unwrap().subcommand
        else {
            panic!("expected build-dev");
        };
        assert_eq!(parsed.lib_targets, ["riscv64gc-unknown-linux-gnu"]);

        for target in ["x86_64-unknown-linux-musl", "thumbv7em-none-eabihf", ""] {
            let err = parse(&["--lib-target", target]).err().expect(target);
            assert_eq!(err.kind(), clap::error::ErrorKind::ValueValidation, "for `{target}`");
        }
    }
}
