// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

use anyhow::{Context, Result};
use std::ffi::OsString;
use std::path::PathBuf;
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
}

impl KaniSession {
    /// Calls `cargo_build` to generate `*.symtab.json` files in `target_dir`
    pub fn cargo_build(&self) -> Result<CargoOutputs> {
        let build_target = env!("TARGET"); // see build.rs
        let target_dir = self.args.target_dir.as_ref().unwrap_or(&PathBuf::from("target")).clone();
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

        args.push("--target".into());
        args.push(build_target.into());

        args.push("--target-dir".into());
        args.push(target_dir.clone().into());

        if self.args.verbose {
            args.push("-v".into());
        }

        let mut cmd = Command::new("cargo");
        cmd.args(args)
            .env("RUSTC", &self.kani_rustc)
            .env("RUSTFLAGS", "--kani-flags")
            .env("KANIFLAGS", flag_env);

        self.run_terminal(cmd)?;

        if self.args.dry_run {
            // mock an answer
            return Ok(CargoOutputs {
                outdir: outdir.clone(),
                symtabs: vec![target_dir.join(build_target).join("debug/deps/dry-run.symtab.json")],
                restrictions: self.args.restrict_vtable().then(|| outdir),
            });
        }

        let build_glob = target_dir.join(build_target).join("debug/deps/*.symtab.json");
        // There isn't a good way to glob with non-UTF-8 paths.
        // https://github.com/rust-lang-nursery/glob/issues/78
        let build_glob = build_glob.to_str().context("Non-UTF-8 path enountered")?;
        let results = glob::glob(build_glob)?;

        // the logic to turn "Iter<Result<T, E>>" into "Result<Vec<T>, E>" doesn't play well
        // with anyhow, so a type annotation is required
        let symtabs: core::result::Result<Vec<PathBuf>, glob::GlobError> = results.collect();

        Ok(CargoOutputs {
            outdir: outdir.clone(),
            symtabs: symtabs?,
            restrictions: self.args.restrict_vtable().then(|| outdir),
        })
    }
}
