// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

use anyhow::{bail, Context, Result};
use std::ffi::OsString;
use std::path::PathBuf;
use std::process::Command;

use crate::context::KaniContext;

impl KaniContext {
    /// Given a `file` (a .symtab.json), produce `{file}.out` by calling symtab2gb
    pub fn cargo_build(&self) -> Result<Vec<PathBuf>> {
        let path = self.hack_rustc_path("--kani-path")?;
        let flag_env = {
            let mut rustc_args = self.kani_rustc_flags();
            let additional = self.hack_rustc_path("--kani-flags")?;
            // We're going to join with spaces, so we can just put this whole string as the last element
            // of the Vec, since that will produce the correctly spaced string.
            rustc_args.push(additional);
            crate::util::join_osstring(&rustc_args, " ")
        };

        let build_target = "x86_64-unknown-linux-gnu";
        let args: Vec<OsString> =
            vec!["build".into(), "--target".into(), build_target.into(), "-v".into()]; // todo -v (only with --verbose)

        let result = Command::new("cargo")
            .args(args)
            .env("RUSTC", path)
            .env("RUSTFLAGS", flag_env)
            .status()
            .context("Failed to invoke cargo")?;

        if !result.success() {
            bail!("cargo exited with status {}", result);
        }

        let build_glob = format!("target/{}/debug/deps/*.symtab.json", build_target);
        let results = glob::glob(&build_glob)?;

        // the logic to turn "Iter<Result<T, E>>" into "Result<Vec<T>, E>" doesn't play well
        // with anyhow, so a type annotation is required
        let symtabs: core::result::Result<Vec<PathBuf>, glob::GlobError> = results.collect();

        Ok(symtabs?)
    }

    // This is surprisingly clumsy code, but it should be temporary.
    // Equivalent of bash `VAR=$(kani-rustc --arg)`
    fn hack_rustc_path(&self, arg: &str) -> Result<OsString> {
        let result = Command::new(&self.kani_rustc).args(&[arg]).output()?;
        // Note the trim: necessary to remove trailing newline!
        let output = std::str::from_utf8(&result.stdout)?.trim();

        if !result.status.success() {
            println!("{}", output);
            bail!("kani-rustc exited with status {}", result.status);
        }

        // todo Non-portable to windows. We can trust "output" to be utf8 there?
        Ok(output.into())
    }
}
