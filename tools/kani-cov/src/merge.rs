// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::{collections::BTreeMap, fs::File, io::{BufReader, BufWriter}, os::linux::raw, path::PathBuf};

use anyhow::Result;

use crate::{args::MergeArgs, coverage::{CheckStatus, CombinedCoverageResults, CovResult, CoverageCheck, CoverageResults}};

pub fn merge_main(args: &MergeArgs) -> Result<()> {
    let raw_results = parse_raw_results(&args.files)?;
    let combined_results = combine_raw_results(&raw_results);
    save_combined_results(&combined_results, &args.output)?;
    Ok(())
}

pub fn validate_merge_args(_args: &MergeArgs) -> Result<()> {
    Ok(())
}

fn parse_raw_results(paths: &Vec<PathBuf>) -> Result<Vec<CoverageResults>> {
    let mut raw_results = Vec::with_capacity(paths.len());
    for path in paths {
        let filename = path.to_string_lossy();
        let file = File::open(path).expect(&format!("could not open file {filename}"));
        let reader = BufReader::new(file);

        let result = serde_json::from_reader(reader).expect(&format!("could not deserialize file {filename}"));
        raw_results.push(result);
    }
    Ok(raw_results)
}

fn combine_raw_results(results: &Vec<CoverageResults>) -> CombinedCoverageResults {
    let all_function_names = function_names_from_results(results);

    let mut new_data = BTreeMap::new();
    for fun_name in all_function_names {
        let mut this_fun_checks: Vec<&CoverageCheck> = Vec::new();

        for result in results {
            if result.data.contains_key(&fun_name) {
                this_fun_checks.extend(result.data.get(&fun_name).unwrap())
            }
        }

        let mut new_results = Vec::new();
        while !this_fun_checks.is_empty() {
            let this_region_check = this_fun_checks[0];
            // should do this with a partition...
            let mut same_region_checks: Vec<&CoverageCheck> = this_fun_checks.iter().cloned().filter(|check| check.region == this_region_check.region).collect();
            this_fun_checks.retain(|check| check.region != this_region_check.region);
            same_region_checks.push(this_region_check);
            let total_times = same_region_checks.len().try_into().unwrap();
            let times_covered = same_region_checks.iter().filter(|check| check.status == CheckStatus::Covered).count().try_into().unwrap();
            let new_result = CovResult { function: fun_name.clone(), region: this_region_check.region.clone() , times_covered, total_times };
            new_results.push(new_result);
        }

        new_data.insert(fun_name, new_results);
    }
    CombinedCoverageResults { data: new_data }
}

fn save_combined_results(results: &CombinedCoverageResults, output: &Option<PathBuf>) -> Result<()> {
    let output_path = if let Some(out) = output { out } else { &PathBuf::from("default_kanicov.json") };
    let file = File::create(output_path)?;
    let writer = BufWriter::new(file);

    serde_json::to_writer(writer, results)?;

    Ok(())
}

fn function_names_from_results(results: &Vec<CoverageResults>) -> Vec<String> {
    results.iter().map(|result| result.data.keys().cloned().collect()).collect()
}
