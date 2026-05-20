// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Implements the `verify-artifacts` subcommand handling.

use crate::args::{ValidateArgs, VerificationArgs};
use clap::error::ErrorKind;
use clap::{Error, Parser};
use kani_metadata::UnstableFeature;
use std::path::PathBuf;

/// Verify pre-built compiler artifacts without rebuilding.
///
/// Reads `*.kani-metadata.json` and `*.symtab.out` files produced by a previous
/// `kani-compiler` invocation (e.g. `cargo kani --only-codegen`, or an external
/// build system that drives `kani-compiler` directly) and runs the verification
/// pipeline. Artifacts must be co-located with their metadata file — this is
/// kani-compiler's emit layout. The directory must be writable: the linker
/// writes the linked goto binary next to each `.symtab.out`.
///
/// This is an **unstable option**. The artifacts must have been produced by the
/// same kani version that verifies them; `kani-driver` does not check.
#[derive(Debug, Parser)]
pub struct VerifyArtifactsArgs {
    /// Directories containing pre-built `*.kani-metadata.json` and
    /// `*.symtab.out` artifacts.
    #[arg(required = true)]
    pub artifact_dirs: Vec<PathBuf>,

    #[command(flatten)]
    pub verify_opts: VerificationArgs,
}

impl ValidateArgs for VerifyArtifactsArgs {
    fn validate(&self) -> Result<(), Error> {
        self.verify_opts.validate()?;

        if !self
            .verify_opts
            .common_args
            .unstable_features
            .contains(UnstableFeature::UnstableOptions)
        {
            return Err(Error::raw(
                ErrorKind::MissingRequiredArgument,
                "The `verify-artifacts` subcommand is unstable and requires -Z unstable-options",
            ));
        }

        for dir in &self.artifact_dirs {
            if !dir.is_dir() {
                return Err(Error::raw(
                    ErrorKind::InvalidValue,
                    format!(
                        "Artifact directory `{}` does not exist or is not a directory",
                        dir.display()
                    ),
                ));
            }
        }

        Ok(())
    }
}
