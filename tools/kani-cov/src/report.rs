// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::io::{BufRead, IsTerminal};
use std::{fs::File, io::BufReader, path::PathBuf};

use anyhow::Result;

use crate::coverage::{function_coverage_results, function_info_from_file, CovResult, MarkerInfo};
use crate::summary::line_coverage_results;
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

    let mut must_highlight = false;

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
                    MarkerInfo::Markers(results) =>
                    // Note: I'm not sure why we need to offset the columns by -1
                    {
                        // Filter out cases where the span is a single unit AND it ends after the line
                        let results: Vec<&CovResult> = results
                            .into_iter()
                            .filter(|m| {
                                if m.region.start.0 as usize == idx
                                    && m.region.end.0 as usize == idx
                                {
                                    (m.region.end.1 - m.region.start.1 != 1)
                                        && (m.region.end.1 as usize) < line.len()
                                } else {
                                    true
                                }
                            })
                            .collect();
                        let mut complete_escapes: Vec<(usize, bool)> = results
                            .iter()
                            .filter(|m| {
                                m.times_covered == 0
                                    && m.region.start.0 as usize == idx
                                    && m.region.end.0 as usize == idx
                            })
                            .map(|m| {
                                vec![
                                    ((m.region.start.1 - 1) as usize, true),
                                    ((m.region.end.1 - 1) as usize, false),
                                ]
                            })
                            .flatten()
                            .collect();
                        // println!("COMPLETE: {complete_escapes:?}");
                        let mut starting_escapes: Vec<(usize, bool)> = results
                            .iter()
                            .filter(|m| {
                                m.times_covered == 0
                                    && m.region.start.0 as usize == idx
                                    && m.region.end.0 as usize != idx
                            })
                            .map(|m| vec![((m.region.start.1 - 1) as usize, true)])
                            .flatten()
                            .collect();
                        // println!("{starting_escapes:?}");
                        let mut ending_escapes: Vec<(usize, bool)> = results
                            .iter()
                            .filter(|m| {
                                m.times_covered == 0
                                    && m.region.start.0 as usize != idx
                                    && m.region.end.0 as usize == idx
                            })
                            .map(|m| vec![((m.region.end.1 - 1) as usize, false)])
                            .flatten()
                            .collect();

                        // println!("{starting_escapes:?}");
                        // println!("{ending_escapes:?}");
                        if must_highlight && ending_escapes.len() > 0 {
                            ending_escapes.push((0_usize, true));
                            must_highlight = false;
                        }
                        if starting_escapes.len() > 0 {
                            starting_escapes.push((line.len(), false));
                            must_highlight = true;
                        }

                        ending_escapes.extend(complete_escapes);
                        ending_escapes.extend(starting_escapes);

                        if must_highlight && ending_escapes.is_empty() {
                            ending_escapes.push((0, true));
                            ending_escapes.push((line.len(), false));
                        }

                        (Some(max), insert_escapes(&line, ending_escapes))
                    }
                }
            } else {
                (
                    None,
                    if !must_highlight {
                        line
                    } else {
                        insert_escapes(&line, vec![(0, true), (line.len(), false)])
                    },
                )
            }
        } else {
            (
                None,
                if !must_highlight {
                    line
                } else {
                    insert_escapes(&line, vec![(0, true), (line.len(), false)])
                },
            )
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
    let mut sym_markers: Vec<(&usize, &str)> = if support_color {
        markers.iter().map(|(i, b)| (i, if *b { "\x1b[41m" } else { "\x1b[0m" })).collect()
    } else {
        markers.iter().map(|(i, b)| (i, if *b { "```" } else { "'''" })).collect()
    };
    // Sorting
    sym_markers.sort();
    for (i, b) in sym_markers {
        new_str.insert_str(i + offset, b);
        offset = offset + b.bytes().len();
    }
    new_str
}
