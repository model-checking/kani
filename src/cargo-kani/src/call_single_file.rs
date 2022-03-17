// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

use anyhow::{Context, Result};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::session::KaniSession;
use crate::util::alter_extension;

/// The outputs of kani-compiler operating on a single Rust source file.
pub struct SingleOutputs {
    /// The directory where compiler outputs should be directed.
    /// May be '.' or a path for 'kani', usually under 'target/' for 'cargo-kani'
    pub outdir: PathBuf,
    /// The *.symtab.json written.
    pub symtab: PathBuf,
    /// The vtable restrictions files, if any.
    pub restrictions: Option<PathBuf>,
    /// The kani-metadata.json file written by kani-compiler.
    pub metadata: PathBuf,
}

impl KaniSession {
    /// Used by `kani` and not `cargo-kani` to process a single Rust file into a `.symtab.json`
    pub fn compile_single_rust_file(&self, file: &Path) -> Result<SingleOutputs> {
        let outdir =
            file.canonicalize()?.parent().context("File doesn't exist in a directory?")?.to_owned();
        let output_filename = alter_extension(file, "symtab.json");
        let typemap_filename = alter_extension(file, "type_map.json");
        let metadata_filename = alter_extension(file, "kani-metadata.json");
        let restrictions_filename = alter_extension(file, "restrictions.json");

        {
            let mut temps = self.temporaries.borrow_mut();
            temps.push(output_filename.clone());
            temps.push(typemap_filename);
            temps.push(metadata_filename.clone());
            if self.args.restrict_vtable() {
                temps.push(restrictions_filename.clone());
            }
        }

        let mut args = self.kani_rustc_flags();

        // kani-compiler workaround part 1/2: *.symtab.json gets generated in the local
        // directory, instead of based on file name like we expect.
        // So let we'll `cd` to that directory and here we only pass filename.
        args.push(file.file_name().unwrap().into());

        if self.args.tests {
            // e.g. `tests/kani/Options/check_tests.rs` will fail because it already has it
            // so this is a hacky workaround
            let t = "--test".into();
            if !args.contains(&t) {
                args.push(t);
            }
        } else {
            // Don't require a 'main' function to exist. We only run against proof harnesses.
            args.push("--crate-type".into());
            args.push("lib".into());
        }

        let mut cmd = Command::new(&self.kani_compiler);
        cmd.args(args);

        // kani-compiler workaround: part 2/2: change directory for the subcommand
        cmd.current_dir(&outdir);

        if self.args.quiet {
            self.run_suppress(cmd)?;
        } else {
            self.run_terminal(cmd)?;
        }

        Ok(SingleOutputs {
            outdir,
            symtab: output_filename,
            metadata: metadata_filename,
            restrictions: if self.args.restrict_vtable() {
                Some(restrictions_filename)
            } else {
                None
            },
        })
    }

    /// These arguments are passed directly here for single file runs,
    /// but are also used by call_cargo to pass as the env var KANIFLAGS.
    pub fn kani_rustc_flags(&self) -> Vec<OsString> {
        let mut flags = vec!["--goto-c".to_string()];

        if self.args.debug {
            flags.push("--log-level=debug".into());
        } else if self.args.verbose {
            flags.push("--log-level=info".into());
        } else {
            flags.push("--log-level=warn".into());
        }

        if self.args.restrict_vtable() {
            flags.push("--restrict-vtable-fn-ptrs".into());
        }
        if !self.args.no_assertion_reach_checks {
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
