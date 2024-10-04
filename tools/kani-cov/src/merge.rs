// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This module includes the implementation of the `merge` subcommand.

use std::{
    collections::{HashMap, HashSet},
    fs::{File, OpenOptions},
    io::{BufReader, BufWriter},
    path::PathBuf,
};

use anyhow::Result;

use crate::{
    args::MergeArgs,
    coverage::{CheckStatus, CombinedCoverageResults, CovResult, CoverageCheck, CoverageResults},
};

/// Executes the `merge` subcommand.
///
/// First, it loads the raw coverage results from "kaniraw" files. Then, it
/// combines those results by coverage region thereby producing aggregated
/// coverage information. Finally, it saves that information into another file
/// (the coverage profile also known as the "kanicov" file).
pub fn merge_main(args: &MergeArgs) -> Result<()> {
    let raw_results = parse_raw_results(&args.files)?;
    let combined_results = combine_raw_results(&raw_results);
    save_combined_results(&combined_results, &args.output)?;
    Ok(())
}

/// Validate arguments to the `merge` subcommand in addition to clap's
/// validation.
pub fn validate_merge_args(_args: &MergeArgs) -> Result<()> {
    // No validation is done at the moment
    Ok(())
}

/// Parse raw coverage results from a set of files (AKA "kaniraw" files)
fn parse_raw_results(paths: &Vec<PathBuf>) -> Result<Vec<CoverageResults>> {
    let mut raw_results = Vec::with_capacity(paths.len());
    for path in paths {
        let filename = path.to_string_lossy();
        let file = File::open(path).expect(&format!("could not open file {filename}"));
        let reader = BufReader::new(file);

        let result = serde_json::from_reader(reader)
            .expect(&format!("could not deserialize file {filename}"));
        raw_results.push(result);
    }
    Ok(raw_results)
}

/// Combine raw coverage results into an aggregated form
fn combine_raw_results(results: &Vec<CoverageResults>) -> CombinedCoverageResults {
    let all_file_function_names = function_names_from_results(results);

    let mut new_data: HashMap<String, Vec<(String, Vec<CovResult>)>> = HashMap::new();

    for (file_name, fun_name) in all_file_function_names {
        let mut this_fun_checks: Vec<&CoverageCheck> = Vec::new();

        for result in results {
            if result.data.contains_key(&file_name) {
                this_fun_checks.extend(
                    result
                        .data
                        .get(&file_name)
                        .unwrap()
                        .iter()
                        .filter(|check| check.function == fun_name),
                )
            }
        }

        let mut new_results = Vec::new();

        while !this_fun_checks.is_empty() {
            // Take the first check, and split `this_fun_checks` into checks
            // covering the same region as that check, and checks which do not.
            let this_region_check = this_fun_checks[0];
            let (same_region_checks, other_region_checks) = this_fun_checks
                .into_iter()
                .partition(|check| check.region == this_region_check.region);
            // Update `this_fun_checks` with checks that aren't being processed yet
            this_fun_checks = other_region_checks;

            // Calculate `total_times` and `times_covered` for this region
            let total_times = same_region_checks.len();
            let times_covered = same_region_checks
                .iter()
                .filter(|check| check.status == CheckStatus::Covered)
                .count();

            let new_result = CovResult {
                function: fun_name.clone(),
                region: this_region_check.region.clone(),
                times_covered,
                total_times,
            };
            new_results.push(new_result);
        }

        let filename_copy = file_name.clone();
        if new_data.contains_key(&file_name) {
            new_data.get_mut(&filename_copy).unwrap().push((fun_name, new_results));
        } else {
            new_data.insert(file_name.clone(), vec![(fun_name, new_results)]);
        }
    }
    CombinedCoverageResults { data: new_data }
}

/// Save the combined coverage results into a file
fn save_combined_results(
    results: &CombinedCoverageResults,
    output: &Option<PathBuf>,
) -> Result<()> {
    let output_path =
        if let Some(out) = output { out } else { &PathBuf::from("default_kanicov.json") };

    let file = OpenOptions::new().write(true).create(true).truncate(true).open(output_path)?;
    let writer = BufWriter::new(file);

    serde_json::to_writer(writer, results)?;

    Ok(())
}

/// All function names appearing in raw coverage results
fn function_names_from_results(results: &[CoverageResults]) -> Vec<(String, String)> {
    let mut file_function_pairs = HashSet::new();
    for result in results {
        for (file, checks) in &result.data {
            for check in checks {
                file_function_pairs.insert((file.clone(), check.function.clone()));
            }
        }
    }
    file_function_pairs.into_iter().collect()
}
