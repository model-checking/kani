// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This module includes the implementation of the `report` subcommand.

use std::io::{BufRead, IsTerminal};
use std::{fs::File, io::BufReader, path::PathBuf};

use anyhow::Result;

use crate::args::ReportFormat;
use crate::coverage::{
    function_coverage_results, function_info_from_file, CovResult, LineResults, MarkerInfo,
};
use crate::summary::line_coverage_results;
use crate::{args::ReportArgs, coverage::CombinedCoverageResults};

/// Executes the `report` subcommand.
///
/// First, it loads the coverage metadata and results from the files passed as
/// arguments. Then, for each file referenced in the metadata, it computes its
/// associated coverage information on a per-function basis, producing a
/// human-readable report for each one of the files.
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
        output_coverage_results(&checked_format, file, file_cov_info)?;
    }

    Ok(())
}

/// Validate arguments to the `report` subcommand in addition to clap's
/// validation.
pub fn validate_report_args(_args: &ReportArgs) -> Result<()> {
    // No validation is done at the moment
    Ok(())
}

/// Checks the `format` argument for the `report` subcommand and selects to
/// another format if the one we are checking is not possible for any reason.
///
/// For example, if the `Terminal` format is specified but we are not writing to
/// a terminal, it will auto-select the `Escapes` format to avoid character
/// issues.
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

/// Output coverage results, and highlight regions or lines which are not
/// covered.
///
/// When highlighting the terminal, highlighted areas must be closed on the each
/// line to avoid highlighting beyond the source code area. Therefore, this
/// highlighting algorithm emits codes for both opening and closing the
/// highlighted area if any highlighting must be done for a line.
///
/// Therefore, a coverage region spanning multiple lines will be represented
/// with multiple highlighted lines. For example, let's take this uncovered
/// function:
///
/// fn _other_function() {
///     println!("Hello, world!");
/// }
///
/// It will be highlighted as follows:
///
/// ```fn _other_function() {'''
/// ```    println!("Hello, world!");'''
/// ```}'''
///
/// Note how there is always a corresponding escape for each line.
/// This is more sophisticated than the naive solution, which would simply emit two escapes
/// per region (one opening, one closing) as follows:
///
/// ```fn _other_function() {
///     println!("Hello, world!");
/// }'''
///
pub fn output_coverage_results(
    format: &ReportFormat,
    filepath: PathBuf,
    results: Vec<LineResults>,
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

        let (max_times, line_fmt) = if let Some((_, Some((max, marker_info)))) = cur_line_result {
            match marker_info {
                MarkerInfo::FullLine => {
                    (Some(max), insert_escapes(&line, vec![(0, true), (line.len(), false)], format))
                }
                MarkerInfo::Markers(results) => {
                    // Filter out cases where the region represents a
                    // single-column span and the regions ends after the line.
                    // TODO: Avoid filtering out these coverage results.
                    // <https://github.com/model-checking/kani/issues/3543>
                    let results: Vec<&CovResult> = results
                        .iter()
                        .filter(|m| {
                            if m.region.start.0 as usize == idx && m.region.end.0 as usize == idx {
                                (m.region.end.1 - m.region.start.1 != 1)
                                    && (m.region.end.1 as usize) < line.len()
                            } else {
                                true
                            }
                        })
                        .collect();
                    // Escapes for the regions which start and finish in this line
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
                    // Escapes for the regions which only start in this line
                    let mut opening_escapes: Vec<(usize, bool)> = results
                        .iter()
                        .filter(|m| {
                            m.times_covered == 0
                                && m.region.start.0 as usize == idx
                                && m.region.end.0 as usize != idx
                        })
                        .flat_map(|m| vec![((m.region.start.1 - 1) as usize, true)])
                        .collect();
                    // Escapes for the regions which only finish in this line
                    let mut closing_escapes: Vec<(usize, bool)> = results
                        .iter()
                        .filter(|m| {
                            m.times_covered == 0
                                && m.region.start.0 as usize != idx
                                && m.region.end.0 as usize == idx
                        })
                        .flat_map(|m| vec![((m.region.end.1 - 1) as usize, false)])
                        .collect();

                    // Emit an opening escape if there was a closing one and we
                    // had to continue the highlight
                    if must_highlight && !closing_escapes.is_empty() {
                        closing_escapes.push((0_usize, true));
                        must_highlight = false;
                    }
                    // Continue the highlight in the next lines if we had an
                    // opening escape
                    if !opening_escapes.is_empty() {
                        opening_escapes.push((line.len(), false));
                        must_highlight = true;
                    }

                    // Join all the escapes, then insert them
                    closing_escapes.extend(complete_escapes);
                    closing_escapes.extend(opening_escapes);

                    if must_highlight && closing_escapes.is_empty() {
                        closing_escapes.push((0, true));
                        closing_escapes.push((line.len(), false));
                    }

                    (Some(max), insert_escapes(&line, closing_escapes, format))
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
        };

        let max_fmt =
            if let Some(num) = max_times { format!("{num:4}") } else { format!("{:4}", " ") };

        println!("{idx:4}| {max_fmt}| {line_fmt}");
    }

    Ok(())
}

/// Inserts opening/closing escape strings into `str` given a set of `markers`.
/// Each marker is a tuple `(offset, type)` where:
///  * `offset` represents the offset in which the marker must be inserted, and
///  * `type` represents whether it is an opening (`true`) or closing (`false`)
///    escape.
///
/// The specific escape to be used are determined by the report format.
fn insert_escapes(str: &str, markers: Vec<(usize, bool)>, format: &ReportFormat) -> String {
    // Determine the escape strings based on the format
    let (open_escape, close_escape) = match format {
        ReportFormat::Terminal => ("\x1b[41m", "\x1b[0m"),
        ReportFormat::Escapes => ("```", "'''"),
    };

    let mut escape_markers: Vec<(&usize, &str)> =
        markers.iter().map(|(i, b)| (i, if *b { open_escape } else { close_escape })).collect();
    escape_markers.sort();

    let mut escaped_str = str.to_owned();
    let mut offset = 0;

    // Iteratively insert the escape strings into the original string
    for (i, b) in escape_markers {
        escaped_str.insert_str(i + offset, b);
        // `offset` keeps track of the bytes we've already inserted so the original
        // index is shifted by the appropriate amount in subsequent insertions.
        offset += b.bytes().len();
    }
    escaped_str
}
