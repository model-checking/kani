// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use anyhow::Result;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::session::{base_folder, lib_folder, KaniSession};

impl KaniSession {
    /// Used by `kani` and not `cargo-kani` to process a single Rust file into a `.symtab.json`
    // TODO: Move these functions to be part of the builder.
    pub fn compile_single_rust_file(
        &self,
        file: &Path,
        crate_name: &String,
        outdir: &Path,
    ) -> Result<()> {
        let mut kani_args = self.kani_compiler_flags();
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
        let kani_compiler_args = to_rustc_arg(kani_args);
        cmd.arg(kani_compiler_args).args(rustc_args);

        if self.args.common_args.quiet {
            self.run_suppress(cmd)?;
        } else {
            self.run_terminal(cmd)?;
        }
        Ok(())
    }

    /// Create a compiler option that represents the reachability mod.
    pub fn reachability_arg(&self) -> String {
        to_rustc_arg(vec![format!("--reachability={}", self.reachability_mode())])
    }

    /// These arguments are arguments passed to kani-compiler that are `kani` compiler specific.
    pub fn kani_compiler_flags(&self) -> Vec<String> {
        let mut flags = vec![check_version()];

        if self.args.common_args.debug {
            flags.push("--log-level=debug".into());
        } else if self.args.common_args.verbose {
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

        // Users activate it via the command line switch
        if self.args.write_json_symtab {
            flags.push("--write-json-symtab".into());
        }

        if self.args.enable_stubbing {
            flags.push("--enable-stubbing".into());
        }
        for harness in &self.args.harnesses {
            flags.push(format!("--harness={harness}"));
        }

        flags.extend(
            self.args
                .common_args
                .unstable_features
                .iter()
                .map(|feature| format!("--unstable={feature}")),
        );

        // This argument will select the Kani flavour of the compiler. It will be removed before
        // rustc driver is invoked.
        flags.push("--goto-c".into());

        flags
    }

    /// This function generates all rustc configurations required by our goto-c codegen.
    pub fn kani_rustc_flags(&self) -> Vec<OsString> {
        let lib_path = lib_folder().unwrap();
        let mut flags: Vec<_> = base_rustc_flags(lib_path);
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

        // This argument will select the Kani flavour of the compiler. It will be removed before
        // rustc driver is invoked.
        flags.push("--kani-compiler".into());

        flags
    }
}

pub fn base_rustc_flags(lib_path: PathBuf) -> Vec<OsString> {
    let kani_std_rlib = lib_path.join("libstd.rlib");
    let kani_std_wrapper = format!("noprelude:std={}", kani_std_rlib.to_str().unwrap());
    let sysroot = base_folder().unwrap();
    let mut flags = [
        "-C",
        "overflow-checks=on",
        "-C",
        "panic=abort",
        "-C",
        "symbol-mangling-version=v0",
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
        "--sysroot",
        sysroot.to_str().unwrap(),
        "-L",
        lib_path.to_str().unwrap(),
        "--extern",
        "kani",
        "--extern",
        kani_std_wrapper.as_str(),
    ]
    .map(OsString::from)
    .to_vec();

    // e.g. compiletest will set 'compile-flags' here and we should pass those down to rustc
    // and we fail in `tests/kani/Match/match_bool.rs`
    if let Ok(str) = std::env::var("RUSTFLAGS") {
        flags.extend(str.split(' ').map(OsString::from));
    }

    flags
}

/// This function can be used to convert Kani compiler specific arguments into a rustc one.
/// We currently pass Kani specific arguments using the `--llvm-args` structure which is the
/// hacky mechanism used by other rustc backend to receive arguments unknown to rustc.
///
/// Note that Cargo caching mechanism takes the building context into consideration, which
/// includes the value of the rust flags. By using `--llvm-args`, we ensure that Cargo takes into
/// consideration all arguments that are used to configure Kani compiler. For example, enabling the
/// reachability checks will force recompilation if they were disabled in previous build.
/// For more details on this caching mechanism, see the
/// [fingerprint documentation](https://github.com/rust-lang/cargo/blob/82c3bb79e3a19a5164e33819ef81bfc2c984bc56/src/cargo/core/compiler/fingerprint/mod.rs)
pub fn to_rustc_arg(kani_args: Vec<String>) -> String {
    format!(r#"-Cllvm-args={}"#, kani_args.join(" "))
}

/// Function that returns a `--check-version` argument to be added to the compiler flags.
/// This is really just used to force the compiler to recompile everything from scratch when a user
/// upgrades Kani. Cargo currently ignores the codegen backend version.
/// See <https://github.com/model-checking/kani/issues/2140> for more context.
fn check_version() -> String {
    format!("--check-version={}", env!("CARGO_PKG_VERSION"))
}
