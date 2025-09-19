// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Module that provides functions which are convenient for different purposes.
//!
//! In particular, the `warning` and `error` functions must be used for
//! diagnostic output across the `kani-driver` components. Please follow the
//! recommendations in <https://model-checking.github.io/kani/conventions.html>
//! when reporting any kind of diagnostic for users. Note that it's recommended
//! to use the Rust compiler's error message utilities if you're working on the
//! `kani-compiler`.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::Result;
use kani_metadata::HarnessMetadata;
use serde_json::json;
use crate::frontend::JsonHandler;
use crate::harness_runner::HarnessResult;
use crate::call_cbmc::VerificationStatus;

/// Replace an extension with another one, in a new PathBuf. (See tests for examples)
pub fn alter_extension(path: &Path, ext: &str) -> PathBuf {
    path.with_extension(ext)
}

/// Generate a valid crate name from the input file.
/// Note that this method will replace invalid characters from the crate name.
pub fn crate_name(path: &Path) -> String {
    let stem = path.file_stem().unwrap().to_str().expect("utf-8 filename");
    stem.replace(['-', '.'], "_")
}

/// Given a path of some sort (usually from argv0), this attempts to extract the basename / stem
/// of the executable. e.g. "/path/foo -> foo" "./foo.exe -> foo" "foo -> foo"
pub fn executable_basename(argv0: &Option<&OsString>) -> Option<OsString> {
    if let Some(path) = argv0 {
        let basename = Path::new(&path).file_stem();
        if let Some(stem) = basename {
            return Some(stem.to_os_string());
        }
    }
    None
}

/// Render a Command as a string, to log it (e.g. in dry runs)
pub fn render_command(cmd: &Command) -> OsString {
    let mut str = OsString::new();

    for (k, v) in cmd.get_envs() {
        if let Some(v) = v {
            str.push(k);
            str.push("=\"");
            str.push(v);
            str.push("\" ");
        }
    }

    str.push(cmd.get_program());

    for a in cmd.get_args() {
        str.push(" ");
        if a.to_string_lossy().contains(' ') {
            str.push("\"");
            str.push(a);
            str.push("\"");
        } else {
            str.push(a);
        }
    }

    str
}

/// Print a warning message. This will add a "warning:" tag before the message and style accordingly.
pub fn warning(msg: &str) {
    let warning = console::style("warning:").bold().yellow();
    let msg_fmt = console::style(msg).bold();
    println!("{warning} {msg_fmt}")
}

/// Print an error message. This will add an "error:" tag before the message and style accordingly.
pub fn error(msg: &str) {
    let error = console::style("error:").bold().red();
    let msg_fmt = console::style(msg).bold();
    println!("{error} {msg_fmt}")
}

/// Print an info message. This will print the stage in bold green and the rest in regular style.
pub fn info_operation(op: &str, msg: &str) {
    let op_fmt = console::style(op).bold().green();
    let msg_fmt = console::style(msg);
    println!("{op_fmt} {msg_fmt}")
}

/// Process harness results and enrich JSON handler with additional metadata.
/// This utility function handles the complex harness processing logic that was previously in main.rs.
pub fn process_harness_results(
    handler: &mut JsonHandler,
    harnesses: &[&HarnessMetadata],
    results: &[HarnessResult],
) -> Result<()> {
    for h in harnesses {
        let harness_result = results.iter().find(|r| r.harness.pretty_name == h.pretty_name);
        let arr = handler.data["verification_runner_results"]["individual_harnesses"]
            .as_array_mut()
            .expect("individual_harnesses must be an array");
        
        // locate matching entry by harness_id and overwrite it
        let entry = arr.iter_mut().find(|v| {
            v.get("harness_id").and_then(|s| s.as_str()) == Some(h.pretty_name.as_str())
        }).expect("matching individual_harness not found");

        // Get the original verification details from the entry before overwriting
        let verification_details = entry.get("verification_details").cloned().unwrap_or(json!([]));
        let status = entry.get("status").and_then(|s| s.as_str()).unwrap_or("Unknown");
        
        *entry = json!({
            "harness_id": h.pretty_name,                    // Keep harness_id for consistency
            "name": h.pretty_name,                          // Also keep name for backward compatibility
            "status": status,                               // Preserve the verification status
            "verification_details": verification_details,   // Preserve verification details
            
            //original source location
            "original": {
                "file": h.original_file,
                "start_line": h.original_start_line,
                "end_line": h.original_end_line
            },

            // attributes
            "kind": format!("{:?}", h.attributes.kind),
            "should_panic": h.attributes.should_panic,
            "has_loop_contracts": h.has_loop_contracts,
            "is_automatically_generated": h.is_automatically_generated,
            "solver": h.attributes.solver.as_ref().map(|s| format!("{:?}", s)),
            "unwind_value": h.attributes.unwind_value,
            "contract": h.contract.as_ref().map(|c| format!("{:?}", c)),
            "stubs": h.attributes.stubs.iter().map(|s| format!("{:?}", s)).collect::<Vec<_>>(),
            "verified_stubs": h.attributes.verified_stubs,

            "summary": harness_result.map_or(json!(null), |result| json!({
                "total": 1,
                "status": match result.result.status {
                    VerificationStatus::Success => "completed",
                    VerificationStatus::Failure => "failed",
                }
            })),
            "timing": harness_result.map_or(json!(null), |result| json!({
                "cbmc_runtime": format!("{:.3}s", result.result.runtime.as_secs_f64())
            }))
        });
        
        // Add error details for this harness
        handler.add_item("error_details", harness_result.map_or(json!(null), |result| {
            match result.result.status {
                VerificationStatus::Failure => {
                    json!({
                        "has_errors": true,
                        "error_type": match result.result.failed_properties {
                            crate::call_cbmc::FailedProperties::None => "unknown_failure",
                            crate::call_cbmc::FailedProperties::PanicsOnly => "assertion_failure",
                            crate::call_cbmc::FailedProperties::Other => "verification_failure",
                        },
                        "failed_properties_type": format!("{:?}", result.result.failed_properties),
                        "exit_status": match &result.result.results {
                            Err(crate::call_cbmc::ExitStatus::Timeout) => "timeout".to_string(),
                            Err(crate::call_cbmc::ExitStatus::OutOfMemory) => "out_of_memory".to_string(),
                            Err(crate::call_cbmc::ExitStatus::Other(code)) => format!("exit_code_{}", code),
                            Ok(_) => "properties_failed".to_string()
                        }
                    })
                },
                VerificationStatus::Success => json!({
                    "has_errors": false
                })
            }
        }));
        
        // Add property details for this harness
        handler.add_harness_detail("property_details", json!({
            "property_details": harness_result.map_or(json!(null), |result| {
                match &result.result.results {
                    Ok(properties) => {
                        let total_properties = properties.len();
                        let passed_properties = properties.iter().filter(|p| matches!(p.status, crate::cbmc_output_parser::CheckStatus::Success)).count();
                        let failed_properties = properties.iter().filter(|p| matches!(p.status, crate::cbmc_output_parser::CheckStatus::Failure)).count();
                        
                        json!({
                            "total_properties": total_properties,
                            "passed": passed_properties,
                            "failed": failed_properties,
                            "unreachable": total_properties - passed_properties - failed_properties
                        })
                    },
                    Err(_) => json!({
                        "total_properties": 0,
                        "error": "Could not extract property details due to verification failure"
                    })
                }
            })
        }));
    }
    
    Ok(())
}

/// # Kani Argument Types
///
/// We have three different kinds of arguments we use to influence our compilation process.
/// This module provides specific types, conversions and methods for each to codify the differences betweeen them
/// and how they can be used.
///
/// [`KaniArg`](args::KaniArg) -- First, there are Kani-specific arguments that the `kani-compiler` uses to direct its codegen.
/// (See the [`Arguments`] struct in `kani-compiler/src/args.rs` for how they are parsed.)
///
/// [`RustcArg`](args::RustcArg) -- Before codegen, the Kani compiler invokes `rustc` to compile Rust source to MIR,
/// and we pass arguments to influence that compilation.
/// See [`LibConfig`](crate::call_single_file::LibConfig) for how this is used in a typical Kani invocation.
///
/// [`CargoArg`](args::CargoArg) -- If we're calling Kani through `cargo kani`, we also want to pass arguments to Cargo itself.
///
///
/// ## Usage
/// * `CargoArg`s can be passed directly to a `&mut Command` using the `pass_cargo_args` method.
/// * `RustcArg`s can also be passed directly to a `&mut Command` using the `pass_rustc_arg{s}` methods,
///   with the added wrinkle that you have to specify whether they should be passed to all crates or just the local one.
///   If passing to `AllCrates`, this uses an environment variable, meaning that it can only be called once for each Command.
/// * Any `KaniArg`s must first be converted into a single `RustcArg` with the [`to_rustc_arg`](args::to_rustc_arg) function
///   before being passed to commands the same way as other `RustcArg`s. We do this (rather than having a separate
///   `pass_kani_args` function) because both kinds of arguments often have to share the same underlying `pass_rustc_args`
///   call to ensure the environment variables for one doesn't get overwritten as mentioned above.
pub(crate) mod args {
    use std::{
        ffi::{OsStr, OsString},
        process::Command,
    };

    #[derive(Clone, PartialEq)]
    /// Kani-specific arguments passed to `rustc` and then used by `kani-compiler`.
    pub struct KaniArg(String);

    #[derive(Clone, PartialEq, Debug)]
    /// Arguments passed to `rustc`.
    pub struct RustcArg(OsString);

    #[derive(Clone, PartialEq)]
    /// Arguments passed to Cargo.
    pub struct CargoArg(OsString);

    macro_rules! from_impl {
        ($type:tt, $inner:ty) => {
            impl<T> From<T> for $type
            where
                T: Into<$inner>,
            {
                fn from(value: T) -> Self {
                    $type(value.into())
                }
            }

            impl $type {
                /// Get a reference to this argument's underlying type.
                pub fn as_inner(&self) -> &$inner {
                    &self.0
                }
            }
        };
    }

    from_impl!(KaniArg, String);
    from_impl!(RustcArg, OsString);
    from_impl!(CargoArg, OsString);

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
    pub fn encode_as_rustc_arg<'a>(kani_args: impl IntoIterator<Item = &'a KaniArg>) -> RustcArg {
        format!(
            r#"-Cllvm-args={}"#,
            kani_args
                .into_iter()
                .map(KaniArg::as_inner)
                .cloned()
                .collect::<Vec<String>>()
                .join(" ")
        )
        .into()
    }

    pub enum PassTo {
        /// TODO: Only pass arguments for use in the local crate.
        /// This will just pass them directly as arguments to the command.
        OnlyLocalCrate,
        /// TODO: Pass arguments for use when compiling all dependencies using the
        /// `CARGO_ENCODED_RUSTFLAGS` environment variable.
        AllCrates,
    }

    /// A wrapper trait that allows us to call these methods on a [Command].
    pub trait CommandWrapper {
        fn pass_cargo_args(&mut self, args: &[CargoArg]) -> &mut Self;
        fn pass_rustc_args(&mut self, args: &[RustcArg], to: PassTo) -> &mut Self;
        fn pass_rustc_arg(&mut self, args: RustcArg, to: PassTo) -> &mut Self;
    }

    impl CommandWrapper for Command {
        /// Pass general arguments to cargo.
        fn pass_cargo_args(&mut self, args: &[CargoArg]) -> &mut Self {
            self.args(args.iter().map(CargoArg::as_inner))
        }

        fn pass_rustc_arg(&mut self, args: RustcArg, to: PassTo) -> &mut Self {
            self.pass_rustc_args(&[args], to)
        }

        /// Pass rustc arguments to the compiler for use in certain dependencies.
        fn pass_rustc_args(&mut self, args: &[RustcArg], to: PassTo) -> &mut Self {
            match to {
                // Since we just want to pass to the local crate, just add them as arguments to the command.
                PassTo::OnlyLocalCrate => self.args(args.iter().map(RustcArg::as_inner)),

                // Since we also want to recursively pass these args to all dependencies,
                // use an environment variable that gets checked for each dependency.
                PassTo::AllCrates => {
                    // TODO: Use of CARGO_ENCODED_RUSTFLAGS instead of RUSTFLAGS is preferred. See
                    // https://doc.rust-lang.org/cargo/reference/environment-variables.html
                    let env_var = OsString::from("CARGO_ENCODED_RUSTFLAGS");

                    // Ensure we wouldn't be overwriting an existing environment variable.
                    let env_var_exists = self.get_envs().any(|(var, _)| var == env_var);
                    assert!(
                        !env_var_exists,
                        "pass_rustc_args() uses an environment variable when called with `PassTo::AllCrates`, \
                        so calling it multiple times in this way will overwrite all but the most recent call. \
                        try combining the arguments you want to add and passing them to a single call instead."
                    );

                    self.env(
                        "CARGO_ENCODED_RUSTFLAGS",
                        args.iter()
                            .map(RustcArg::as_inner)
                            .cloned()
                            .collect::<Vec<OsString>>()
                            .join(OsStr::new("\x1f")),
                    )
                }
            }
        }
    }
}
