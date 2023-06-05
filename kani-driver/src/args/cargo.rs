// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Module that define parsers that mimic Cargo options.

use crate::args::ValidateArgs;
use clap::error::Error;
use std::ffi::OsString;
use std::path::PathBuf;

/// Arguments that Kani pass down into Cargo essentially uninterpreted.
/// These generally have to do with selection of packages or activation of features.
/// These do not (currently) include cargo args that kani pays special attention to:
/// for instance, we keep `--tests` and `--target-dir` elsewhere.
#[derive(Debug, Default, clap::Args)]
pub struct CargoCommonArgs {
    /// Activate all package features
    #[arg(long)]
    pub all_features: bool,
    /// Do not activate the `default` feature
    #[arg(long)]
    pub no_default_features: bool,

    // This tolerates spaces too, but we say "comma" only because this is the least error-prone approach...
    /// Comma separated list of package features to activate
    #[arg(short = 'F', long)]
    features: Vec<String>,

    /// Path to Cargo.toml
    #[arg(long, name = "PATH")]
    pub manifest_path: Option<PathBuf>,

    /// Build all packages in the workspace
    #[arg(long)]
    pub workspace: bool,

    /// Run Kani on the specified packages.
    #[arg(long, short, conflicts_with("workspace"), num_args(1..))]
    pub package: Vec<String>,

    /// Exclude the specified packages
    #[arg(long, short, requires("workspace"), conflicts_with("package"), num_args(1..))]
    pub exclude: Vec<String>,
}

impl CargoCommonArgs {
    /// Parse the string we're given into a list of feature names
    ///
    /// clap can't do this for us because it accepts multiple different delimeters
    pub fn features(&self) -> Vec<String> {
        let mut result = Vec::new();

        for s in &self.features {
            for piece in s.split(&[' ', ',']) {
                result.push(piece.to_owned());
            }
        }
        result
    }

    /// Convert the arguments back to a format that cargo can understand.
    /// Note that the `exclude` option requires special processing and it's not included here.
    pub fn to_cargo_args(&self) -> Vec<OsString> {
        let mut cargo_args: Vec<OsString> = vec![];
        if self.all_features {
            cargo_args.push("--all-features".into());
        }

        if self.no_default_features {
            cargo_args.push("--no-default-features".into());
        }

        let features = self.features();
        if !features.is_empty() {
            cargo_args.push(format!("--features={}", features.join(",")).into());
        }

        if let Some(path) = &self.manifest_path {
            cargo_args.push("--manifest-path".into());
            cargo_args.push(path.into());
        }
        if self.workspace {
            cargo_args.push("--workspace".into())
        }

        cargo_args.extend(self.package.iter().map(|pkg| format!("-p={pkg}").into()));
        cargo_args
    }
}

/// Leave it for Cargo to validate these for now.
impl ValidateArgs for CargoCommonArgs {
    fn validate(&self) -> Result<(), Error> {
        Ok(())
    }
}

/// Arguments that cargo Kani supports to select build / test target.
#[derive(Debug, Default, clap::Args)]
pub struct CargoTargetArgs {
    /// Test only the specified binary target.
    #[arg(long)]
    pub bin: Vec<String>,

    /// Test all binaries.
    #[arg(long)]
    pub bins: bool,

    /// Test only the package's library unit tests.
    #[arg(long)]
    pub lib: bool,
}

impl CargoTargetArgs {
    /// Convert the arguments back to a format that cargo can understand.
    pub fn to_cargo_args(&self) -> Vec<OsString> {
        let mut cargo_args = self
            .bin
            .iter()
            .map(|binary| format!("--bin={binary}").into())
            .collect::<Vec<OsString>>();

        if self.bins {
            cargo_args.push("--bins".into());
        }

        if self.lib {
            cargo_args.push("--lib".into());
        }

        cargo_args
    }

    pub fn include_bin(&self, name: &String) -> bool {
        self.bins || (self.bin.is_empty() && !self.lib) || self.bin.contains(name)
    }

    pub fn include_lib(&self) -> bool {
        self.lib || (!self.bins && self.bin.is_empty())
    }

    pub fn include_tests(&self) -> bool {
        !self.lib && !self.bins && self.bin.is_empty()
    }
}

impl ValidateArgs for CargoTargetArgs {
    fn validate(&self) -> Result<(), Error> {
        Ok(())
    }
}

/// Arguments that Kani pass down into Cargo test essentially uninterpreted.
#[derive(Debug, Default, clap::Args)]
pub struct CargoTestArgs {
    /// Arguments to pass down to Cargo
    #[command(flatten)]
    pub common: CargoCommonArgs,

    /// Arguments used to select Cargo target.
    #[command(flatten)]
    pub target: CargoTargetArgs,
}

impl CargoTestArgs {
    /// Convert the arguments back to a format that cargo can understand.
    pub fn to_cargo_args(&self) -> Vec<OsString> {
        let mut cargo_args = self.common.to_cargo_args();
        cargo_args.append(&mut self.target.to_cargo_args());
        cargo_args
    }
}

impl ValidateArgs for CargoTestArgs {
    fn validate(&self) -> Result<(), Error> {
        self.common.validate()?;
        self.target.validate()
    }
}
