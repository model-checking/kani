// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::cmp::Ordering;

use serde::{Deserialize, Serialize};

use crate::harness_runner::HarnessResult;

use super::table_builder::{ColumnType, RenderableTableRow, TableBuilder, TableRow};

/// Reports the "test harnesses" most likely to be easily turned into proof harnesses.
///
/// This presently is very naive and just reports successful harnesses, however
/// there is significant potential here to make use of improved heuristics,
/// and to find a way to *sort* these harnesses.
///
/// Example:
/// ```text
/// =============================================================================
///  Candidate for proof harness                           | Location
/// -------------------------------------------------------+---------------------
///  float::tests::f64_edge_cases                          | src/float.rs:226
///  float::tests::f32_edge_cases                          | src/float.rs:184
///  integer::tests::test_integers                         | src/integer.rs:171
///  other::tests::test_misc                               | src/other.rs:284
/// =============================================================================
/// ```
pub(crate) fn build(results: &[HarnessResult]) -> TableBuilder<PromisingTestsTableRow> {
    let mut builder = TableBuilder::new();

    for r in results.iter().filter(|res| res.result.results.is_ok()) {
        // For now we're just reporting "successful" harnesses as candidates.
        // In the future this heuristic should be expanded. More data is required to do this, however.
        if r.result.failed_properties().is_empty() {
            // The functions assess runs are actually the closures inside the test harness macro expansion.
            // This means they have (pretty) names like `krate::module::a_test_name::{closure#0}`
            // Strip that closure suffix, so we have better names for showing humans:
            let name = r.harness.pretty_name.trim_end_matches("::{closure#0}").to_string();
            // Location in a format "clickable" in e.g. IDE terminals
            let location = format!("{}:{}", r.harness.original_file, r.harness.original_start_line);

            builder.add(PromisingTestsTableRow { name, location });
        }
    }

    builder
}

/// Reports tests that Kani successfully analyzes, with a direct link to the test for easy viewing.
///
/// See [`build`]
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct PromisingTestsTableRow {
    /// The "pretty name" of the test, like "module::test_name"
    pub name: String,
    /// The "clickable location" of the test, like "/full/path/to/file.rs:123"
    pub location: String,
}

impl TableRow for PromisingTestsTableRow {
    type Key = String;

    fn key(&self) -> Self::Key {
        self.name.clone()
    }

    fn merge(&mut self, _new: Self) {
        unreachable!("This table should never have duplicate keys")
    }

    fn compare(&self, right: &Self) -> Ordering {
        // In the future this should use heuristics, but for now we have no really desired order
        self.name.cmp(&right.name)
    }
}

impl RenderableTableRow for PromisingTestsTableRow {
    fn headers() -> Vec<&'static str> {
        vec!["Candidate for proof harness", "Location"]
    }

    fn columns() -> Vec<ColumnType> {
        use ColumnType::*;
        vec![Text, Text]
    }

    fn row(&self) -> Vec<String> {
        vec![self.name.to_owned(), self.location.to_owned()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_row_lengths() {
        use PromisingTestsTableRow as Row;
        assert_eq!(Row::columns().len(), Row::headers().len());
        assert_eq!(Row::columns().len(), Row::row(&Default::default()).len());
    }
}
