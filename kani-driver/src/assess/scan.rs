// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::collections::HashSet;
use std::path::Path;
use std::path::PathBuf;
use std::time::Instant;

use anyhow::Result;
use cargo_metadata::Package;

use crate::session::setup_cargo_command;
use crate::session::KaniSession;

use super::metadata::AssessMetadata;
use super::metadata::{aggregate_metadata, read_metadata};
use crate::args::ScanArgs;

/// `cargo kani assess scan` is not a normal invocation of `cargo kani`: we don't directly build anything.
/// Instead we perform a scan of the local directory for all cargo projects, and run assess on each of those.
/// Then we aggregate the results.
///
/// This is actually similar to something cargo does when it's trying to find a package in a git repo
/// (e.g. from `cargo install --git`) so we draw inspiration from `cargo::ops::cargo_read_manifest::read_packages`.
///
/// A simplified version of this algorithm looks like:
/// 1. Walk directory trees, don't descend into `target` and also stop when finding a `Cargo.toml`
/// 2. Examine each `Cargo.toml` (potentially a workspace of multiple packages)
/// 3. Aggregate all of those together.
pub(crate) fn assess_scan_main(session: KaniSession, args: &ScanArgs) -> Result<()> {
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
    let package_filter = {
        if let Some(path) = &args.filter_packages_file {
            let file = std::fs::File::open(path)?;
            let reader = std::io::BufReader::new(file);
            use std::io::BufRead;
            let set: HashSet<String> = reader.lines().map(|x| x.expect("text")).collect();
            Some(set)
        } else {
            None
        }
    };

    for project in projects {
        println!("Found {}: {}", project.name, project.manifest_path);
    }

    let overall_start_time = Instant::now();

    let mut failed_packages = Vec::new();
    let mut success_metas = Vec::new();
    for workspace in &project_metadata {
        let workspace_root = workspace.workspace_root.as_std_path();
        for package in workspace.workspace_packages() {
            if let Some(filter) = &package_filter {
                if !filter.contains(&package.name) {
                    println!("Skipping filtered-out package {}", package.name);
                    continue;
                }
            }
            // This is a hack. Some repos contains workspaces with "examples" (not actually cargo examples, but
            // full packages as workspace members) that are named after other crates.
            // It's not fully clear what approach we should take to fix this.
            // For the moment, we try to filter out "example" packages through this hack.
            // Current known instances: `syn` contains a package named `lazy_static`.
            // (syn/examples/lazy-static/lazy-static/Cargo.toml)
            if package.manifest_path.components().any(|x| x.as_str() == "examples") {
                println!(
                    "Warning: Skipping (by heuristic) {} in {}",
                    package.name, workspace.workspace_root
                );
                continue;
            }
            let package_start_time = Instant::now();
            let name = &package.name;
            let manifest = package.manifest_path.as_std_path();

            // We could reasonably choose to write these under 'target/kani', but can't because that gets deleted when 'cargo kani' runs.
            // We could reasonably choose to write these under 'target', but the present way I experiment with 'assess'
            // deletes target directories to save disk space in large runs. (Builds are big!)
            // So at present we choose to put them next to the workspace root Cargo.toml.
            let outfile = workspace_root.join(format!("{name}.kani-assess-metadata.json"));
            let logfile = workspace_root.join(format!("{name}.kani-assess.log"));

            let result = if args.existing_only {
                Ok(())
            } else {
                invoke_assess(&session, name, manifest, &outfile, &logfile)
            };

            let meta = read_metadata(&outfile);
            if let Ok(meta) = meta {
                if meta.error.is_some() {
                    println!("Failed: {name}");
                    // Some execution error that we have collected.
                    failed_packages.push((package, Some(meta)))
                } else {
                    success_metas.push(meta);
                }
            } else {
                println!("Failed: {name}");
                failed_packages.push((
                    package,
                    result.err().map(|err| AssessMetadata::from_error(err.as_ref())),
                ));
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
    print_failures(failed_packages);
    println!("{}", results.unsupported_features.render());

    if !session.args.only_codegen {
        println!("{}", results.failure_reasons.render());
        println!("{}", results.promising_tests.render());
    }

    if let Some(path) = &args.emit_metadata {
        let out_file = std::fs::File::create(path)?;
        let writer = std::io::BufWriter::new(out_file);
        // use pretty for now to keep things readable and debuggable, but this should change eventually
        serde_json::to_writer_pretty(writer, &results)?;
    }

    Ok(())
}

/// Calls `cargo kani assess` on a single package.
fn invoke_assess(
    session: &KaniSession,
    package: &str,
    manifest: &Path,
    outfile: &Path,
    logfile: &Path,
) -> Result<()> {
    let dir = manifest.parent().expect("file not in a directory?");
    let log = std::fs::File::create(logfile)?;

    let mut cmd = setup_cargo_command()?;
    cmd.arg("kani");
    // Use of options before 'assess' subcommand is a hack, these should be factored out.
    // TODO: --only-codegen should be outright an option to assess. (perhaps tests too?)
    if session.args.only_codegen {
        cmd.arg("--only-codegen");
    }
    // TODO: -p likewise, probably fixed with a "CargoArgs" refactoring
    // Additionally, this should be `--manifest-path` but `cargo kani` doesn't support that yet.
    cmd.arg("-p").arg(package);
    cmd.arg("--enable-unstable"); // This has to be after `-p` due to an argument parsing bug in kani-driver
    cmd.args(["assess", "--emit-metadata"])
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
    let Ok(entries) = std::fs::read_dir(path) else {
        return;
    };
    for entry in entries {
        let Ok(entry) = entry else {
            continue;
        };
        let Ok(typ) = entry.file_type() else {
            continue;
        };
        // symlinks are not `is_dir()`
        if typ.is_dir() {
            scan_cargo_projects(entry.path(), accumulator)
        }
    }
}

/// Print failures if any happened.
fn print_failures(mut failures: Vec<(&Package, Option<AssessMetadata>)>) {
    if !failures.is_empty() {
        println!("Failed to assess packages:");
        let unknown = "Unknown".to_string();
        failures.sort_by_key(|(pkg, _)| &pkg.name);
        for (pkg, meta) in failures {
            println!(
                "  - `{}`: {}",
                pkg.name,
                meta.as_ref().map_or(&unknown, |md| {
                    md.error.as_ref().map_or(&unknown, |error| &error.msg)
                }),
            );
        }
        println!();
    }
}
