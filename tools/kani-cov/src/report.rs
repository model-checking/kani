// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::io::{BufRead, IsTerminal};
use std::{fs::File, io::BufReader, path::PathBuf};

use anyhow::Result;

use crate::args::ReportFormat;
use crate::coverage::{function_coverage_results, function_info_from_file, CovResult, MarkerInfo};
use crate::summary::line_coverage_results;
use crate::{args::ReportArgs, coverage::CombinedCoverageResults};

pub fn report_main(args: &ReportArgs) -> Result<()> {
    let mapfile = File::open(&args.mapfile)?;
    let reader = BufReader::new(mapfile);

    let covfile = File::open(&args.profile)?;
    let covreader = BufReader::new(covfile);
    let results: CombinedCoverageResults =
        serde_json::from_reader(covreader).expect("could not load coverage results");

    let source_files: Vec<PathBuf> =
        serde_json::from_reader(reader).expect("could not parse coverage metadata");

    let checked_format = check_format(&args.format);

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
        print_coverage_results(&checked_format, file, file_cov_info)?;
    }

    Ok(())
}

// Validate arguments to the `report` subcommand in addition to clap's
// validation.
pub fn validate_report_args(_args: &ReportArgs) -> Result<()> {
    // No validation is done at the moment
    Ok(())
}

fn check_format(format: &ReportFormat) -> ReportFormat {
    let is_terminal = std::io::stdout().is_terminal();
    match format {
        ReportFormat::Terminal => {
            if is_terminal {
                ReportFormat::Terminal
            } else {
                ReportFormat::Escapes
            }
        }
        ReportFormat::Escapes => ReportFormat::Escapes,
    }
}

pub fn print_coverage_results(
    format: &ReportFormat,
    filepath: PathBuf,
    results: Vec<Vec<(usize, Option<(u32, MarkerInfo)>)>>,
) -> Result<()> {
    let flattened_results: Vec<(usize, Option<(u32, MarkerInfo)>)> =
        results.into_iter().flatten().collect();
    println!("{}", filepath.to_string_lossy());

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
                    MarkerInfo::FullLine => (
                        Some(max),
                        insert_escapes(&line, vec![(0, true), (line.len(), false)], format),
                    ),
                    MarkerInfo::Markers(results) =>
                    // Note: I'm not sure why we need to offset the columns by -1
                    {
                        // Filter out cases where the span is a single unit AND it ends after the line
                        let results: Vec<&CovResult> = results
                            .iter()
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
                        let complete_escapes: Vec<(usize, bool)> = results
                            .iter()
                            .filter(|m| {
                                m.times_covered == 0
                                    && m.region.start.0 as usize == idx
                                    && m.region.end.0 as usize == idx
                            })
                            .flat_map(|m| {
                                vec![
                                    ((m.region.start.1 - 1) as usize, true),
                                    ((m.region.end.1 - 1) as usize, false),
                                ]
                            })
                            .collect();
                        // println!("COMPLETE: {complete_escapes:?}");
                        let mut starting_escapes: Vec<(usize, bool)> = results
                            .iter()
                            .filter(|m| {
                                m.times_covered == 0
                                    && m.region.start.0 as usize == idx
                                    && m.region.end.0 as usize != idx
                            })
                            .flat_map(|m| vec![((m.region.start.1 - 1) as usize, true)])
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
                        if must_highlight && !ending_escapes.is_empty() {
                            ending_escapes.push((0_usize, true));
                            must_highlight = false;
                        }
                        if !starting_escapes.is_empty() {
                            starting_escapes.push((line.len(), false));
                            must_highlight = true;
                        }

                        ending_escapes.extend(complete_escapes);
                        ending_escapes.extend(starting_escapes);

                        if must_highlight && ending_escapes.is_empty() {
                            ending_escapes.push((0, true));
                            ending_escapes.push((line.len(), false));
                        }

                        (Some(max), insert_escapes(&line, ending_escapes, format))
                    }
                }
            } else {
                (
                    None,
                    if !must_highlight {
                        line
                    } else {
                        insert_escapes(&line, vec![(0, true), (line.len(), false)], format)
                    },
                )
            }
        } else {
            (
                None,
                if !must_highlight {
                    line
                } else {
                    insert_escapes(&line, vec![(0, true), (line.len(), false)], format)
                },
            )
        };

        let max_fmt =
            if let Some(num) = max_times { format!("{num:4}") } else { format!("{:4}", " ") };

        println!("{idx:4}| {max_fmt}| {line_fmt}");
    }

    Ok(())
}

fn insert_escapes(str: &String, markers: Vec<(usize, bool)>, format: &ReportFormat) -> String {
    let mut new_str = str.clone();
    let mut offset = 0;

    let (open_escape, close_escape) = match format {
        ReportFormat::Terminal => ("\x1b[41m", "\x1b[0m"),
        ReportFormat::Escapes => ("```", "'''"),
    };

    let mut sym_markers: Vec<(&usize, &str)> =
        markers.iter().map(|(i, b)| (i, if *b { open_escape } else { close_escape })).collect();

    sym_markers.sort();
    for (i, b) in sym_markers {
        new_str.insert_str(i + offset, b);
        offset += b.bytes().len();
    }
    new_str
}
