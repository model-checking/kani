// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! This module defines coverage-oriented data structures shared among
//! subcommands and other utilities like the Rust tree-sitter.

use console::style;
use serde_derive::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::{collections::HashMap, fmt::Display};
use std::{fmt, fs};
use tree_sitter::{Node, Parser};

pub type Function = String;
pub type Filename = String;
pub type LineNumber = usize;
pub type ColumnNumber = usize;

pub type LineResults = Vec<(LineNumber, Option<(usize, MarkerInfo)>)>;

/// The possible outcomes for a Kani check.
///
/// Note: This data structure should not be duplicated in Kani -
/// <https://github.com/model-checking/kani/issues/3541>
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

/// Kani coverage check.
///
/// Note: This data structure should not be duplicated in Kani -
/// <https://github.com/model-checking/kani/issues/3541>
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageCheck {
    pub function: Filename,
    term: CoverageTerm,
    pub region: CoverageRegion,
    pub status: CheckStatus,
}

// Note: This `impl` should not be duplicated in Kani -
// <https://github.com/model-checking/kani/issues/3541>
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

/// Raw Kani coverage results.
///
/// Note: This data structure should not be duplicated in Kani -
/// <https://github.com/model-checking/kani/issues/3541>
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CoverageResults {
    pub data: HashMap<Function, Vec<CoverageCheck>>,
}

/// Aggregated coverage results.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CombinedCoverageResults {
    pub data: HashMap<Filename, Vec<(Function, Vec<CovResult>)>>,
}

/// The coverage result associated to a particular coverage region.
///
/// Basically, this aggregates the information of one or more `CoverageCheck`
/// for a particular region. Thus, `total_times` represents the total number of
/// such checks, while `times_covered` keeps track of how many of those checks
/// had the `COVERED` status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CovResult {
    pub function: Filename,
    pub region: CoverageRegion,
    pub times_covered: usize,
    pub total_times: usize,
}

/// A coverage region.
/// `start` and `end` are tuples containing the line and column numbers.
///
/// Note: This data structure should not be duplicated in Kani -
/// <https://github.com/model-checking/kani/issues/3541>
#[derive(Debug, Clone, Eq, PartialEq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct CoverageRegion {
    pub file: Filename,
    pub start: (LineNumber, ColumnNumber),
    pub end: (LineNumber, ColumnNumber),
}

impl Display for CoverageRegion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}:{} - {}:{}", self.file, self.start.0, self.start.1, self.end.0, self.end.1)
    }
}

/// A coverage term.
///
/// Note: This data structure should not be duplicated in Kani -
/// <https://github.com/model-checking/kani/issues/3541>
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CoverageTerm {
    Counter(u32),
    Expression(u32),
}

/// The coverage information to produce for a particular file.
pub struct FileCoverageInfo {
    pub filename: Filename,
    pub function: CoverageMetric,
    pub line: CoverageMetric,
    pub region: CoverageMetric,
}

/// A coverage metric.
pub struct CoverageMetric {
    pub covered: usize,
    pub total: usize,
}

impl CoverageMetric {
    pub fn new(covered: usize, total: usize) -> Self {
        CoverageMetric { covered, total }
    }
}

/// Function information obtained through a tree-sitter
#[derive(Debug)]
pub struct FunctionInfo {
    pub name: Function,
    pub start: (LineNumber, ColumnNumber),
    pub end: (LineNumber, ColumnNumber),
}

/// Extract function information from a file using a tree-sitter
pub fn function_info_from_file(filepath: &PathBuf) -> Vec<FunctionInfo> {
    let source_code = fs::read_to_string(filepath).expect("could not read source file");
    let mut parser = Parser::new();
    parser.set_language(&tree_sitter_rust::LANGUAGE.into()).expect("Error loading Rust grammar");

    let tree = parser.parse(&source_code, None).expect("Failed to parse file");

    let source_code_bytes = source_code.as_bytes();
    let mut function_info: Vec<FunctionInfo> = Vec::new();
    let mut cursor = tree.walk();

    for node in tree.root_node().children(&mut cursor) {
        if node.kind() == "function_item" {
            function_info.push(function_info_from_node(node, source_code_bytes));
        }
    }

    function_info
}

/// Helper function to extract function information using a tree-sitter
fn function_info_from_node(node: Node, source: &[u8]) -> FunctionInfo {
    let name = node
        .child_by_field_name("name")
        .and_then(|name| name.utf8_text(source).ok())
        .expect("couldn't get function name")
        .to_string();
    let start = (node.start_position().row + 1, node.start_position().column + 1);
    let end = (node.end_position().row + 1, node.end_position().column + 1);
    FunctionInfo { name, start, end }
}

/// Extract the coverage results associated to a function
pub fn function_coverage_results(
    info: &FunctionInfo,
    file: &Path,
    results: &CombinedCoverageResults,
) -> Option<(Function, Vec<CovResult>)> {
    // The filenames in "kaniraw" files are not absolute, so we need to match
    // them with the ones we have in the aggregated results (i.e., the filenames
    // in the "kanimap" files).
    let filename = file.to_str()?;
    let right_filename = results.data.keys().find(|p| filename.ends_with(*p)).unwrap();
    // TODO: The filenames in kaniraw files should be absolute, just like in metadata.
    // Otherwise the key for `results` just fails... <https://github.com/model-checking/kani/issues/3542>
    let file_results = results.data.get(right_filename).unwrap();
    let function = &info.name;
    let fun_results = file_results.iter().find(|(f, _)| *f == *function);
    fun_results.cloned()
}

/// Marker information, mainly useful for highlighting coverage
#[derive(Debug, Clone)]
pub enum MarkerInfo {
    FullLine,
    Markers(Vec<CovResult>),
}
