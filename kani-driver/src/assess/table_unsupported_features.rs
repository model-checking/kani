// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::{cmp::Ordering, collections::HashSet};

use kani_metadata::KaniMetadata;
use serde::{Deserialize, Serialize};

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
pub(crate) fn build(metadata: &[KaniMetadata]) -> TableBuilder<UnsupportedFeaturesTableRow> {
    let mut builder = TableBuilder::new();

    for package_metadata in metadata {
        for item in &package_metadata.unsupported_features {
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
                crates_impacted: HashSet::from([package_metadata.crate_name.to_owned()]),
                instances_of_use: item.locations.len(),
            })
        }
    }

    builder
}

/// Reports features that Kani does not yet support and records the packages that triggered these warnings.
///
/// See [`build`]
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct UnsupportedFeaturesTableRow {
    /// The unsupported feature name, generally given to `codegen_unimplemented` in `kani-compiler`
    pub unsupported_feature: String,
    /// The set of packages which had an instance of this feature somewhere in their build (even if from a reachable dependency)
    pub crates_impacted: HashSet<String>,
    /// The total count of the uses of this feature (we don't record details about where from only because that seems uninteresting so far)
    pub instances_of_use: usize,
}

impl TableRow for UnsupportedFeaturesTableRow {
    type Key = String;

    fn key(&self) -> Self::Key {
        self.unsupported_feature.clone()
    }

    fn merge(&mut self, new: Self) {
        self.crates_impacted.extend(new.crates_impacted);
        self.instances_of_use += new.instances_of_use;
    }

    fn compare(&self, right: &Self) -> Ordering {
        self.crates_impacted
            .len()
            .cmp(&right.crates_impacted.len())
            .reverse()
            .then_with(|| self.instances_of_use.cmp(&right.instances_of_use).reverse())
            .then_with(|| self.unsupported_feature.cmp(&right.unsupported_feature))
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
            self.crates_impacted.len().to_string(),
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
