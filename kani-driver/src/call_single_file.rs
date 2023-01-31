// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use anyhow::Result;
use std::path::Path;
use std::process::Command;
use std::{ffi::OsString, path::PathBuf};

use crate::session::{KaniSession, ReachabilityMode};

impl KaniSession {
    /// Used by `kani` and not `cargo-kani` to process a single Rust file into a `.symtab.json`
    // TODO: Move these functions to be part of the builder.

    /// This function generates all rustc configurations required by our goto-c codegen.
    fn rustc_gotoc_flags(lib_path: &str) -> Vec<String> {
        // The option below provides a mechanism by which definitions in the
        // standard library can be overriden. See
        // https://rust-lang.zulipchat.com/#narrow/stream/182449-t-compiler.2Fhelp/topic/.E2.9C.94.20Globally.20override.20an.20std.20macro/near/268873354
        // for more details.
        let kani_std_rlib = PathBuf::from(lib_path).join("libstd.rlib");
        let kani_std_wrapper = format!("noprelude:std={}", kani_std_rlib.to_str().unwrap());
        let args = vec![
            "-C",
            "overflow-checks=on",
            "-C",
            "panic=abort",
            "-Z",
            "unstable-options",
            "-Z",
            "panic_abort_tests=yes",
            "-Z",
            "trim-diagnostic-paths=no",
            "-Z",
            "human_readable_cgu_names",
            "-Z",
            "always-encode-mir",
            "--cfg=kani",
            "-Z",
            "crate-attr=feature(register_tool)",
            "-Z",
            "crate-attr=register_tool(kanitool)",
            "-L",
            lib_path,
            "--extern",
            "kani",
            "--extern",
            kani_std_wrapper.as_str(),
        ];
        args.iter().map(|s| s.to_string()).collect()
    }

    /// Convert an argument from OsStr to String.
    /// If conversion fails, panic with a custom message.
    fn convert_arg(arg: &OsStr) -> String {
        arg.to_str()
            .expect(format!("[Error] Cannot parse argument \"{arg:?}\".").as_str())
            .to_string()
    }

    /// Generate the arguments to pass to rustc_driver.
    fn generate_rustc_args(args: &ArgMatches) -> Vec<String> {
        let mut rustc_args = vec![String::from("rustc")];
        if args.get_flag(parser::GOTO_C) {
            let mut default_path = kani_root();
            if args.reachability_type() == ReachabilityType::Legacy {
                default_path.push("legacy-lib")
            } else {
                default_path.push("lib");
            }
            let gotoc_args = rustc_gotoc_flags(
                args.get_one::<String>(parser::KANI_LIB)
                    .unwrap_or(&default_path.to_str().unwrap().to_string()),
            );
            rustc_args.extend_from_slice(&gotoc_args);
        }

        if args.get_flag(parser::RUSTC_VERSION) {
            rustc_args.push(String::from("--version"))
        }

        if args.get_flag(parser::JSON_OUTPUT) {
            rustc_args.push(String::from("--error-format=json"));
        }

        if let Some(extra_flags) = args.get_raw(parser::RUSTC_OPTIONS) {
            extra_flags.for_each(|arg| rustc_args.push(convert_arg(arg)));
        }
        let sysroot = sysroot_path(args);
        rustc_args.push(String::from("--sysroot"));
        rustc_args.push(convert_arg(sysroot.as_os_str()));
        tracing::debug!(?rustc_args, "Compile");
        rustc_args
    }

    pub fn compile_single_rust_file(
        &self,
        file: &Path,
        crate_name: &String,
        outdir: &Path,
    ) -> Result<()> {
        let mut kani_args = self.kani_specific_flags();
        kani_args.push(format!("--reachability={}", self.reachability_mode()));

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
    pub fn kani_specific_flags(&self) -> Vec<String> {
        let mut flags = vec![];

        if self.args.debug {
            flags.push("--log-level=debug".into());
        } else if self.args.verbose {
            // Print the symtab command being invoked.
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
