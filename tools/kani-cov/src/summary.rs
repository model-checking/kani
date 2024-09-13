// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::{
    cmp::max, fs::{self, File}, io::BufReader, path::PathBuf
};

use anyhow::Result;
use tree_sitter::{Node, Parser};

use crate::{args::{SummaryArgs, SummaryFormat}, coverage::{CombinedCoverageResults, CovResult, CoverageRegion}};

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
            let cur_function_coverage_results = FunctionCoverageResults { is_covered: function_coverage, total_lines: line_coverage.1, covered_lines: line_coverage.0, covered_regions: region_coverage.0, total_regions: region_coverage.1 };
            file_cov_info.push(cur_function_coverage_results);
        }
        let aggr_cov_info = aggregate_cov_info(&file, &file_cov_info);
        all_cov_info.push(aggr_cov_info);
    }
    print_coverage_info(&all_cov_info, &args.format);

    Ok(())
}

fn aggregate_cov_info(file: &PathBuf, file_cov_info: &Vec<FunctionCoverageResults>) -> FileCoverageInfo {
    let total_functions = file_cov_info.len().try_into().unwrap();
    let covered_functions = file_cov_info.iter().filter(|f| f.is_covered).count().try_into().unwrap();
    let fun_cov_info = FunCovInfo { covered: covered_functions, total: total_functions };
    
    let covered_lines = file_cov_info.iter().map(|c| c.covered_lines).sum();
    let total_lines = file_cov_info.iter().map(|c| c.total_lines).sum();
    let lines_cov_info = LineCovInfo  { covered: covered_lines, total: total_lines };
    
    let covered_regions = file_cov_info.iter().map(|c| c.covered_regions).sum();
    let total_regions = file_cov_info.iter().map(|c| c.total_regions).sum();
    let region_cov_info = RegionCovInfo { covered: covered_regions, total: total_regions };

    FileCoverageInfo {
        filename: file.to_string_lossy().to_string(),
        function: fun_cov_info,
        line: lines_cov_info,
        region: region_cov_info,
    }
}

fn function_coverage_info(cov_results: &Option<(String, Vec<CovResult>)>) -> bool {
    if let Some(res) = cov_results {
        res.1.iter().any(|c| c.times_covered > 0)
    } else {
        false
    }
}

struct FunctionCoverageResults {
    is_covered: bool,
    covered_lines: u32,
    total_lines: u32,
    covered_regions: u32,
    total_regions: u32,
}

pub fn validate_summary_args(_args: &SummaryArgs) -> Result<()> {
    Ok(())
}

fn function_coverage_results(info: &FunctionInfo, file: &PathBuf, results: &CombinedCoverageResults) -> Option<(String, Vec<CovResult>)> {
    // `info` does not include file so how do we match?
    // use function just for now...
    let filename = file.clone().into_os_string().into_string().unwrap();
    let right_filename = results.data.keys().find(|p| filename.ends_with(*p)).unwrap();
    // TODO: The filenames in kaniraw files should be absolute, just like in metadata
    // Otherwise the key for `results` just fails...
    let file_results = results.data.get(right_filename).unwrap();
    let function = info.name.clone();
    let fun_results = file_results.iter().find(|(f, _)| *f == function);
    fun_results.cloned()
}

// fn calculate_coverage_info(info: &FunctionInfo, file: &PathBuf, results: &CombinedCoverageResults) -> CovInfo {
//     let cov_info = calculate_cov_info(info, fun_results);
//     let lines_total = cov_info.iter().filter(|c|c.is_some()).count();
//     let lines_covered = cov_info.iter().filter(|c|c.is_some() && c.as_ref().unwrap().0 > 0).count();

//     CovInfo { filename: function, function: FunCovInfo { covered: 0, total: 0 }, line: LineCovInfo { covered: lines_covered.try_into().unwrap(), total: lines_total.try_into().unwrap() }, region: RegionCovInfo { covered: 0, total: 0 }}
//     // println!("{filename} {lines_covered}/{lines_total}");
//     // println!("{fun_results:?}");
// }

struct FileCoverageInfo {
    filename: String,
    function: FunCovInfo,
    line: LineCovInfo,
    region: RegionCovInfo,
}

struct FunCovInfo {
    covered: u32,
    total: u32,
}

struct LineCovInfo {
    covered: u32,
    total: u32,
}

struct RegionCovInfo {
    covered: u32,
    total: u32,
}


// enum LineCoverStatus {
//     Full,
//     Partial,
//     None,
// }

fn line_coverage_info(info: &FunctionInfo, fun_results: &Option<(String, Vec<crate::coverage::CovResult>)>) -> (u32, u32) {
    let start_line: u32 = info.start.0.try_into().unwrap();
    let end_line: u32 = info.end.0.try_into().unwrap();
    // `line_status` represents all the lines between `start_line` and
    // `end_line`. For each line, we will have either:
    // - `None`, meaning there were no results associated with this line (this
    // may happen with lines that only contain a closing `}`, for example).
    // - `Some(max, other)`, where `max` represents the maximum number of times
    // the line was covered by any coverage result, and `other` specifies the
    // coverage results that don't amount to the maximum.
    let mut line_status: Vec<Option<(u32, Vec<crate::coverage::CovResult>)>> = Vec::with_capacity((end_line - start_line + 1).try_into().unwrap());

    if let Some(res) = fun_results {
        let mut cur_results =  res.1.clone();
        // was this sorted already? looks like it was not
        // println!("BEFORE: {cur_results:?}");
        cur_results.sort_by(|a,b| b.region.start.0.cmp(&a.region.start.0));
        // println!("AFTER: {cur_results:?}");

        fn line_contained_in_region(line: u32, region: &CoverageRegion) -> bool {
            region.start.0 <= line && region.end.0 >= line
        }


        for line in start_line..end_line {
            let line_results: Vec<crate::coverage::CovResult> = cur_results.iter().filter(|c| line_contained_in_region(line, &c.region)).cloned().collect();
            if line_results.is_empty() {
                line_status.push(None);
            } else {
                let max_covered = line_results.iter().max_by_key(|obj| obj.times_covered).map(|obj| obj.times_covered).unwrap_or(0);
                let other_covered: Vec<crate::coverage::CovResult> = line_results.iter().filter(|obj| obj.times_covered != max_covered).cloned().collect();
                line_status.push(Some((max_covered, other_covered)));
            }
        }
        
        // println!("{} : {:?}", res.0, line_status);
        
        // sanity check
        // let info_start = (info.start.0.try_into().unwrap(),info.start.1.try_into().unwrap());
        // assert_eq!(cur_span.region.start, info_start);
    }
    let total_lines = line_status.iter().filter(|s| s.is_some()).count().try_into().unwrap();
    let covered_lines = line_status.iter().filter(|s| s.is_some() && s.as_ref().unwrap().0 > 0).count().try_into().unwrap();
    (covered_lines, total_lines)
}

fn region_coverage_info(fun_results: &Option<(String, Vec<crate::coverage::CovResult>)>) -> (u32, u32) {
    if let Some(res) = fun_results {
        let total_regions = res.1.len().try_into().unwrap();
        let covered_regions = res.1.iter().filter(|c| c.times_covered > 0).count().try_into().unwrap();
        (covered_regions, total_regions)
    } else { (0, 0) }
}

#[derive(Debug)]
struct FunctionInfo {
    name: String,
    start: (usize, usize),
    end: (usize, usize),
    num_lines: usize,
}

#[derive(Debug)]
struct NewFunctionInfo {
    // name: String,
    // start: (usize, usize),
    // end: (usize, usize),
    // function_covered: bool,
    // lines_covered: usize,
    // lines_total: usize,
    // regions_covered: Option<usize>,
    // regions_total: Option<usize>,
}

// struct SummaryInfo {
//     covered_functions: u32,
//     total_functions: u32,
// }

fn print_coverage_info(info: &Vec<FileCoverageInfo>, format: &SummaryFormat) {
    match format {
        SummaryFormat::Markdown => print_coverage_markdown_info(info),
    }
}

fn print_coverage_markdown_info(info: &Vec<FileCoverageInfo>) {

    fn safe_div(num: u32, denom: u32) -> Option<f32> {
        if denom == 0 { None }
        else { Some(num as f32/denom as f32) }
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
    let filename_space: String = std::iter::repeat(' ').take(max_filename_fmt_width - FILENAME_HEADER.len()).collect::<String>();
    let function_sep: String = std::iter::repeat('-').take(max_function_fmt_width).collect();
    let function_space: String = std::iter::repeat(' ').take(max_function_fmt_width - FUNCTION_HEADER.len()).collect::<String>();
    let line_sep: String = std::iter::repeat('-').take(max_line_fmt_width).collect();
    let line_space: String = std::iter::repeat(' ').take(max_line_fmt_width - LINE_HEADER.len()).collect::<String>();
    let region_sep: String = std::iter::repeat('-').take(max_region_fmt_width).collect();
    let region_space: String = std::iter::repeat(' ').take(max_region_fmt_width - REGION_HEADER.len()).collect::<String>();

    let sep_row = format!("| {filename_sep} | {function_sep} | {line_sep} | {region_sep} |");
    table_rows.push(format!("| {FILENAME_HEADER}{filename_space} | {FUNCTION_HEADER}{function_space} | {LINE_HEADER}{line_space} | {REGION_HEADER}{region_space} |"));
    table_rows.push(sep_row);
    for (filename, function_fmt, line_fmt, region_fmt) in data_rows {
        let filename_space: String = std::iter::repeat(' ').take(max_filename_fmt_width - filename.len()).collect::<String>();
        let function_space: String = std::iter::repeat(' ').take(max_function_fmt_width - function_fmt.len()).collect::<String>();
        let line_space: String = std::iter::repeat(' ').take(max_line_fmt_width - line_fmt.len()).collect::<String>();
        let region_space: String = std::iter::repeat(' ').take(max_region_fmt_width - region_fmt.len()).collect::<String>();
        let cur_row = format!("| {filename}{filename_space} | {function_fmt}{function_space} | {line_fmt}{line_space} | {region_fmt}{region_space} |");
        table_rows.push(cur_row);
    }

    println!("{}", table_rows.join("\n"));
}

fn function_info_from_file(filepath: &PathBuf) -> Vec<FunctionInfo> {
    let source_code = fs::read_to_string(filepath).expect("could not read source file");
    let mut parser = Parser::new();
    parser.set_language(&tree_sitter_rust::language()).expect("Error loading Rust grammar");

    let tree = parser.parse(&source_code, None).expect("Failed to parse file");

    let mut cursor = tree.walk();
    let first_child_exists = cursor.goto_first_child();

    if !first_child_exists {
        return vec![];
    }

    let mut function_info: Vec<FunctionInfo> = Vec::new();

    if cursor.node().kind() == "function_item" {
        function_info.push(function_info_from_node(cursor.node(), source_code.as_bytes()))
    };

    while cursor.goto_next_sibling() {
        if cursor.node().kind() == "function_item" {
            function_info.push(function_info_from_node(cursor.node(), source_code.as_bytes()))
        }
    }

    function_info
}

fn function_info_from_node<'a>(node: Node, source: &'a [u8]) -> FunctionInfo {
    let name = node
        .child_by_field_name("name")
        .and_then(|name| name.utf8_text(source).ok())
        .expect("couldn't get function name")
        .to_string();
    let start = (node.start_position().row + 1, node.start_position().column + 1);
    let end = (node.end_position().row + 1, node.end_position().column + 1);
    let num_lines = end.0 - start.0 + 1;
    FunctionInfo { name, start, end, num_lines }
}
