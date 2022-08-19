// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use anyhow::{Context, Result};
use cargo_metadata::MetadataCommand;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::session::KaniSession;

/// The outputs of kani-compiler being invoked via cargo on a project.
pub struct CargoOutputs {
    /// The directory where compiler outputs should be directed.
    /// Usually 'target/BUILD_TRIPLE/debug/deps/'
    pub outdir: PathBuf,
    /// The collection of *.symtab.json files written.
    pub symtabs: Vec<PathBuf>,
    /// The location of vtable restrictions files (a directory of *.restrictions.json)
    pub restrictions: Option<PathBuf>,
    /// The kani-metadata.json files written by kani-compiler.
    pub metadata: Vec<PathBuf>,
}

/// Finds the "target" directory while considering workspaces,
fn find_target_dir() -> PathBuf {
    fn maybe_get_target() -> Option<PathBuf> {
        Some(MetadataCommand::new().exec().ok()?.target_directory.into())
    }

    maybe_get_target().unwrap_or(PathBuf::from("target"))
}

impl KaniSession {
    /// Calls `cargo_build` to generate `*.symtab.json` files in `target_dir`
    pub fn cargo_build(&self) -> Result<CargoOutputs> {
        let build_target = env!("TARGET"); // see build.rs
        let target_dir = self.args.target_dir.as_ref().unwrap_or(&find_target_dir()).clone();
        let outdir = target_dir.join(build_target).join("debug/deps");

        let flag_env = {
            let rustc_args = self.kani_rustc_flags();
            crate::util::join_osstring(&rustc_args, " ")
        };

        let mut args: Vec<OsString> = Vec::new();

        if self.args.tests {
            args.push("test".into());
            args.push("--no-run".into());
        } else {
            args.push("build".into());
        }

        if let Some(package) = self.args.package.as_ref() {
            args.push("--package".into());
            args.push(package.into());
        }

        if self.args.all_features {
            args.push("--all-features".into());
        }

        if self.args.workspace {
            args.push("--workspace".into());
        }

        args.push("--target".into());
        args.push(build_target.into());

        args.push("--target-dir".into());
        args.push(target_dir.into());

        if self.args.verbose {
            args.push("-v".into());
        }

        let mut cmd = Command::new("cargo");
        cmd.args(args)
            .env("RUSTC", &self.kani_compiler)
            .env("RUSTFLAGS", "--kani-flags")
            .env("KANIFLAGS", flag_env);

        self.run_terminal(cmd)?;

        if self.args.dry_run {
            // mock an answer: mostly the same except we don't/can't expand the globs
            return Ok(CargoOutputs {
                outdir: outdir.clone(),
                symtabs: vec![outdir.join("*.symtab.json")],
                metadata: vec![outdir.join("*.kani-metadata.json")],
                restrictions: self.args.restrict_vtable().then_some(outdir),
            });
        }

        Ok(CargoOutputs {
            outdir: outdir.clone(),
            symtabs: glob(&outdir.join("*.symtab.json"))?,
            metadata: glob(&outdir.join("*.kani-metadata.json"))?,
            restrictions: self.args.restrict_vtable().then_some(outdir),
        })
    }
}

/// Given a `path` with glob characters in it (e.g. `*.json`), return a vector of matching files
fn glob(path: &Path) -> Result<Vec<PathBuf>> {
    let results = glob::glob(path.to_str().context("Non-UTF-8 path enountered")?)?;
    // the logic to turn "Iter<Result<T, E>>" into "Result<Vec<T>, E>" doesn't play well
    // with anyhow, so a type annotation is required
    let v: core::result::Result<Vec<PathBuf>, glob::GlobError> = results.collect();
    Ok(v?)
}
