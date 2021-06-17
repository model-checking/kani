// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Useful utilities for CBMC

/// RMC bug report URL, for asserts/errors
pub const BUG_REPORT_URL: &str =
    "https://github.com/model-checking/rmc/issues/new?template=bug_report.md";

/// The aggregate name used in CBMC for aggregates of type `n`.
pub fn aggr_name(n: &str) -> String {
    format!("tag-{}", n)
}

/// Provides a useful shortcut for making BTreeMaps.
#[macro_export]
macro_rules! btree_map {
    ($($x:expr),*) => {{
        use std::collections::BTreeMap;
        use std::iter::FromIterator;
        (BTreeMap::from_iter(vec![$($x),*]))
    }};
    ($($x:expr,)*) => {{
        use std::collections::BTreeMap;
        use std::iter::FromIterator;
        (BTreeMap::from_iter(vec![$($x),*]))
    }}
}

/// Provides a useful shortcut for making BTreeMaps with String keys.
/// Arg: a list of (?, V) tuples, where ? is any type that implements `.to_string()`.
/// Result: BtreeMap<String, V> initilized with the values from the arg list.
#[macro_export]
macro_rules! btree_string_map {
    ($($x:expr),*) => {{
        use std::collections::BTreeMap;
        use std::iter::FromIterator;
        (BTreeMap::from_iter(vec![$($x),*].into_iter().map(|(k,v)|(k.to_string(),v))))
    }};
    ($($x:expr,)*) => {{
        use std::collections::BTreeMap;
        use std::iter::FromIterator;
        (BTreeMap::from_iter(vec![$($x),*].into_iter().map(|(k,v)|(k.to_string(),v))))
    }}
}
