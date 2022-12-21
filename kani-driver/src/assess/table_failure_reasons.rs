// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::cmp::Ordering;

use comfy_table::Table;

use crate::harness_runner::HarnessResult;

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
pub(crate) fn build(results: &[HarnessResult]) -> Table {
    let mut builder = TableBuilder::new();

    for r in results {
        let failures = r.result.failed_properties();
        let classification = if failures.is_empty() {
            "none (success)".to_string()
        } else {
            let mut classes: Vec<_> = failures.into_iter().map(|p| p.property_class()).collect();
            classes.sort();
            classes.dedup();
            classes.join(" + ")
        };

        builder.add(FailureReasonsTableRow { reason: classification, count: 1 });
    }

    builder.render()
}

#[derive(Default)]
pub struct FailureReasonsTableRow {
    pub reason: String,
    pub count: usize,
}

impl TableRow for FailureReasonsTableRow {
    type Key = String;

    fn key(&self) -> Self::Key {
        self.reason.clone()
    }

    fn merge(&mut self, new: Self) {
        self.count += new.count;
    }

    fn compare(&self, right: &Self) -> Ordering {
        self.count.cmp(&right.count).reverse().then_with(|| self.reason.cmp(&right.reason))
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
        vec![self.reason.clone(), self.count.to_string()]
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
