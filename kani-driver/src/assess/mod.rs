// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use anyhow::Result;

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

    let crate_count = project.metadata.len();

    // An interesting thing to print here would be "number of crates without any warnings"
    // however this will have to wait until a refactoring of how we aggregate metadata
    // from multiple crates together here.
    // tracking for that: https://github.com/model-checking/kani/issues/1758
    println!("Analyzed {crate_count} crates");

    let metadata = merge_kani_metadata(project.metadata.clone());
    if !metadata.unsupported_features.is_empty() {
        println!("{}", table_unsupported_features::build(&metadata));
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
