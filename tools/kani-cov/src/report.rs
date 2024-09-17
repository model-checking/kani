// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::io::{BufRead, IsTerminal};
use std::{fs::File, io::BufReader, path::PathBuf};

use anyhow::Result;

use crate::coverage::{
    function_coverage_results, function_info_from_file, CovResult, FileCoverageInfo, FunctionInfo,
    MarkerInfo,
};
use crate::summary::{line_coverage_info, line_coverage_results};
use crate::{args::ReportArgs, coverage::CombinedCoverageResults};
// use crate::coverage::CoverageResults;
// use args::Args;

pub fn report_main(args: &ReportArgs) -> Result<()> {
    let mapfile = File::open(&args.mapfile)?;
    let reader = BufReader::new(mapfile);

    let covfile = File::open(&args.profile)?;
    let covreader = BufReader::new(covfile);
    let results: CombinedCoverageResults =
        serde_json::from_reader(covreader).expect("could not load coverage results");

    let source_files: Vec<PathBuf> =
        serde_json::from_reader(reader).expect("could not parse coverage metadata");

    for file in source_files {
        let fun_info = function_info_from_file(&file);
        let mut file_cov_info = Vec::new();
        for info in fun_info {
            let cov_results = function_coverage_results(&info, &file, &results);
            let line_coverage = line_coverage_results(&info, &cov_results);
            let line_coverage_matched: Vec<(usize, Option<(u32, MarkerInfo)>)> =
                (info.start.0..=info.end.0).zip(line_coverage.clone()).collect();
            // println!("REG: {line_coverage:?}");
            // println!("MATCHED: {line_coverage_matched:?}");
            // let new_res = line_coverage_matched.into_iter().filter(|(num, data)| data.is_some()).collect();
            file_cov_info.push(line_coverage_matched);
        }
        print_coverage_results(file, file_cov_info)?;
    }

    Ok(())
}

pub fn validate_report_args(_args: &ReportArgs) -> Result<()> {
    Ok(())
}

pub fn print_coverage_results(
    filepath: PathBuf,
    results: Vec<Vec<(usize, Option<(u32, MarkerInfo)>)>>,
) -> Result<()> {
    let flattened_results: Vec<(usize, Option<(u32, MarkerInfo)>)> =
        results.into_iter().flatten().collect();
    println!("{}", filepath.to_string_lossy().to_string());

    let file = File::open(filepath)?;
    let reader = BufReader::new(file);

    for (i, line) in reader.lines().enumerate() {
        let idx = i + 1;
        let line = line?;
        let cur_line_result = flattened_results.iter().find(|(num, _)| *num == idx);

        let (max_times, line_fmt) = if let Some((_, span_data)) = cur_line_result {
            if let Some((max, marker_info)) = span_data {
                match marker_info {
                    MarkerInfo::FullLine => {
                        (Some(max), insert_escapes(&line, vec![(0, true), (line.len(), false)]))
                    }
                    MarkerInfo::Markers(markers) =>
                    // Note: I'm not sure why we need to offset the columns by -1
                    {
                        (
                            Some(max),
                            insert_escapes(
                                &line,
                                markers
                                    .iter()
                                    .filter(|m| m.2 == 0)
                                    .map(|m| {
                                        vec![
                                            ((m.0 - 1) as usize, true),
                                            ((m.1 - 1) as usize, false),
                                        ]
                                    })
                                    .flatten()
                                    .collect(),
                            ),
                        )
                    }
                }
            } else {
                (None, line)
            }
        } else {
            (None, line)
        };

        let max_fmt =
            if let Some(num) = max_times { format!("{num:4}") } else { format!("{:4}", " ") };

        println!("{idx:4}| {max_fmt}| {line_fmt}");
    }

    Ok(())
}

fn insert_escapes(str: &String, markers: Vec<(usize, bool)>) -> String {
    let mut new_str = str.clone();
    let mut offset = 0;

    let support_color = std::io::stdout().is_terminal();
    let sym_markers: Vec<(&usize, &str)> = if support_color {
        markers.iter().map(|(i, b)| (i, if *b { "\x1b[41m" } else { "\x1b[0m" })).collect()
    } else {
        markers.iter().map(|(i, b)| (i, if *b { "```" } else { "'''" })).collect()
    };
    for (i, b) in sym_markers {
        // println!("{}", i + offset);
        new_str.insert_str(i + offset, b);
        offset = offset + b.bytes().len();
    }
    new_str
}
