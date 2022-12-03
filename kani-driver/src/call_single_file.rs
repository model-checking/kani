// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use anyhow::Result;
use std::ffi::OsString;
use std::path::Path;
use std::process::Command;

use crate::session::{KaniSession, ReachabilityMode};

impl KaniSession {
    /// Used by `kani` and not `cargo-kani` to process a single Rust file into a `.symtab.json`
    // TODO: Move these functions to be part of the builder.
    pub fn compile_single_rust_file(
        &self,
        file: &Path,
        crate_name: &String,
        outdir: &Path,
    ) -> Result<()> {
        let mut kani_args = self.kani_specific_flags();
        kani_args.push(
            match self.reachability_mode() {
                ReachabilityMode::Legacy => "--reachability=legacy",
                ReachabilityMode::ProofHarnesses => "--reachability=harnesses",
                ReachabilityMode::AllPubFns => "--reachability=pub_fns",
                ReachabilityMode::Tests => "--reachability=tests",
            }
            .into(),
        );

        let mut rustc_args = self.kani_rustc_flags();
        rustc_args.push(file.into());
        rustc_args.push("--out-dir".into());
        rustc_args.push(OsString::from(outdir.as_os_str()));
        rustc_args.push("--crate-name".into());
        rustc_args.push(crate_name.into());

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

        if self.args.quiet {
            self.run_suppress(cmd)?;
        } else {
            self.run_terminal(cmd)?;
        }
        Ok(())
    }

    /// These arguments are arguments passed to kani-compiler that are `kani` specific.
    /// These are also used by call_cargo to pass as the env var KANIFLAGS.
    pub fn kani_specific_flags(&self) -> Vec<OsString> {
        let mut flags = vec![OsString::from("--goto-c")];

        if self.args.debug {
            flags.push("--log-level=debug".into());
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

        if self.args.enable_stubbing {
            flags.push("--enable-stubbing".into());
        }
        if let Some(harness) = &self.args.harness {
            flags.push(format!("--harness={harness}").into());
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
                flags.push(format!("layout-seed={seed}").into());
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
