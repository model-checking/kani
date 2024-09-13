// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use console::style;
use serde_derive::{Deserialize, Serialize};
use std::{fmt, fs};
use std::path::PathBuf;
use std::{collections::BTreeMap, fmt::Display};
use tree_sitter::{Node, Parser};

#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum CheckStatus {
    Failure,
    Covered,   // for `code_coverage` properties only
    Satisfied, // for `cover` properties only
    Success,
    Undetermined,
    Unreachable,
    Uncovered,     // for `code_coverage` properties only
    Unsatisfiable, // for `cover` properties only
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CoverageResults {
    pub data: BTreeMap<String, Vec<CoverageCheck>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CombinedCoverageResults {
    pub data: BTreeMap<String, Vec<(String, Vec<CovResult>)>>,
}

// pub fn fmt_coverage_results(coverage_results: &CoverageResults) -> Result<String> {
//     let mut fmt_string = String::new();
//     for (file, checks) in coverage_results.data.iter() {
//         let mut checks_by_function: BTreeMap<String, Vec<CoverageCheck>> = BTreeMap::new();

//         // // Group checks by function
//         for check in checks {
//             // Insert the check into the vector corresponding to its function
//             checks_by_function
//                 .entry(check.function.clone())
//                 .or_insert_with(Vec::new)
//                 .push(check.clone());
//         }

//         for (function, checks) in checks_by_function {
//             writeln!(fmt_string, "{file} ({function})")?;
//             let mut sorted_checks: Vec<CoverageCheck> = checks.to_vec();
//             sorted_checks.sort_by(|a, b| a.region.start.cmp(&b.region.start));
//             for check in sorted_checks.iter() {
//                 writeln!(fmt_string, " * {} {}", check.region, check.status)?;
//             }
//             writeln!(fmt_string, "")?;
//         }
//     }
//     Ok(fmt_string)
// }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageCheck {
    pub function: String,
    term: CoverageTerm,
    pub region: CoverageRegion,
    pub status: CheckStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CovResult {
    pub function: String,
    // term: CoverageTerm,
    pub region: CoverageRegion,
    // status: CheckStatus,
    pub times_covered: u32,
    pub total_times: u32,
}

#[derive(Debug, Clone, Eq, PartialEq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct CoverageRegion {
    pub file: String,
    pub start: (u32, u32),
    pub end: (u32, u32),
}

impl Display for CoverageRegion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{} - {}:{}", self.start.0, self.start.1, self.end.0, self.end.1)
    }
}

// impl CoverageRegion {
//     pub fn from_str(str: String) -> Self {
//         let str_splits: Vec<&str> = str.split([':', '-']).map(|s| s.trim()).collect();
//         assert_eq!(str_splits.len(), 5, "{str:?}");
//         let file = str_splits[0].to_string();
//         let start = (str_splits[1].parse().unwrap(), str_splits[2].parse().unwrap());
//         let end = (str_splits[3].parse().unwrap(), str_splits[4].parse().unwrap());
//         Self { file, start, end }
//     }
// }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CoverageTerm {
    Counter(u32),
    Expression(u32),
}

impl std::fmt::Display for CheckStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let check_str = match self {
            CheckStatus::Satisfied => style("SATISFIED").green(),
            CheckStatus::Success => style("SUCCESS").green(),
            CheckStatus::Covered => style("COVERED").green(),
            CheckStatus::Uncovered => style("UNCOVERED").red(),
            CheckStatus::Failure => style("FAILURE").red(),
            CheckStatus::Unreachable => style("UNREACHABLE").yellow(),
            CheckStatus::Undetermined => style("UNDETERMINED").yellow(),
            CheckStatus::Unsatisfiable => style("UNSATISFIABLE").yellow(),
        };
        write!(f, "{check_str}")
    }
}

pub struct FileCoverageInfo {
    pub filename: String,
    pub function: CoverageMetric,
    pub line: CoverageMetric,
    pub region: CoverageMetric,
}

pub struct CoverageMetric {
    pub covered: u32,
    pub total: u32,
    // rate: Option<f32>
}

impl CoverageMetric {
    pub fn new(covered: u32, total: u32) -> Self {
        CoverageMetric { covered, total }
    }
}

pub fn function_info_from_file(filepath: &PathBuf) -> Vec<FunctionInfo> {
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

#[derive(Debug)]
pub struct FunctionInfo {
    pub name: String,
    pub start: (usize, usize),
    pub end: (usize, usize),
    pub num_lines: usize,
}

pub fn function_coverage_results(info: &FunctionInfo, file: &PathBuf, results: &CombinedCoverageResults) -> Option<(String, Vec<CovResult>)> {
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
