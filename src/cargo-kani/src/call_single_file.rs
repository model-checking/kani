// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

use anyhow::Result;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::session::KaniSession;
use crate::util::alter_extension;

impl KaniSession {
    /// Used by `kani` and not `cargo-kani` to process a single Rust file into a `.symtab.json`
    pub fn compile_single_rust_file(&self, file: &Path) -> Result<PathBuf> {
        let output_filename = alter_extension(file, "symtab.json");

        {
            let mut temps = self.temporaries.borrow_mut();
            temps.push(output_filename.clone());
            temps.push(alter_extension(file, "type_map.json"));
            temps.push(alter_extension(file, "kani-metadata.json"));
            if self.args.restrict_vtable() {
                temps.push(alter_extension(file, "restrictions.json"));
            }
        }

        let mut args = self.kani_rustc_flags();

        // kani-compiler workaround: *.symtab.json gets generated in the local
        // directory, instead of based on file name like we expect.
        // So we'll `cd` to that directory and only pass filename here.
        args.push(file.file_name().unwrap().into());

        if self.args.tests {
            // e.g. `tests/kani/Options/check_tests.rs` will fail because it already has it
            // so this is a hacky workaround
            let t = "--test".into();
            if !args.contains(&t) {
                args.push(t);
            }
        }

        let mut cmd = Command::new(&self.kani_rustc);
        cmd.args(args);

        // kani-compiler workaround: part 2: change directory for the subcommand
        if let Some(p) = file.canonicalize()?.parent() {
            cmd.current_dir(p);
        }

        if self.args.debug && !self.args.quiet {
            self.run_terminal(cmd)?;
        } else {
            self.run_suppress(cmd)?;
        }

        Ok(output_filename)
    }

    /// These arguments are passed directly here for single file runs,
    /// but are also used by call_cargo to pass as the env var KANIFLAGS.
    pub fn kani_rustc_flags(&self) -> Vec<OsString> {
        let mut flags = vec!["--goto-c".to_string()];

        if self.args.debug {
            flags.push("--log-level=debug".into());
        }
        if self.args.restrict_vtable() {
            flags.push("--restrict-vtable-fn-ptrs".into());
        }
        if self.args.assertion_reach_checks {
            flags.push("--assertion-reach-checks".into());
        }

        // Stratification point!
        // Above are arguments that should be parsed by kani-compiler
        // Below are arguments that should be parsed by the rustc call
        // We need to ensure these are in-order due to the way kani-compiler parses arguments. :(

        if self.args.use_abs {
            flags.push("-Z".into());
            flags.push("force-unstable-if-unmarked=yes".into()); // ??
            flags.push("--cfg=use_abs".into());
            flags.push("--cfg".into());
            flags.push(format!("abs_type={}", self.args.abs_type.to_string().to_lowercase()));
        }

        flags.push("-C".into());
        flags.push("symbol-mangling-version=v0".into());

        // e.g. compiletest will set 'compile-flags' here and we should pass those down to rustc
        // and we fail in `tests/kani/Match/match_bool.rs`
        if let Ok(str) = std::env::var("RUSTFLAGS") {
            flags.extend(str.split(' ').map(|x| x.to_string()));
        }

        flags.iter().map(|x| x.into()).collect()
    }
}
