// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use crate::args::KaniArgs;
use crate::call_single_file::to_rustc_arg;
use crate::session::KaniSession;
use anyhow::{bail, Context, Result};
use cargo_metadata::diagnostic::{Diagnostic, DiagnosticLevel};
use cargo_metadata::{Message, Metadata, MetadataCommand, Package};
use std::ffi::{OsStr, OsString};
use std::fs;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::{debug, trace};

//---- Crate types identifier used by cargo.
const CRATE_TYPE_BIN: &str = "bin";
const CRATE_TYPE_CDYLIB: &str = "cdylib";
const CRATE_TYPE_DYLIB: &str = "dylib";
const CRATE_TYPE_LIB: &str = "lib";
const CRATE_TYPE_PROC_MACRO: &str = "proc-macro";
const CRATE_TYPE_RLIB: &str = "rlib";
const CRATE_TYPE_STATICLIB: &str = "staticlib";
const CRATE_TYPE_TEST: &str = "test";

/// The outputs of kani-compiler being invoked via cargo on a project.
pub struct CargoOutputs {
    /// The directory where compiler outputs should be directed.
    /// Usually 'target/BUILD_TRIPLE/debug/deps/'
    pub outdir: PathBuf,
    /// The collection of *.symtab.out goto binary files written.
    pub symtab_gotos: Vec<PathBuf>,
    /// The location of vtable restrictions files (a directory of *.restrictions.json)
    pub restrictions: Option<PathBuf>,
    /// The kani-metadata.json files written by kani-compiler.
    pub metadata: Vec<PathBuf>,
    /// Recording the cargo metadata from the build
    pub cargo_metadata: Metadata,
}

impl KaniSession {
    /// Calls `cargo_build` to generate `*.symtab.json` files in `target_dir`
    pub fn cargo_build(&self) -> Result<CargoOutputs> {
        let build_target = env!("TARGET"); // see build.rs
        let metadata = self.cargo_metadata(build_target)?;
        let target_dir = self
            .args
            .target_dir
            .as_ref()
            .unwrap_or(&metadata.target_directory.clone().into())
            .clone()
            .join("kani");
        let outdir = target_dir.join(build_target).join("debug/deps");

        // Clean directory before building since we are unable to handle cache today.
        // TODO: https://github.com/model-checking/kani/issues/1736
        if target_dir.exists() {
            fs::remove_dir_all(&target_dir)?;
        }

        let mut rustc_args = self.kani_rustc_flags();
        rustc_args.push(to_rustc_arg(self.kani_compiler_flags()).into());

        let mut cargo_args: Vec<OsString> = vec!["rustc".into()];
        if let Some(path) = &self.args.cargo.manifest_path {
            cargo_args.push("--manifest-path".into());
            cargo_args.push(path.into());
        }
        if self.args.cargo.all_features {
            cargo_args.push("--all-features".into());
        }
        if self.args.cargo.no_default_features {
            cargo_args.push("--no-default-features".into());
        }
        let features = self.args.cargo.features();
        if !features.is_empty() {
            cargo_args.push(format!("--features={}", features.join(",")).into());
        }

        cargo_args.push("--target".into());
        cargo_args.push(build_target.into());

        cargo_args.push("--target-dir".into());
        cargo_args.push(target_dir.into());

        // Configuration needed to parse cargo compilation status.
        cargo_args.push("--message-format".into());
        cargo_args.push("json-diagnostic-rendered-ansi".into());

        if self.args.tests {
            // Use test profile in order to pull dev-dependencies and compile using `--test`.
            // Initially the plan was to use `--tests` but that brings in multiple targets.
            cargo_args.push("--profile".into());
            cargo_args.push("test".into());
        }

        if self.args.verbose {
            cargo_args.push("-v".into());
        }

        // Arguments that will only be passed to the target package.
        let mut pkg_args: Vec<String> = vec![];
        pkg_args.extend(["--".to_string(), self.reachability_arg()]);

        let mut found_target = false;
        let packages = packages_to_verify(&self.args, &metadata);
        for package in packages {
            for target in package_targets(&self.args, package) {
                let mut cmd = Command::new("cargo");
                cmd.args(&cargo_args)
                    .args(vec!["-p", &package.name])
                    .args(&target.to_args())
                    .args(&pkg_args)
                    .env("RUSTC", &self.kani_compiler)
                    // Use CARGO_ENCODED_RUSTFLAGS instead of RUSTFLAGS is preferred. See
                    // https://doc.rust-lang.org/cargo/reference/environment-variables.html
                    .env("CARGO_ENCODED_RUSTFLAGS", rustc_args.join(OsStr::new("\x1f")))
                    .env("CARGO_TERM_PROGRESS_WHEN", "never");

                self.run_cargo(cmd)?;
                found_target = true;
            }
        }

        if !found_target {
            bail!("No supported targets were found.");
        }

        Ok(CargoOutputs {
            outdir: outdir.clone(),
            symtab_gotos: glob(&outdir.join("*.symtab.out"))?,
            metadata: glob(&outdir.join("*.kani-metadata.json"))?,
            restrictions: self.args.restrict_vtable().then_some(outdir),
            cargo_metadata: metadata,
        })
    }

    fn cargo_metadata(&self, build_target: &str) -> Result<Metadata> {
        let mut cmd = MetadataCommand::new();

        // restrict metadata command to host platform. References:
        // https://github.com/rust-lang/rust-analyzer/issues/6908
        // https://github.com/rust-lang/rust-analyzer/pull/6912
        cmd.other_options(vec![String::from("--filter-platform"), build_target.to_owned()]);

        // Set a --manifest-path if we're given one
        if let Some(path) = &self.args.cargo.manifest_path {
            cmd.manifest_path(path);
        }
        // Pass down features enables, which may affect dependencies or build metadata
        // (multiple calls to features are ok with cargo_metadata:)
        if self.args.cargo.all_features {
            cmd.features(cargo_metadata::CargoOpt::AllFeatures);
        }
        if self.args.cargo.no_default_features {
            cmd.features(cargo_metadata::CargoOpt::NoDefaultFeatures);
        }
        let features = self.args.cargo.features();
        if !features.is_empty() {
            cmd.features(cargo_metadata::CargoOpt::SomeFeatures(features));
        }

        cmd.exec().context("Failed to get cargo metadata.")
    }

    /// Run cargo and collect any error found.
    /// TODO: We should also use this to collect the artifacts generated by cargo.
    fn run_cargo(&self, cargo_cmd: Command) -> Result<()> {
        let support_color = atty::is(atty::Stream::Stdout);
        if let Some(mut cargo_process) = self.run_piped(cargo_cmd)? {
            let reader = BufReader::new(cargo_process.stdout.take().unwrap());
            let mut error_count = 0;
            for message in Message::parse_stream(reader) {
                let message = message.unwrap();
                match message {
                    Message::CompilerMessage(msg) => match msg.message.level {
                        DiagnosticLevel::FailureNote => {
                            print_msg(&msg.message, support_color)?;
                        }
                        DiagnosticLevel::Error => {
                            error_count += 1;
                            print_msg(&msg.message, support_color)?;
                        }
                        DiagnosticLevel::Ice => {
                            print_msg(&msg.message, support_color)?;
                            let _ = cargo_process.wait();
                            return Err(anyhow::Error::msg(msg.message).context(format!(
                                "Failed to compile `{}` due to an internal compiler error.",
                                msg.target.name
                            )));
                        }
                        _ => {
                            if !self.args.quiet {
                                print_msg(&msg.message, support_color)?;
                            }
                        }
                    },
                    Message::CompilerArtifact(_)
                    | Message::BuildScriptExecuted(_)
                    | Message::BuildFinished(_) => {
                        // do nothing
                    }
                    Message::TextLine(msg) => {
                        if !self.args.quiet {
                            println!("{msg}");
                        }
                    }

                    // Non-exhaustive enum.
                    _ => {
                        if !self.args.quiet {
                            println!("{message:?}");
                        }
                    }
                }
            }
            let status = cargo_process.wait()?;
            if !status.success() {
                bail!(
                    "Failed to execute cargo ({status}). Found {error_count} compilation errors."
                );
            }
        }
        Ok(())
    }
}

/// Print the compiler message following the coloring schema.
fn print_msg(diagnostic: &Diagnostic, use_rendered: bool) -> Result<()> {
    if use_rendered {
        print!("{diagnostic}");
    } else {
        print!("{}", console::strip_ansi_codes(diagnostic.rendered.as_ref().unwrap()));
    }
    Ok(())
}

/// Given a `path` with glob characters in it (e.g. `*.json`), return a vector of matching files
fn glob(path: &Path) -> Result<Vec<PathBuf>> {
    let results = glob::glob(path.to_str().context("Non-UTF-8 path enountered")?)?;
    // the logic to turn "Iter<Result<T, E>>" into "Result<Vec<T>, E>" doesn't play well
    // with anyhow, so a type annotation is required
    let v: core::result::Result<Vec<PathBuf>, glob::GlobError> = results.collect();
    Ok(v?)
}

/// Extract the packages that should be verified.
/// If `--package <pkg>` is given, return the list of packages selected.
/// If `--workspace` is given, return the list of workspace members.
/// If no argument provided, return the root package if there's one or all members.
///   - I.e.: Do whatever cargo does when there's no `default_members`.
///   - This is because `default_members` is not available in cargo metadata.
///     See <https://github.com/rust-lang/cargo/issues/8033>.
fn packages_to_verify<'b>(args: &KaniArgs, metadata: &'b Metadata) -> Vec<&'b Package> {
    debug!(package_selection=?args.cargo.package, workspace=args.cargo.workspace, "packages_to_verify args");
    let packages = if !args.cargo.package.is_empty() {
        args.cargo
            .package
            .iter()
            .map(|pkg_name| {
                metadata
                    .packages
                    .iter()
                    .find(|pkg| pkg.name == *pkg_name)
                    .expect(&format!("Cannot find package '{pkg_name}'"))
            })
            .collect()
    } else {
        match (args.cargo.workspace, metadata.root_package()) {
            (true, _) | (_, None) => metadata.workspace_packages(),
            (_, Some(root_pkg)) => vec![root_pkg],
        }
    };
    trace!(?packages, "packages_to_verify result");
    packages
}

/// Possible verification targets.
enum VerificationTarget {
    Bin(String),
    Lib,
    Test(String),
}

impl VerificationTarget {
    /// Convert to cargo argument that select the specific target.
    fn to_args(&self) -> Vec<String> {
        match self {
            VerificationTarget::Test(name) => vec![String::from("--test"), name.clone()],
            VerificationTarget::Bin(name) => vec![String::from("--bin"), name.clone()],
            VerificationTarget::Lib => vec![String::from("--lib")],
        }
    }
}

/// Extract the targets inside a package.
///
/// If `--tests` is given, the list of targets will include any integration tests.
///
/// We use the `target.kind` as documented here. Note that `kind` for library will
/// match the `crate-type`, despite them not being explicitly listed in the documentation:
/// <https://docs.rs/cargo_metadata/0.15.0/cargo_metadata/struct.Target.html#structfield.kind>
///
/// The documentation for `crate-type` explicitly states that the only time `kind` and
/// `crate-type` differs is for examples.
/// <https://docs.rs/cargo_metadata/0.15.0/cargo_metadata/struct.Target.html#structfield.crate_types>
fn package_targets(args: &KaniArgs, package: &Package) -> Vec<VerificationTarget> {
    let mut ignored_tests = vec![];
    let mut ignored_unsupported = vec![];
    let mut verification_targets = vec![];
    for target in &package.targets {
        debug!(name=?package.name, target=?target.name, kind=?target.kind, crate_type=?target
                .crate_types,
                "package_targets");
        let (mut supported_lib, mut unsupported_lib) = (false, false);
        for kind in &target.kind {
            match kind.as_str() {
                CRATE_TYPE_BIN => {
                    // Binary targets.
                    verification_targets.push(VerificationTarget::Bin(target.name.clone()));
                }
                CRATE_TYPE_LIB | CRATE_TYPE_RLIB | CRATE_TYPE_CDYLIB | CRATE_TYPE_DYLIB
                | CRATE_TYPE_STATICLIB => {
                    supported_lib = true;
                }
                CRATE_TYPE_PROC_MACRO => {
                    unsupported_lib = true;
                    ignored_unsupported.push(target.name.as_str());
                }
                CRATE_TYPE_TEST => {
                    // Test target.
                    if args.tests {
                        verification_targets.push(VerificationTarget::Test(target.name.clone()));
                    } else {
                        ignored_tests.push(target.name.as_str());
                    }
                }
                _ => {
                    ignored_unsupported.push(target.name.as_str());
                }
            }
        }
        match (supported_lib, unsupported_lib) {
            (true, true) => println!(
                "warning: Skipped verification of `{}` due to unsupported crate-type: \
                        `proc-macro`.",
                target.name,
            ),
            (true, false) => verification_targets.push(VerificationTarget::Lib),
            (_, _) => {}
        }
    }

    if args.verbose {
        // Print targets that were skipped only on verbose mode.
        if !ignored_tests.is_empty() {
            println!("Skipped the following test targets: '{}'.", ignored_tests.join("', '"));
            println!("    -> Use '--tests' to verify harnesses inside a 'test' crate.");
        }
        if !ignored_unsupported.is_empty() {
            println!(
                "Skipped the following unsupported targets: '{}'.",
                ignored_unsupported.join("', '")
            );
        }
    }
    verification_targets
}
