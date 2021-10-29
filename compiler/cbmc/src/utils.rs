// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Useful utilities for CBMC

use num::bigint::BigInt;

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

#[test]
fn test_max_int() {
    // Unsigned
    assert_eq!(max_int(8, false), BigInt::from(u8::MAX));
    assert_eq!(max_int(16, false), BigInt::from(u16::MAX));
    assert_eq!(max_int(32, false), BigInt::from(u32::MAX));
    assert_eq!(max_int(64, false), BigInt::from(u64::MAX));
    assert_eq!(max_int(128, false), BigInt::from(u128::MAX));

    //Signed
    assert_eq!(max_int(8, true), BigInt::from(i8::MAX));
    assert_eq!(max_int(16, true), BigInt::from(i16::MAX));
    assert_eq!(max_int(32, true), BigInt::from(i32::MAX));
    assert_eq!(max_int(64, true), BigInt::from(i64::MAX));
    assert_eq!(max_int(128, true), BigInt::from(i128::MAX));
}

pub fn max_int(width: u64, signed: bool) -> BigInt {
    let mut bi = BigInt::from(0);
    if signed {
        bi.set_bit(width - 1, true);
    } else {
        bi.set_bit(width, true);
    }
    bi - 1
}

#[test]
fn test_min_int() {
    // Unsigned
    assert_eq!(min_int(8, false), BigInt::from(u8::MIN));
    assert_eq!(min_int(16, false), BigInt::from(u16::MIN));
    assert_eq!(min_int(32, false), BigInt::from(u32::MIN));
    assert_eq!(min_int(64, false), BigInt::from(u64::MIN));
    assert_eq!(min_int(128, false), BigInt::from(u128::MIN));

    //Signed
    assert_eq!(min_int(8, true), BigInt::from(i8::MIN));
    assert_eq!(min_int(16, true), BigInt::from(i16::MIN));
    assert_eq!(min_int(32, true), BigInt::from(i32::MIN));
    assert_eq!(min_int(64, true), BigInt::from(i64::MIN));
    assert_eq!(min_int(128, true), BigInt::from(i128::MIN));
}
pub fn min_int(width: u64, signed: bool) -> BigInt {
    if signed {
        let max = max_int(width, true);
        let min = -max - 1;
        min
    } else {
        BigInt::from(0)
    }
}
