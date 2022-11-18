// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::{collections::HashMap, path::PathBuf};

use anyhow::Result;
use comfy_table::Table;
use kani_metadata::KaniMetadata;

use crate::harness_runner::HarnessResult;
use crate::session::KaniSession;

/// Set some defaults for how we format tables
fn assess_table_new() -> Table {
    use comfy_table::*;

    let mut table = Table::new();
    table.set_content_arrangement(ContentArrangement::Dynamic);
    table
        .load_preset(comfy_table::presets::NOTHING)
        .set_style(TableComponent::BottomBorder, '=')
        .set_style(TableComponent::BottomBorderIntersections, '=')
        .set_style(TableComponent::TopBorder, '=')
        .set_style(TableComponent::TopBorderIntersections, '=')
        .set_style(TableComponent::HeaderLines, '-')
        .set_style(TableComponent::MiddleHeaderIntersections, '+')
        .set_style(TableComponent::VerticalLines, '|');
    table
}

/// Internal data type for constructing the unsupported features table
#[derive(Default)]
struct UnsupportedFeaturesTableData {
    crates_impacted: usize,
    instances_of_use: usize,
}

/// Reports unsupported features, in descending order of number of crates impacted.
///
/// The feature names come directly from the `operation_name` listed in `codegen_unimplemented`
///
/// For example:
///
/// ```text
/// ===================================================
///  Unsupported feature        |   Crates | Instances
///                             | impacted |    of use
/// ----------------------------+----------+-----------
///  'simd_or' intrinsic        |        4 |         5
///  try                        |        2 |        17
///  drop_in_place              |        2 |         2
/// ===================================================
/// ```
fn build_unsupported_features_table(metadata: &KaniMetadata) -> Table {
    // Map "unsupported feature name" -> (crates impacted, instance of use)
    let mut counts: HashMap<String, UnsupportedFeaturesTableData> = HashMap::new();

    for item in &metadata.unsupported_features {
        // key is unsupported feature name
        let mut key = item.feature.clone();
        // There are several "feature for <instance of use>" unsupported features.
        // We aggregate those here by reducing it to just "feature".
        // We should replace this with an enum: https://github.com/model-checking/kani/issues/1765
        if let Some((prefix, _)) = key.split_once(" for ") {
            key = prefix.to_string();
        }
        let entry = counts.entry(key).or_default();
        entry.crates_impacted += 1;
        entry.instances_of_use += item.locations.len();
    }

    // Sort descending by number of crates impacted by this missing feature
    let mut sorted_counts: Vec<(String, UnsupportedFeaturesTableData)> =
        counts.into_iter().collect();
    sorted_counts.sort_by_key(|(_, data)| usize::MAX - data.crates_impacted);

    {
        use comfy_table::*;

        let mut table = assess_table_new();
        table.set_header(vec!["Unsupported feature", "Crates\nimpacted", "Instances\nof use"]);
        table.column_mut(0).unwrap().set_cell_alignment(CellAlignment::Left);
        table
            .column_mut(0)
            .unwrap()
            .set_constraint(ColumnConstraint::UpperBoundary(Width::Fixed(80)));
        table.column_mut(1).unwrap().set_cell_alignment(CellAlignment::Right);
        table.column_mut(2).unwrap().set_cell_alignment(CellAlignment::Right);

        for (feature, data) in sorted_counts {
            table.add_row(vec![
                feature,
                data.crates_impacted.to_string(),
                data.instances_of_use.to_string(),
            ]);
        }

        table
    }
}

/// Reports the most common "failure reasons" for tests run by assess.
///
/// The reasons are presently just a combination of "property classes"
/// from failed CBMC properties. This is only really meaningful to us,
/// and could use significant improvement for customers. In fact,
/// this particular data set might *only* be interesting to us as
/// developers of 'assess', and not customers, once we get fewer failures
/// and the heuristics for "promising tests" are improved.
///
/// Example:
///
/// ```text
/// ================================================
///  Reason for failure           | Number of tests
/// ------------------------------+-----------------
///  unwind                       |              61
///  success                      |               6
///  assertion                    |               4
///  assertion + overflow         |               2
/// ================================================
/// ```
fn build_failure_reasons_table(results: &[HarnessResult]) -> Table {
    // Map "Reason for failure" -> (Number of tests)
    let mut counts: HashMap<String, usize> = HashMap::new();

    for r in results {
        let failures = r.result.failed_properties();
        let classification = if failures.is_empty() {
            "success".to_string()
        } else {
            let mut classes: Vec<_> = failures.iter().map(|p| p.property_class()).collect();
            classes.sort();
            classes.dedup();
            classes.join(" + ")
        };
        let entry = counts.entry(classification).or_default();
        *entry += 1;
    }

    // Sort descending by number of failures for this reason
    let mut sorted_counts: Vec<(String, usize)> = counts.into_iter().collect();
    sorted_counts.sort_by_key(|(_, count)| usize::MAX - count);

    {
        use comfy_table::*;

        let mut table = assess_table_new();
        table.set_header(vec!["Reason for failure", "Number of tests"]);
        table.column_mut(0).unwrap().set_cell_alignment(CellAlignment::Left);
        table
            .column_mut(0)
            .unwrap()
            .set_constraint(ColumnConstraint::UpperBoundary(Width::Fixed(80)));
        table.column_mut(1).unwrap().set_cell_alignment(CellAlignment::Right);

        for (class, count) in sorted_counts {
            table.add_row(vec![class, count.to_string()]);
        }

        table
    }
}

/// Reports the "test harnesses" most likely to be easily turned into proof harnesses.
///
/// This presently is very naive and just reports successful harnesses, however
/// there is significant potential here to make use of improved heuristics,
/// and to find a way to *sort* these harnesses.
///
/// Example:
/// ```text
/// =============================================================================
///  Candidate for proof harness                           | Location
/// -------------------------------------------------------+---------------------
///  float::tests::f64_edge_cases                          | src/float.rs:226
///  float::tests::f32_edge_cases                          | src/float.rs:184
///  integer::tests::test_integers                         | src/integer.rs:171
///  other::tests::test_misc                               | src/other.rs:284
/// =============================================================================
/// ```
fn build_promising_tests_table(results: &[HarnessResult]) -> Table {
    {
        use comfy_table::*;

        let mut table = assess_table_new();
        table.set_header(vec!["Candidate for proof harness", "Location"]);
        table.column_mut(0).unwrap().set_cell_alignment(CellAlignment::Left);
        table
            .column_mut(0)
            .unwrap()
            .set_constraint(ColumnConstraint::UpperBoundary(Width::Fixed(80)));
        table.column_mut(1).unwrap().set_cell_alignment(CellAlignment::Left);

        for r in results {
            // For now we're just reporting "successful" harnesses as candidates.
            // In the future this heuristic should be expanded. More data is required to do this, however.
            if r.result.failed_properties().is_empty() {
                // The functions we extract are actually the closures inside the test harness macro expansion
                // Strip that closure suffix, so we have better names:
                let name = r.harness.pretty_name.trim_end_matches("::{closure#0}").to_string();
                // Location in a format "clickable" in e.g. IDE terminals
                let location =
                    format!("{}:{}", r.harness.original_file, r.harness.original_start_line);

                table.add_row(vec![name, location]);
            }
        }

        table
    }
}

pub(crate) fn cargokani_assess_main(mut ctx: KaniSession) -> Result<()> {
    // fix some settings
    ctx.args.unwind = Some(1);
    ctx.args.tests = true;
    ctx.args.output_format = crate::args::OutputFormat::Terse;
    ctx.codegen_tests = true;
    if ctx.args.jobs.is_none() {
        // assess will default to fully parallel instead of single-threaded.
        // can be overridden with e.g. `cargo kani --enable-unstable -j 8 assess`
        ctx.args.jobs = Some(None); // -j, num_cpu
    }

    let outputs = ctx.cargo_build()?;
    let metadata = ctx.collect_kani_metadata(&outputs.metadata)?;

    let crate_count = outputs.metadata.len();

    // An interesting thing to print here would be "number of crates without any warnings"
    // however this will have to wait until a refactoring of how we aggregate metadata
    // from multiple crates together here.
    // tracking for that: https://github.com/model-checking/kani/issues/1758
    println!("Analyzed {crate_count} crates");

    if !metadata.unsupported_features.is_empty() {
        println!("{}", build_unsupported_features_table(&metadata));
    } else {
        println!("No crates contained Rust features unsupported by Kani");
    }

    // The section is a "copy and paste" from cargo kani.
    // We could start thinking about abtracting this stuff out into a shared function...
    let mut goto_objs: Vec<PathBuf> = Vec::new();
    for symtab in &outputs.symtabs {
        let goto_obj_filename = symtab.with_extension("out");
        goto_objs.push(goto_obj_filename);
    }

    if ctx.args.only_codegen {
        return Ok(());
    }

    let linked_obj = outputs.outdir.join("cbmc-linked.out");
    ctx.link_goto_binary(&goto_objs, &linked_obj)?;
    if let Some(restrictions) = outputs.restrictions {
        ctx.apply_vtable_restrictions(&linked_obj, &restrictions)?;
    }

    // Done with the 'cargo-kani' part, now we're going to run *test* harnesses instead of proof:

    let harnesses = metadata.test_harnesses;
    let report_base = ctx.args.target_dir.clone().unwrap_or(PathBuf::from("target"));

    let runner = crate::harness_runner::HarnessRunner {
        sess: &ctx,
        linked_obj: &linked_obj,
        report_base: &report_base,
        symtabs: &outputs.symtabs,
        retain_specialized_harnesses: false,
    };

    let results = runner.check_all_harnesses(&harnesses)?;

    // two tables we want to print:
    // 1. "Reason for failure" will count reasons why harnesses did not succeed
    //    e.g.  successs   6
    //          unwind     234
    println!("{}", build_failure_reasons_table(&results));

    // TODO: Should add another interesting table: Count the actually hit constructs (e.g. 'try', 'InlineAsm', etc)
    // The above table will just say "unsupported_construct   6" without telling us which constructs.
    // Tracking issue: https://github.com/model-checking/kani/issues/1819

    // 2. "Test cases that might be good proof harness starting points"
    //    e.g.  All Successes and maybe Assertions?
    println!("{}", build_promising_tests_table(&results));

    Ok(())
}
