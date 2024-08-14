// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use crate::cbmc_output_parser::CheckStatus;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fmt, fmt::Display};

/// The coverage data maps a function name to a set of coverage checks.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CoverageResults {
    pub data: BTreeMap<String, Vec<CoverageCheck>>,
}

impl CoverageResults {
    pub fn new(data: BTreeMap<String, Vec<CoverageCheck>>) -> Self {
        Self { data }
    }
}

impl fmt::Display for CoverageResults {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for (file, checks) in self.data.iter() {
            let mut checks_by_function: BTreeMap<String, Vec<CoverageCheck>> = BTreeMap::new();

            // Group checks by function
            for check in checks {
                // Insert the check into the vector corresponding to its function
                checks_by_function.entry(check.function.clone()).or_default().push(check.clone());
            }

            for (function, checks) in checks_by_function {
                writeln!(f, "{file} ({function})")?;
                let mut sorted_checks: Vec<CoverageCheck> = checks.to_vec();
                sorted_checks.sort_by(|a, b| a.region.start.cmp(&b.region.start));
                for check in sorted_checks.iter() {
                    writeln!(f, " * {} {}", check.region, check.status)?;
                }
                writeln!(f)?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageCheck {
    pub function: String,
    term: CoverageTerm,
    pub region: CoverageRegion,
    status: CheckStatus,
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CoverageTerm {
    Counter(u32),
    Expression(u32),
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
        let blank_splits: Vec<&str> = str.split_whitespace().map(|s| s.trim()).collect();
        assert!(blank_splits[1] == "-");
        let str_splits1: Vec<&str> = blank_splits[0].split([':']).collect();
        let str_splits2: Vec<&str> = blank_splits[2].split([':']).collect();
        assert_eq!(str_splits1.len(), 3, "{str:?}");
        assert_eq!(str_splits2.len(), 2, "{str:?}");
        let file = str_splits1[0].to_string();
        let start = (str_splits1[1].parse().unwrap(), str_splits1[2].parse().unwrap());
        let end = (str_splits2[0].parse().unwrap(), str_splits2[1].parse().unwrap());
        Self { file, start, end }
    }
}
