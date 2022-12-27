// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use anyhow::Result;
use kani_metadata::KaniMetadata;

use crate::metadata::merge_kani_metadata;
use crate::project;
use crate::session::KaniSession;

pub use self::args::AssessArgs;

mod args;
mod metadata;
mod scan;
mod table_builder;
mod table_failure_reasons;
mod table_promising_tests;
mod table_unsupported_features;

pub(crate) fn cargokani_assess_main(mut session: KaniSession, args: AssessArgs) -> Result<()> {
    if let Some(args::AssessSubcommand::Scan(args)) = &args.command {
        return scan::assess_scan_main(session, args);
    }

    // fix (as in "make unchanging/unchangable") some settings
    session.args.all_features = true;
    session.args.unwind = Some(1);
    session.args.tests = true;
    session.args.output_format = crate::args::OutputFormat::Terse;
    session.codegen_tests = true;
    if session.args.jobs.is_none() {
        // assess will default to fully parallel instead of single-threaded.
        // can be overridden with e.g. `cargo kani --enable-unstable -j 8 assess`
        session.args.jobs = Some(None); // -j, num_cpu
    }

    let project = project::cargo_project(&session)?;
    let cargo_metadata = project.cargo_metadata.as_ref().expect("built with cargo");

    let packages_metadata = if project.merged_artifacts {
        // With the legacy linker we can't expect to find the metadata structure we'd expect
        // so we just use it as-is. This does mean the "package count" will be wrong, but
        // we will at least continue to see everything.
        project.metadata.clone()
    } else {
        reconstruct_metadata_structure(cargo_metadata, &project.metadata)?
    };

    // We don't really have a list of crates that went into building our various targets,
    // so we can't easily count them.

    // It would also be interesting to classify them by whether they build without warnings or not.
    // Tracking for the latter: https://github.com/model-checking/kani/issues/1758

    println!("Found {} packages", packages_metadata.len());

    let metadata = merge_kani_metadata(packages_metadata.clone());
    let unsupported_features = table_unsupported_features::build(&packages_metadata);
    if !metadata.unsupported_features.is_empty() {
        println!("{}", unsupported_features.render());
    } else {
        println!("No crates contained Rust features unsupported by Kani");
    }

    if session.args.only_codegen {
        metadata::write_partial_metadata(&args, unsupported_features)?;
        return Ok(());
    }

    // Done with the 'cargo-kani' part, now we're going to run *test* harnesses instead of proof:
    let harnesses = metadata.test_harnesses;
    let runner = crate::harness_runner::HarnessRunner { sess: &session, project };

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

    metadata::write_metadata(
        &args,
        metadata::AssessMetadata { unsupported_features, failure_reasons, promising_tests },
    )?;

    Ok(())
}

/// Merges a collection of Kani metadata by figuring out which package each belongs to, from cargo metadata.
///
/// Initially, `kani_metadata` is a kani metadata structure for each _target_ of every package.
/// This function works by collecting each target
///
/// This function, properly speaking, should not exist. We should have this information already from `Project`.
/// This should function should be removable when we fix how driver handles metadata:
/// <https://github.com/model-checking/kani/issues/1758>
fn reconstruct_metadata_structure(
    cargo_metadata: &cargo_metadata::Metadata,
    kani_metadata: &[KaniMetadata],
) -> Result<Vec<KaniMetadata>> {
    let mut remaining_metas = kani_metadata.to_owned();
    let mut package_metas = vec![];
    for package in cargo_metadata.workspace_packages() {
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
                    "Didn't find metadata for target {} in package {}",
                    target.name, package.name
                )
            }
        }
        let mut merged = crate::metadata::merge_kani_metadata(package_artifacts);
        merged.crate_name = package.name.clone();
        package_metas.push(merged);
    }
    if !remaining_metas.is_empty() {
        let remaining_names: Vec<_> = remaining_metas.into_iter().map(|x| x.crate_name).collect();
        println!("Found remaining (unused) metadata after reconstruction: {:?}", remaining_names);
    }
    Ok(package_metas)
}
