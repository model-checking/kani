// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use crate::args::KaniArgs;
use crate::session::{KaniSession, ReachabilityMode};
use anyhow::{bail, Context, Result};
use cargo_metadata::{Metadata, MetadataCommand, Package};
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::{debug, trace};

/// The outputs of kani-compiler being invoked via cargo on a project.
pub struct CargoOutputs {
    /// The directory where compiler outputs should be directed.
    /// Usually 'target/BUILD_TRIPLE/debug/deps/'
    pub outdir: PathBuf,
    /// The collection of *.symtab.json files written.
    pub symtabs: Vec<PathBuf>,
    /// The location of vtable restrictions files (a directory of *.restrictions.json)
    pub restrictions: Option<PathBuf>,
    /// The kani-metadata.json files written by kani-compiler.
    pub metadata: Vec<PathBuf>,
}

impl KaniSession {
    /// Calls `cargo_build` to generate `*.symtab.json` files in `target_dir`
    pub fn cargo_build(&self) -> Result<CargoOutputs> {
        let build_target = env!("TARGET"); // see build.rs
        let metadata = MetadataCommand::new().exec().context("Failed to get cargo metadata.")?;
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

        let mut kani_args = self.kani_specific_flags();
        let rustc_args = self.kani_rustc_flags();

        let mut cargo_args: Vec<OsString> = vec!["rustc".into()];
        if self.args.all_features {
            cargo_args.push("--all-features".into());
        }

        cargo_args.push("--target".into());
        cargo_args.push(build_target.into());

        cargo_args.push("--target-dir".into());
        cargo_args.push(target_dir.into());

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
        let mut pkg_args: Vec<OsString> = vec![];
        match self.reachability_mode() {
            ReachabilityMode::Legacy => {
                // For this mode, we change `kani_args` not `pkg_args`
                kani_args.push("--reachability=legacy".into());
            }
            ReachabilityMode::ProofHarnesses => {
                pkg_args.extend(["--".into(), "--reachability=harnesses".into()]);
            }
            ReachabilityMode::AllPubFns => {
                pkg_args.extend(["--".into(), "--reachability=pub_fns".into()]);
            }
        }

        // Only joing them at the end. All kani flags must come first.
        kani_args.extend_from_slice(&rustc_args);

        let mut any_target = false;
        let packages = packages_to_verify(&self.args, &metadata);
        for package in packages {
            for target in package_targets(&self.args, package) {
                let mut cmd = Command::new("cargo");
                cmd.args(&cargo_args)
                    .args(vec!["-p", &package.name])
                    .args(&target.to_args())
                    .args(&pkg_args)
                    .env("RUSTC", &self.kani_compiler)
                    .env("RUSTFLAGS", "--kani-flags")
                    .env("KANIFLAGS", &crate::util::join_osstring(&kani_args, " "));

                self.run_terminal(cmd)?;
                any_target = true;
            }
        }

        if !any_target {
            bail!("No supported targets were found.");
        }
        if self.args.dry_run {
            // mock an answer: mostly the same except we don't/can't expand the globs
            return Ok(CargoOutputs {
                outdir: outdir.clone(),
                symtabs: vec![outdir.join("*.symtab.json")],
                metadata: vec![outdir.join("*.kani-metadata.json")],
                restrictions: self.args.restrict_vtable().then_some(outdir),
            });
        }

        Ok(CargoOutputs {
            outdir: outdir.clone(),
            symtabs: glob(&outdir.join("*.symtab.json"))?,
            metadata: glob(&outdir.join("*.kani-metadata.json"))?,
            restrictions: self.args.restrict_vtable().then_some(outdir),
        })
    }
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
/// If --package <pkg> is given, return the list of packages selected.
/// If --workspace is given, return the list of workspace members.
/// If no argument provided, return the root package if there's one or all members.
///   - I.e.: Do whatever cargo does when there's no default_members.
///   - This is because `default_members` is not available in cargo metadata.
///     See <https://github.com/rust-lang/cargo/issues/8033>.
fn packages_to_verify<'a, 'b>(args: &'a KaniArgs, metadata: &'b Metadata) -> Vec<&'b Package> {
    debug!(package_selection=?args.package, workspace=args.workspace, "packages_to_verify args");
    let packages = if !args.package.is_empty() {
        args.package
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
        match (args.workspace, metadata.root_package()) {
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
    let mut ignored_libs = vec![];
    let mut ignored_tests = vec![];
    let mut ignored_unsupported = vec![];
    let verification_targets = package
        .targets
        .iter()
        .filter_map(|target| {
            debug!(name=?package.name, target=?target.name, kind=?target.kind, crate_type=?target
                .crate_types,
                "package_targets");
            if target.kind.contains(&String::from("bin")) {
                // Binary targets.
                Some(VerificationTarget::Bin(target.name.clone()))
            } else if target.kind.contains(&String::from("lib"))
                || target.kind.contains(&String::from("rlib"))
            {
                // Lib targets.
                if target.kind.iter().any(|kind| {
                    matches!(kind.as_str(), "cdylib" | "dylib" | "staticlib" | "proc-macro")
                }) {
                    ignored_libs.push(target.name.as_str());
                    None
                } else {
                    Some(VerificationTarget::Lib)
                }
            } else if target.kind.contains(&String::from("test")) {
                // Test target.
                if args.tests {
                    Some(VerificationTarget::Test(target.name.clone()))
                } else {
                    ignored_tests.push(target.name.as_str());
                    None
                }
            } else {
                ignored_unsupported.push(target.name.as_str());
                None
            }
        })
        .collect();

    if !ignored_libs.is_empty() {
        // Print a warning for a lib that had at least one supported crate-types and one
        // unsupported one.
        println!(
            "warning: Skipped verification of the following targets: '{}'",
            ignored_libs.join("', '")
        );
        println!(
            "    -> The targets above contained at least of one unsupported library type. \
        Supported types are 'lib' and 'rlib'."
        );
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
