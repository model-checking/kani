// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use anyhow::Result;
use kani_metadata::KaniMetadata;

use crate::metadata::merge_kani_metadata;
use crate::project;
use crate::session::KaniSession;

mod table_builder;
mod table_failure_reasons;
mod table_promising_tests;
mod table_unsupported_features;

pub(crate) fn cargokani_assess_main(mut session: KaniSession) -> Result<()> {
    // fix some settings
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
    if !metadata.unsupported_features.is_empty() {
        println!("{}", table_unsupported_features::build(&packages_metadata));
    } else {
        println!("No crates contained Rust features unsupported by Kani");
    }

    if session.args.only_codegen {
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
    println!("{}", table_failure_reasons::build(&results));

    // TODO: Should add another interesting table: Count the actually hit constructs (e.g. 'try', 'InlineAsm', etc)
    // The above table will just say "unsupported_construct   6" without telling us which constructs.
    // Tracking issue: https://github.com/model-checking/kani/issues/1819

    // 2. "Test cases that might be good proof harness starting points"
    //    e.g.  All Successes and maybe Assertions?
    println!("{}", table_promising_tests::build(&results));

    Ok(())
}

/// Merges a collection of Kani metadata by figuring out which package each belongs to, from cargo metadata.
///
/// This function, properly speaking, should not exist. We should have this information already from `Project`.
/// This should function should be removable when we fix how driver handles metadata:
/// <https://github.com/model-checking/kani/issues/1758>
fn reconstruct_metadata_structure(
    cargo_metadata: &cargo_metadata::Metadata,
    kani_metadata: &[KaniMetadata],
) -> Result<Vec<KaniMetadata>> {
    let mut search = kani_metadata.to_owned();
    let mut results = vec![];
    for package in &cargo_metadata.packages {
        let mut artifacts = vec![];
        for target in &package.targets {
            // cargo_metadata doesn't provide name mangling help here?
            // we need to know cargo's name changes when it's given to rustc
            let target_name = target.name.replace('-', "_");
            if let Some(i) = search.iter().position(|x| x.crate_name == target_name) {
                let md = search.swap_remove(i);
                artifacts.push(md);
            } else {
                println!(
                    "Didn't find metadata for target {} in package {}",
                    target.name, package.name
                )
            }
        }
        let mut merged = crate::metadata::merge_kani_metadata(artifacts);
        merged.crate_name = package.name.clone();
        results.push(merged);
    }
    if !search.is_empty() {
        let search_names: Vec<_> = search.into_iter().map(|x| x.crate_name).collect();
        println!("Found remaining (unused) metadata after reconstruction: {:?}", search_names);
    }
    Ok(results)
}
