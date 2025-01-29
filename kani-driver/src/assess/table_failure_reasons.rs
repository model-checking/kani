// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::cmp::Ordering;
use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::{call_cbmc::ExitStatus, harness_runner::HarnessResult};

use super::table_builder::{ColumnType, RenderableTableRow, TableBuilder, TableRow};

/// Reports the most common "failure reasons" for tests run by assess.
///
/// The reasons are presently just a combination of "property classes"
/// from failed CBMC properties. This is only really meaningful to us,
/// and could use significant improvement for customers. In fact,
/// this particular data set might *only* be interesting to us as
/// developers of 'assess', and not customers, once we get fewer failures
/// and the heuristics for "promising tests" are improved.
///
/// Example:
///
/// ```text
/// ================================================
///  Reason for failure           | Number of tests
/// ------------------------------+-----------------
///  unwind                       |              61
///  none (success)               |               6
///  assertion                    |               4
///  assertion + overflow         |               2
/// ================================================
/// ```
pub(crate) fn build(results: &[HarnessResult]) -> TableBuilder<FailureReasonsTableRow> {
    let mut builder = TableBuilder::new();

    for r in results {
        let classification = if let Err(exit_status) = r.result.results {
            match exit_status {
                ExitStatus::Timeout => String::from("CBMC timed out"),
                ExitStatus::OutOfMemory => String::from("CBMC ran out of memory"),
                ExitStatus::Other(exit_code) => format!("CBMC failed with status {exit_code}"),
            }
        } else {
            let failures = r.result.failed_properties();
            if failures.is_empty() {
                "none (success)".to_string()
            } else {
                let mut classes: Vec<_> =
                    failures.into_iter().map(|p| p.property_class()).collect();
                classes.sort();
                classes.dedup();
                classes.join(" + ")
            }
        };

        let name = r.harness.pretty_name.trim_end_matches("::{closure#0}").to_string();
        let identity =
            format!("{} @ {}:{}", name, r.harness.original_file, r.harness.original_start_line);

        builder.add(FailureReasonsTableRow {
            reason: classification,
            tests: HashSet::from([identity]),
        });
    }

    builder
}

/// Reports the reasons that tests failed to be analyzed by Kani
///
/// See [`build`]
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct FailureReasonsTableRow {
    /// "Failure reasons" look like "unwind" or "assertion + overflow"
    pub reason: String,
    /// Tests are identified by "pretty_name @ /full/path/file.rs:line"
    pub tests: HashSet<String>,
}

impl TableRow for FailureReasonsTableRow {
    type Key = String;

    fn key(&self) -> Self::Key {
        self.reason.clone()
    }

    fn merge(&mut self, new: Self) {
        self.tests.extend(new.tests);
    }

    fn compare(&self, right: &Self) -> Ordering {
        self.tests
            .len()
            .cmp(&right.tests.len())
            .reverse()
            .then_with(|| self.reason.cmp(&right.reason))
    }
}

impl RenderableTableRow for FailureReasonsTableRow {
    fn headers() -> Vec<&'static str> {
        vec!["Reason for failure", "Number of tests"]
    }

    fn columns() -> Vec<ColumnType> {
        vec![ColumnType::Text, ColumnType::Number]
    }

    fn row(&self) -> Vec<String> {
        vec![self.reason.clone(), self.tests.len().to_string()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_row_lengths() {
        use FailureReasonsTableRow as Row;
        assert_eq!(Row::columns().len(), Row::headers().len());
        assert_eq!(Row::columns().len(), Row::row(&Default::default()).len());
    }
}
