// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use self::metadata::{write_metadata, AssessMetadata};
use anyhow::{bail, Result};
use kani_metadata::KaniMetadata;

use crate::assess::table_builder::TableBuilder;
use crate::metadata::merge_kani_metadata;
use crate::project;
use crate::session::KaniSession;

pub use crate::args::{AssessArgs, AssessSubcommand};

mod metadata;
mod scan;
mod table_builder;
mod table_failure_reasons;
mod table_promising_tests;
mod table_unsupported_features;

/// `cargo kani assess` main entry point.
///
/// See <https://model-checking.github.io/kani/dev-assess.html>
pub(crate) fn run_assess(session: KaniSession, args: AssessArgs) -> Result<()> {
    if let Some(AssessSubcommand::Scan(args)) = &args.command {
        return scan::assess_scan_main(session, args);
    }

    let result = assess_project(session);
    match result {
        Ok(metadata) => write_metadata(&args, metadata),
        Err(err) => {
            let metadata = AssessMetadata::from_error(err.as_ref());
            write_metadata(&args, metadata)?;
            Err(err.context("Failed to assess project"))
        }
    }
}

fn assess_project(mut session: KaniSession) -> Result<AssessMetadata> {
    // Fix (as in "make unchanging/unchangable") some settings.
    // This is a temporary hack to make things work, until we get around to refactoring how arguments
    // work generally in kani-driver. These arguments, for instance, are all prepended to the subcommand,
    // which is not a nice way of taking arguments.
    session.args.unwind = Some(session.args.default_unwind.unwrap_or(1));
    session.args.tests = true;
    session.args.output_format = crate::args::OutputFormat::Terse;
    session.codegen_tests = true;
    if session.args.jobs.is_none() {
        // assess will default to fully parallel instead of single-threaded.
        // can be overridden with e.g. `cargo kani --enable-unstable -j 8 assess`
        session.args.jobs = Some(None); // -j, num_cpu
    }

    let project = project::cargo_project(&session, true)?;
    let cargo_metadata = project.cargo_metadata.as_ref().expect("built with cargo");

    let packages_metadata =
        reconstruct_metadata_structure(&session, cargo_metadata, &project.metadata)?;

    // We don't really have a list of crates that went into building our various targets,
    // so we can't easily count them.

    // It would also be interesting to classify them by whether they build without warnings or not.
    // Tracking for the latter: https://github.com/model-checking/kani/issues/1758

    let build_fail = project.failed_targets.as_ref().unwrap();
    match (build_fail.len(), packages_metadata.len()) {
        (0, 0) => println!("No relevant data was found."),
        (0, succeeded) => println!("Analyzed {succeeded} packages"),
        (_failed, 0) => bail!("Failed to build all targets"),
        (failed, succeeded) => {
            println!("Analyzed {succeeded} packages. Failed to build {failed} targets",)
        }
    }

    let metadata = merge_kani_metadata(packages_metadata.clone());
    let unsupported_features = table_unsupported_features::build(&packages_metadata);
    if !metadata.unsupported_features.is_empty() {
        println!("{}", unsupported_features.render());
    } else {
        println!("No crates contained Rust features unsupported by Kani");
    }

    if session.args.only_codegen {
        return Ok(AssessMetadata::new(
            unsupported_features,
            TableBuilder::new(),
            TableBuilder::new(),
        ));
    }

    // Done with the 'cargo-kani' part, now we're going to run *test* harnesses instead of proof:
    let harnesses = Vec::from_iter(metadata.test_harnesses.iter());
    let runner = crate::harness_runner::HarnessRunner { sess: &session, project: &project };

    let results = runner.check_all_harnesses(&harnesses)?;

    // two tables we want to print:
    // 1. "Reason for failure" will count reasons why harnesses did not succeed
    //    e.g.  successs   6
    //          unwind     234
    let failure_reasons = table_failure_reasons::build(&results);
    println!("{}", failure_reasons.render());

    // TODO: Should add another interesting table: Count the actually hit constructs (e.g. 'try', 'InlineAsm', etc)
    // The above table will just say "unsupported_construct   6" without telling us which constructs.
    // Tracking issue: https://github.com/model-checking/kani/issues/1819

    // 2. "Test cases that might be good proof harness starting points"
    //    e.g.  All Successes and maybe Assertions?
    let promising_tests = table_promising_tests::build(&results);
    println!("{}", promising_tests.render());

    Ok(AssessMetadata::new(unsupported_features, failure_reasons, promising_tests))
}

/// Merges a collection of Kani metadata by figuring out which package each belongs to, from cargo metadata.
///
/// Initially, `kani_metadata` is a kani metadata structure for each _target_ of every package.
/// This function works by collecting each target and merging them into a package-wide metadata.
///
/// This function, properly speaking, should not exist. We should have this information already from `Project`.
/// This should function should be removable when we fix how driver handles metadata:
/// <https://github.com/model-checking/kani/issues/1758>
fn reconstruct_metadata_structure(
    session: &KaniSession,
    cargo_metadata: &cargo_metadata::Metadata,
    kani_metadata: &[KaniMetadata],
) -> Result<Vec<KaniMetadata>> {
    let mut remaining_metas = kani_metadata.to_owned();
    let mut package_metas = vec![];
    for package in cargo_metadata.workspace_packages() {
        if !session.args.cargo.package.is_empty() {
            // If a specific package (set) is requested, skip all other packages.
            // This is a necessary workaround because we're reconstructing which metas go to which packages
            // based on the "crate name" given to the target, and the same workspace can have two
            // packages with targets that have the same crate name.
            // This is just an inherent problem with trying to reconstruct this information, and should
            // be fixed by the issue linked in the function description.
            // The best we can do for now is ignore packages we know we didn't build, to reduce the amount
            // of confusion we might suffer here (which at least solves the problem for 'scan' which only
            // builds 1 package at a time.)
            if !session.args.cargo.package.contains(&package.name) {
                continue;
            }
        }
        let mut package_artifacts = vec![];
        for target in &package.targets {
            // cargo_metadata doesn't provide name mangling help here?
            // We need to know cargo's name changes when it's given to rustc, because kani-metadata
            // records `crate_name` as rustc sees it, but `target` is name as cargo sees it
            let target_name = target.name.replace('-', "_");
            if let Some(i) = remaining_metas.iter().position(|x| x.crate_name == target_name) {
                let md = remaining_metas.swap_remove(i);
                package_artifacts.push(md);
            } else {
                println!(
                    "Didn't find metadata for target {} (kind {:?}) in package {}",
                    target.name, target.kind, package.name
                )
            }
        }
        if !package_artifacts.is_empty() {
            let mut merged = crate::metadata::merge_kani_metadata(package_artifacts);
            merged.crate_name.clone_from(&package.name);
            package_metas.push(merged);
        }
    }
    if !remaining_metas.is_empty() {
        let remaining_names: Vec<_> = remaining_metas.into_iter().map(|x| x.crate_name).collect();
        println!("Found remaining (unused) metadata after reconstruction: {remaining_names:?}");
    }
    Ok(package_metas)
}
