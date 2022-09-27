// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use anyhow::{Context, Result};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::session::KaniSession;
use crate::util::{alter_extension, guess_rlib_name};

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
        let rlib_filename = guess_rlib_name(file);

        {
            let mut temps = self.temporaries.borrow_mut();
            temps.push(rlib_filename);
            temps.push(output_filename.clone());
            temps.push(typemap_filename);
            temps.push(metadata_filename.clone());
            if self.args.restrict_vtable() {
                temps.push(restrictions_filename.clone());
            }
        }

        let mut kani_args = self.kani_specific_flags();
        if self.args.mir_linker {
            kani_args.push("--reachability=harnesses".into());
        } else {
            kani_args.push("--reachability=legacy".into());
        }

        let mut rustc_args = self.kani_rustc_flags();
        // kani-compiler workaround part 1/2: *.symtab.json gets generated in the local
        // directory, instead of based on file name like we expect.
        // So let we'll `cd` to that directory and here we only pass filename.
        rustc_args.push(file.file_name().unwrap().into());

        if self.args.tests {
            // e.g. `tests/kani/Options/check_tests.rs` will fail because it already has it
            // so this is a hacky workaround
            let t = "--test".into();
            if !rustc_args.contains(&t) {
                rustc_args.push(t);
            }
        } else {
            // If we specifically request "--function main" then don't override crate type
            if Some("main".to_string()) != self.args.function {
                // We only run against proof harnesses normally, and this change
                // 1. Means we do not require a `fn main` to exist
                // 2. Don't forget it also changes visibility rules.
                rustc_args.push("--crate-type".into());
                rustc_args.push("lib".into());
            }
        }

        // Note that the order of arguments is important. Kani specific flags should precede
        // rustc ones.
        let mut cmd = Command::new(&self.kani_compiler);
        cmd.args(kani_args).args(rustc_args);

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

    /// These arguments are arguments passed to kani-compiler that are `kani` specific.
    /// These are also used by call_cargo to pass as the env var KANIFLAGS.
    pub fn kani_specific_flags(&self) -> Vec<OsString> {
        let mut flags = vec![OsString::from("--goto-c")];

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
        if self.args.assertion_reach_checks() {
            flags.push("--assertion-reach-checks".into());
        }
        if self.args.ignore_global_asm {
            flags.push("--ignore-global-asm".into());
        }

        #[cfg(feature = "unsound_experiments")]
        flags.extend(self.args.unsound_experiments.process_args());

        flags
    }

    /// These arguments are arguments passed to kani-compiler that are `rustc` specific.
    /// These are also used by call_cargo to pass as the env var KANIFLAGS.
    pub fn kani_rustc_flags(&self) -> Vec<OsString> {
        let mut flags = Vec::<OsString>::new();
        if self.args.use_abs {
            flags.push("-Z".into());
            flags.push("force-unstable-if-unmarked=yes".into()); // ??
            flags.push("--cfg=use_abs".into());
            flags.push("--cfg".into());
            let abs_type = format!("abs_type={}", self.args.abs_type.to_string().to_lowercase());
            flags.push(abs_type.into());
        }

        if let Some(seed_opt) = self.args.randomize_layout {
            flags.push("-Z".into());
            flags.push("randomize-layout".into());
            if let Some(seed) = seed_opt {
                flags.push("-Z".into());
                flags.push(format!("layout-seed={}", seed).into());
            }
        }

        flags.push("-C".into());
        flags.push("symbol-mangling-version=v0".into());

        // e.g. compiletest will set 'compile-flags' here and we should pass those down to rustc
        // and we fail in `tests/kani/Match/match_bool.rs`
        if let Ok(str) = std::env::var("RUSTFLAGS") {
            flags.extend(str.split(' ').map(OsString::from));
        }

        flags
    }
}
