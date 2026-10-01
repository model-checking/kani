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
                sorted_checks.sort_by_key(|a| a.region.start);
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
    pub fn from_str(input: String) -> Self {
        // Split from the right: filenames may contain spaces, colons, or even " - ".
        let (start, end) = input.rsplit_once(" - ").unwrap();
        let (file_and_line, start_col) = start.rsplit_once(':').unwrap();
        let (file, start_line) = file_and_line.rsplit_once(':').unwrap();
        let (end_line, end_col) = end.split_once(':').unwrap();
        Self {
            file: file.to_owned(),
            start: (start_line.parse().unwrap(), start_col.parse().unwrap()),
            end: (end_line.parse().unwrap(), end_col.parse().unwrap()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::CoverageRegion;

    #[test]
    fn parse_coverage_region_paths() {
        for file in [
            "main.rs",
            "a a.rs",
            "dir with spaces/a a.rs",
            "dir:name/a - b.rs",
            "a:a:a:a2333::::::a.rs",
            "a - b - c.rs",
            " a.rs ",
        ] {
            let region = CoverageRegion::from_str(format!("{file}:12:3 - 14:5"));
            assert_eq!(
                region,
                CoverageRegion { file: file.to_owned(), start: (12, 3), end: (14, 5) }
            );
        }
    }
}
