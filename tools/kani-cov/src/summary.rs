// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::{cmp::max, fs::File, io::BufReader, path::PathBuf};

use anyhow::Result;

use crate::{
    args::{SummaryArgs, SummaryFormat},
    coverage::{
        function_coverage_results, function_info_from_file, CombinedCoverageResults, CovResult,
        CoverageMetric, CoverageRegion, FileCoverageInfo, FunctionInfo, MarkerInfo,
    },
};

pub fn summary_main(args: &SummaryArgs) -> Result<()> {
    let mapfile = File::open(&args.mapfile)?;
    let reader = BufReader::new(mapfile);

    let covfile = File::open(&args.profile)?;
    let covreader = BufReader::new(covfile);
    let results: CombinedCoverageResults =
        serde_json::from_reader(covreader).expect("could not load coverage results");

    let source_files: Vec<PathBuf> =
        serde_json::from_reader(reader).expect("could not parse coverage metadata");

    let mut all_cov_info: Vec<FileCoverageInfo> = Vec::new();

    for file in source_files {
        let fun_info = function_info_from_file(&file);
        let mut file_cov_info = Vec::new();
        for info in fun_info {
            let cov_results = function_coverage_results(&info, &file, &results);
            let function_coverage = function_coverage_info(&cov_results);
            let line_coverage = line_coverage_info(&info, &cov_results);
            let region_coverage = region_coverage_info(&cov_results);
            let cur_function_coverage_results = FunctionCoverageResults {
                is_covered: function_coverage,
                total_lines: line_coverage.1,
                covered_lines: line_coverage.0,
                covered_regions: region_coverage.0,
                total_regions: region_coverage.1,
            };
            file_cov_info.push(cur_function_coverage_results);
        }
        let aggr_cov_info = aggregate_cov_info(&file, &file_cov_info);
        all_cov_info.push(aggr_cov_info);
    }
    print_coverage_info(&all_cov_info, &args.format);

    Ok(())
}

fn aggregate_cov_info(
    file: &PathBuf,
    file_cov_info: &Vec<FunctionCoverageResults>,
) -> FileCoverageInfo {
    let total_functions = file_cov_info.len().try_into().unwrap();
    let covered_functions =
        file_cov_info.iter().filter(|f| f.is_covered).count().try_into().unwrap();
    let fun_cov_info = CoverageMetric::new(covered_functions, total_functions);

    let covered_lines = file_cov_info.iter().map(|c| c.covered_lines).sum();
    let total_lines = file_cov_info.iter().map(|c| c.total_lines).sum();
    let lines_cov_info = CoverageMetric::new(covered_lines, total_lines);

    let covered_regions = file_cov_info.iter().map(|c| c.covered_regions).sum();
    let total_regions = file_cov_info.iter().map(|c| c.total_regions).sum();
    let region_cov_info = CoverageMetric::new(covered_regions, total_regions);

    FileCoverageInfo {
        filename: file.to_string_lossy().to_string(),
        function: fun_cov_info,
        line: lines_cov_info,
        region: region_cov_info,
    }
}

fn function_coverage_info(cov_results: &Option<(String, Vec<CovResult>)>) -> bool {
    if let Some(res) = cov_results { res.1.iter().any(|c| c.times_covered > 0) } else { false }
}

struct FunctionCoverageResults {
    is_covered: bool,
    covered_lines: u32,
    total_lines: u32,
    covered_regions: u32,
    total_regions: u32,
}

// Validate arguments to the `summary` subcommand in addition to clap's
// validation.
pub fn validate_summary_args(_args: &SummaryArgs) -> Result<()> {
    // No validation is done at the moment
    Ok(())
}

/// Computes coverage results from a line-based perspective.
///
/// Basically, for each line we produce an `<Option<(u32, MarkerInfo)>>` result
/// where:
///  * `None` means there were no coverage results associated with this line.
/// This may happen in lines that only contain a closing `}`, for example.
///  * `Some(max, markers)` means there were coverage results associated with
/// the line or we deduced no results were possible based on function
/// information (i.e., the function was not reachable during verification).
/// Here, `max` represents the maximum number of times the line was covered by
/// any coverage result, and `markers` represents marker information which is
/// relevant to the line (including coverage results).
///
/// As a result, we essentially precompute here most of the information required
/// for the generation of coverage reports.
pub fn line_coverage_results(
    info: &FunctionInfo,
    fun_results: &Option<(String, Vec<crate::coverage::CovResult>)>,
) -> Vec<Option<(u32, MarkerInfo)>> {
    let start_line: u32 = info.start.0.try_into().unwrap();
    let end_line: u32 = info.end.0.try_into().unwrap();

    let mut line_status: Vec<Option<(u32, MarkerInfo)>> =
        Vec::with_capacity((end_line - start_line + 1).try_into().unwrap());

    if let Some(res) = fun_results {
        let mut cur_results = res.1.clone();
        // Sort the results by row
        cur_results.sort_by(|a, b| b.region.start.0.cmp(&a.region.start.0));

        /// Checks if a line is relevant to a region.
        /// Here, we define "relevant" as the line appearing after/at the start
        /// of a region and before/at the end of a region.
        fn line_relevant_to_region(line: u32, region: &CoverageRegion) -> bool {
            region.start.0 <= line && region.end.0 >= line
        }

        for line in start_line..end_line {
            // Filter results which are relevant to the current line
            let line_results: Vec<crate::coverage::CovResult> = cur_results
                .iter()
                .filter(|c| line_relevant_to_region(line, &c.region))
                .cloned()
                .collect();

            if line_results.is_empty() {
                line_status.push(None);
            } else {
                let max_covered = line_results
                    .iter()
                    .max_by_key(|res| res.times_covered)
                    .map(|res| res.times_covered)
                    .unwrap_or(0);
                line_status.push(Some((max_covered, MarkerInfo::Markers(line_results))));
            }
        }
    } else {
        line_status = std::iter::repeat(Some((0, MarkerInfo::FullLine)))
            .take((end_line - start_line + 1).try_into().unwrap())
            .collect();
    }
    line_status
}

/// Compute the number of covered lines and number of total lines given the
/// coverage results for a given function.
pub fn line_coverage_info(
    info: &FunctionInfo,
    fun_results: &Option<(String, Vec<crate::coverage::CovResult>)>,
) -> (u32, u32) {
    let line_status = line_coverage_results(info, fun_results);
    let total_lines = line_status.iter().filter(|s| s.is_some()).count().try_into().unwrap();
    let covered_lines = line_status
        .iter()
        .filter(|s| s.is_some() && s.as_ref().unwrap().0 > 0)
        .count()
        .try_into()
        .unwrap();
    (covered_lines, total_lines)
}

/// Compute the number of covered regions and number of total regions given the
/// coverage results for a given function.
fn region_coverage_info(
    fun_results: &Option<(String, Vec<crate::coverage::CovResult>)>,
) -> (u32, u32) {
    if let Some(res) = fun_results {
        let total_regions = res.1.len().try_into().unwrap();
        let covered_regions =
            res.1.iter().filter(|c| c.times_covered > 0).count().try_into().unwrap();
        (covered_regions, total_regions)
    } else {
        (0, 0)
    }
}

fn print_coverage_info(info: &Vec<FileCoverageInfo>, format: &SummaryFormat) {
    match format {
        SummaryFormat::Markdown => print_coverage_markdown_info(info),
        // SummaryFormat::Json => print_coverage_json_info(info),
    }
}

fn print_coverage_markdown_info(info: &Vec<FileCoverageInfo>) {
    fn safe_div(num: u32, denom: u32) -> Option<f32> {
        if denom == 0 { None } else { Some(num as f32 / denom as f32) }
    }

    const HEADERS_ROWS: usize = 3;
    const FILENAME_HEADER: &str = "Filename";
    const FUNCTION_HEADER: &str = "Function (%)";
    const LINE_HEADER: &str = "Line (%)";
    const REGION_HEADER: &str = "Region (%)";

    let mut table_rows: Vec<String> = Vec::with_capacity(HEADERS_ROWS + info.len() + 1);
    let mut max_filename_fmt_width = FILENAME_HEADER.len();
    let mut max_function_fmt_width = FUNCTION_HEADER.len();
    let mut max_line_fmt_width = LINE_HEADER.len();
    let mut max_region_fmt_width = REGION_HEADER.len();

    let mut data_rows: Vec<(String, String, String, String)> = Vec::with_capacity(info.len());

    for cov_info in info {
        let filename = cov_info.filename.to_string();
        let function_covered = cov_info.function.covered;
        let function_total = cov_info.function.total;
        let function_rate = safe_div(function_covered, function_total);
        let function_rate_fmt = if let Some(rate) = function_rate {
            format!("{:.2}", (rate * 100_f32))
        } else {
            "N/A".to_string()
        };
        let function_fmt = format!("{function_covered}/{function_total} ({function_rate_fmt})");

        let line_covered = cov_info.line.covered;
        let line_total = cov_info.line.total;
        let line_rate = safe_div(line_covered, line_total);
        let line_rate_fmt = if let Some(rate) = line_rate {
            format!("{:.2}", (rate * 100_f32))
        } else {
            "N/A".to_string()
        };
        let line_fmt = format!("{line_covered}/{line_total} ({line_rate_fmt})");

        let region_covered = cov_info.region.covered;
        let region_total = cov_info.region.total;
        let region_rate = safe_div(region_covered, region_total);
        let region_rate_fmt = if let Some(rate) = region_rate {
            format!("{:.2}", (rate * 100_f32))
        } else {
            "N/A".to_string()
        };
        let region_fmt = format!("{region_covered}/{region_total} ({region_rate_fmt})");

        max_filename_fmt_width = max(max_filename_fmt_width, filename.len());
        max_function_fmt_width = max(max_function_fmt_width, function_fmt.len());
        max_line_fmt_width = max(max_line_fmt_width, line_fmt.len());
        max_region_fmt_width = max(max_region_fmt_width, region_fmt.len());

        data_rows.push((filename, function_fmt, line_fmt, region_fmt));
    }

    let filename_sep: String = std::iter::repeat('-').take(max_filename_fmt_width).collect();
    let filename_space: String = std::iter::repeat(' ')
        .take(max_filename_fmt_width - FILENAME_HEADER.len())
        .collect::<String>();
    let function_sep: String = std::iter::repeat('-').take(max_function_fmt_width).collect();
    let function_space: String = std::iter::repeat(' ')
        .take(max_function_fmt_width - FUNCTION_HEADER.len())
        .collect::<String>();
    let line_sep: String = std::iter::repeat('-').take(max_line_fmt_width).collect();
    let line_space: String =
        std::iter::repeat(' ').take(max_line_fmt_width - LINE_HEADER.len()).collect::<String>();
    let region_sep: String = std::iter::repeat('-').take(max_region_fmt_width).collect();
    let region_space: String =
        std::iter::repeat(' ').take(max_region_fmt_width - REGION_HEADER.len()).collect::<String>();

    let sep_row = format!("| {filename_sep} | {function_sep} | {line_sep} | {region_sep} |");
    table_rows.push(format!("| {FILENAME_HEADER}{filename_space} | {FUNCTION_HEADER}{function_space} | {LINE_HEADER}{line_space} | {REGION_HEADER}{region_space} |"));
    table_rows.push(sep_row);
    for (filename, function_fmt, line_fmt, region_fmt) in data_rows {
        let filename_space: String = std::iter::repeat(' ')
            .take(max_filename_fmt_width - filename.len())
            .collect::<String>();
        let function_space: String = std::iter::repeat(' ')
            .take(max_function_fmt_width - function_fmt.len())
            .collect::<String>();
        let line_space: String =
            std::iter::repeat(' ').take(max_line_fmt_width - line_fmt.len()).collect::<String>();
        let region_space: String = std::iter::repeat(' ')
            .take(max_region_fmt_width - region_fmt.len())
            .collect::<String>();
        let cur_row = format!(
            "| {filename}{filename_space} | {function_fmt}{function_space} | {line_fmt}{line_space} | {region_fmt}{region_space} |"
        );
        table_rows.push(cur_row);
    }

    println!("{}", table_rows.join("\n"));
}
