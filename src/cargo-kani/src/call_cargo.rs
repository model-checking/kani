// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

use anyhow::Result;
use std::ffi::OsString;
use std::path::PathBuf;
use std::process::Command;

use crate::context::KaniContext;

impl KaniContext {
    /// Given a `file` (a .symtab.json), produce `{file}.out` by calling symtab2gb
    pub fn cargo_build(&self) -> Result<Vec<PathBuf>> {
        let flag_env = {
            let rustc_args = self.kani_rustc_flags();
            crate::util::join_osstring(&rustc_args, " ")
        };

        let build_target = "x86_64-unknown-linux-gnu";
        let mut args: Vec<OsString> = vec!["build".into(), "--target".into(), build_target.into()];

        if self.args.verbose {
            args.push("-v".into());
        }

        let mut cmd = Command::new("cargo");
        cmd.args(args)
            .env("RUSTC", &self.kani_rustc)
            .env("RUSTFLAGS", "--kani-flags")
            .env("KaniFLAGS", flag_env);

        if self.args.debug {
            cmd.env("KANI_LOG", "rustc_codegen_kani");
        }

        self.run_terminal(cmd)?;

        let build_glob = format!("target/{}/debug/deps/*.symtab.json", build_target);
        let results = glob::glob(&build_glob)?;

        // the logic to turn "Iter<Result<T, E>>" into "Result<Vec<T>, E>" doesn't play well
        // with anyhow, so a type annotation is required
        let symtabs: core::result::Result<Vec<PathBuf>, glob::GlobError> = results.collect();

        Ok(symtabs?)
    }
}
