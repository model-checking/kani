// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::cmp::Ordering;

use comfy_table::Table;
use kani_metadata::KaniMetadata;

use super::table_builder::{ColumnType, RenderableTableRow, TableBuilder, TableRow};

/// Reports unsupported features, in descending order of number of crates impacted.
///
/// The feature names come directly from the `operation_name` listed in `codegen_unimplemented`
///
/// For example:
///
/// ```text
/// ===================================================
///  Unsupported feature        |   Crates | Instances
///                             | impacted |    of use
/// ----------------------------+----------+-----------
///  'simd_or' intrinsic        |        4 |         5
///  try                        |        2 |        17
///  drop_in_place              |        2 |         2
/// ===================================================
/// ```
pub(crate) fn build(metadata: &KaniMetadata) -> Table {
    let mut builder = TableBuilder::new();

    for item in &metadata.unsupported_features {
        // key is unsupported feature name
        let mut key = item.feature.clone();
        // There are several "feature for <instance of use>" unsupported features.
        // We aggregate those here by reducing it to just "feature".
        // We should replace this with an enum: https://github.com/model-checking/kani/issues/1765
        if let Some((prefix, _)) = key.split_once(" for ") {
            key = prefix.to_string();
        }

        builder.add(UnsupportedFeaturesTableRow {
            unsupported_feature: key,
            crates_impacted: 1,
            instances_of_use: item.locations.len(),
        })
    }

    builder.render()
}

#[derive(Default)]
pub struct UnsupportedFeaturesTableRow {
    pub unsupported_feature: String,
    pub crates_impacted: usize,
    pub instances_of_use: usize,
}

impl TableRow for UnsupportedFeaturesTableRow {
    type Key = String;

    fn key(&self) -> Self::Key {
        self.unsupported_feature.clone()
    }

    fn merge(&mut self, new: Self) {
        self.crates_impacted += new.crates_impacted;
        self.instances_of_use += new.instances_of_use;
    }

    fn compare(&self, right: &Self) -> Ordering {
        self.crates_impacted
            .cmp(&right.crates_impacted)
            .reverse()
            .then_with(|| self.instances_of_use.cmp(&right.instances_of_use).reverse())
    }
}
impl RenderableTableRow for UnsupportedFeaturesTableRow {
    fn headers() -> Vec<&'static str> {
        vec!["Unsupported feature", "Crates\nimpacted", "Instances\nof use"]
    }

    fn columns() -> Vec<ColumnType> {
        use ColumnType::*;
        vec![Text, Number, Number]
    }

    fn row(&self) -> Vec<String> {
        vec![
            self.unsupported_feature.clone(),
            self.crates_impacted.to_string(),
            self.instances_of_use.to_string(),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_row_lengths() {
        use UnsupportedFeaturesTableRow as Row;
        assert_eq!(Row::columns().len(), Row::headers().len());
        assert_eq!(Row::columns().len(), Row::row(&Default::default()).len());
    }
}
