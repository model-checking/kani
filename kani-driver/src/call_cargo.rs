// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use crate::args::KaniArgs;
use crate::session::KaniSession;
use anyhow::{Context, Result};
use cargo_metadata::{Metadata, MetadataCommand};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

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
        let metadata = MetadataCommand::new().exec().expect("Failed to get cargo metadata.");
        let target_dir = self
            .args
            .target_dir
            .as_ref()
            .unwrap_or(&metadata.target_directory.clone().into())
            .clone();
        let outdir = target_dir.join(build_target).join("debug/deps");

        let mut kani_args = self.kani_specific_flags();
        let rustc_args = self.kani_rustc_flags();

        let mut cargo_args: Vec<OsString> = vec!["rustc".into()];
        if self.args.tests {
            cargo_args.push("--tests".into());
        }

        if self.args.all_features {
            cargo_args.push("--all-features".into());
        }

        cargo_args.push("--target".into());
        cargo_args.push(build_target.into());

        cargo_args.push("--target-dir".into());
        cargo_args.push(target_dir.into());

        if self.args.verbose {
            cargo_args.push("-v".into());
        }

        // Arguments that will only be passed to the target package.
        let mut pkg_args: Vec<OsString> = vec![];
        if self.args.mir_linker {
            // Only provide reachability flag to the target package.
            pkg_args.push("--".into());
            pkg_args.push("--reachability=harnesses".into());
        } else {
            // Pass legacy reachability to the target package and its dependencies.
            kani_args.push("--reachability=legacy".into());
        }

        // Only joing them at the end. All kani flags must come first.
        kani_args.extend_from_slice(&rustc_args);

        let members = project_members(&self.args, &metadata);
        for member in members {
            let mut cmd = Command::new("cargo");
            cmd.args(&cargo_args)
                .args(vec!["-p", &member])
                .args(&pkg_args)
                .env("RUSTC", &self.kani_compiler)
                .env("RUSTFLAGS", "--kani-flags")
                .env("KANIFLAGS", &crate::util::join_osstring(&kani_args, " "));

            self.run_terminal(cmd)?;
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
fn project_members(args: &KaniArgs, metadata: &Metadata) -> Vec<String> {
    if !args.package.is_empty() {
        args.package.clone()
    } else {
        match (args.workspace, metadata.root_package()) {
            (true, _) | (_, None) => {
                metadata.workspace_members.iter().map(|id| metadata[id].name.clone()).collect()
            }
            (_, Some(root_pkg)) => vec![root_pkg.name.clone()],
        }
    }
}
