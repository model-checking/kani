// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::{
    path::{Path, PathBuf},
    process::Command,
    time::Instant,
};

use anyhow::Result;

use crate::session::KaniSession;

use super::{
    args::ScanArgs,
    metadata::{aggregate_metadata, read_metadata},
};

/// `cargo kani assess scan` is not a normal invocation of `cargo kani`: we don't directly build anything.
/// Instead we perform a scan of the local directory for all cargo projects, and run assess on each of those.
/// Then we aggregate the results.
///
/// This is actually similar to something cargo does when it's trying to find a package in a git repo
/// (e.g. from `cargo install --git`) so we draw inspiration from `cargo::ops::cargo_read_manifest::read_packages`.
///
/// A simplified version of this algorithms looks like:
/// 1. Walk directory trees, don't descend into `target` and also stop when finding a `Cargo.toml`
/// 2. Examine each `Cargo.toml` (potentially a workspace of multiple packages)
/// 3. Aggregate all of those together.
pub(crate) fn main(session: KaniSession, args: &ScanArgs) -> Result<()> {
    let cargo_toml_files = {
        let mut files = Vec::new();
        scan_cargo_projects(PathBuf::from("."), &mut files);
        files
    };
    let build_target = env!("TARGET"); // see build.rs
    let project_metadata = {
        // Things kind blow up trying to handle errors here, so write it iteratively instead of using map
        let mut metas = Vec::with_capacity(cargo_toml_files.len());
        for file in cargo_toml_files {
            let meta = cargo_metadata::MetadataCommand::new()
                .features(cargo_metadata::CargoOpt::AllFeatures)
                .no_deps()
                .manifest_path(file)
                .other_options(vec![String::from("--filter-platform"), build_target.to_owned()])
                .exec()?;
            metas.push(meta);
        }
        metas
    };
    let projects: Vec<_> = project_metadata.iter().flat_map(|x| &x.packages).collect();

    for project in projects {
        println!("Found {}", project.manifest_path);
    }

    let overall_start_time = Instant::now();

    let mut failed_packages = Vec::new();
    let mut success_metas = Vec::new();
    for workspace in &project_metadata {
        let workspace_root = workspace.workspace_root.as_std_path();
        for package in &workspace.packages {
            let package_start_time = Instant::now();
            let name = &package.name;
            let manifest = package.manifest_path.as_std_path();

            // We could reasonably choose to write these under 'target/kani', but can't because that gets deleted when 'cargo kani' runs.
            // We could reasonably choose to write these under 'target', but the present way I experiment with 'assess'
            // deletes target directories to save disk space in large runs. (Builds are big!)
            // So at present we choose to put the next to the workspace root Cargo.toml.
            let outfile = workspace_root.join(format!("{name}.kani-assess-metadata.json"));
            let logfile = workspace_root.join(format!("{name}.kani-assess.log"));

            let result = if args.existing_only {
                Ok(())
            } else {
                invoke_assess(&session, name, manifest, &outfile, &logfile)
            };

            if result.is_err() {
                println!("Failed: {name}");
                failed_packages.push(package);
            } else {
                let meta = read_metadata(&outfile);
                if let Ok(meta) = meta {
                    success_metas.push(meta);
                } else {
                    failed_packages.push(package);
                }
            }
            //TODO: cargo clean?
            println!(
                "Package {} analysis time: {}",
                name,
                package_start_time.elapsed().as_secs_f32()
            );
        }
    }

    println!("Overall analysis time: {}s", overall_start_time.elapsed().as_secs_f32());
    println!(
        "Assessed {} successfully, with {} failures.",
        success_metas.len(),
        failed_packages.len()
    );
    let results = aggregate_metadata(success_metas);
    println!("{}", results.unsupported_features.render());
    println!("{}", results.failure_reasons.render());
    println!("{}", results.promising_tests.render());

    Ok(())
}

fn invoke_assess(
    session: &KaniSession,
    package: &str,
    manifest: &Path,
    outfile: &Path,
    logfile: &Path,
) -> Result<()> {
    let dir = manifest.parent().expect("file not in a directory?");
    let log = std::fs::File::create(logfile)?;
    let mut cmd = Command::new("cargo");
    cmd.arg("kani");
    // Use of options before 'assess' subcommand is a hack, these should be factored out.
    // TODO: --only-codegen should be outright an option to assess. (perhaps tests too?)
    if session.args.only_codegen {
        cmd.arg("--only-codegen");
    }
    // TODO: -p likewise, probably fixed with a "CargoArgs" refactoring
    cmd.arg("-p").arg(package);
    cmd.arg("--enable-unstable"); // This has to be after `-p` due to an argument parsing bug in kani-driver
    cmd.args(&["assess", "--emit-metadata"])
        .arg(outfile)
        .current_dir(dir)
        .stdout(log.try_clone()?)
        .stderr(log)
        .env("RUST_BACKTRACE", "1");
    println!("Running {}", crate::util::render_command(&cmd).to_string_lossy());
    anyhow::ensure!(cmd.status()?.success());
    Ok(())
}

/// A short-circuiting directory walk for finding Cargo.toml files.
/// Sadly, strangely difficult to do with high-level libraries, so implement it ourselves
fn scan_cargo_projects(path: PathBuf, accumulator: &mut Vec<PathBuf>) {
    debug_assert!(path.is_dir());
    let cargo_file = path.join("Cargo.toml");
    if cargo_file.exists() {
        accumulator.push(cargo_file);
        // short-circuit and stop descending
        return;
    }
    // Errors are silently skipped entirely here
    let Ok(entries) = std::fs::read_dir(path) else { return; };
    for entry in entries {
        let Ok(entry) = entry else { continue; };
        let Ok(typ) = entry.file_type() else { continue; };
        if typ.is_dir() {
            scan_cargo_projects(entry.path(), accumulator)
        }
    }
}
