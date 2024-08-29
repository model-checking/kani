// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use anyhow::Result;
use console::style;
use serde_derive::{Deserialize, Serialize};
use std::fmt::{self, Write};
use std::{collections::BTreeMap, fmt::Display};

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

impl CoverageResults {
    pub fn new(data: BTreeMap<String, Vec<CoverageCheck>>) -> Self {
        Self { data }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CombinedCoverageResults {
    pub data: BTreeMap<String, Vec<CovResult>>,
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

impl CoverageCheck {
    pub fn new(
        function: String,
        term: CoverageTerm,
        region: CoverageRegion,
        status: CheckStatus,
    ) -> Self {
        Self { function, term, region, status }
    }

    pub fn is_covered(&self) -> bool {
        self.status == CheckStatus::Covered
    }
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

impl CoverageRegion {
    pub fn from_str(str: String) -> Self {
        let str_splits: Vec<&str> = str.split([':', '-']).map(|s| s.trim()).collect();
        assert_eq!(str_splits.len(), 5, "{str:?}");
        let file = str_splits[0].to_string();
        let start = (str_splits[1].parse().unwrap(), str_splits[2].parse().unwrap());
        let end = (str_splits[3].parse().unwrap(), str_splits[4].parse().unwrap());
        Self { file, start, end }
    }
}

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
