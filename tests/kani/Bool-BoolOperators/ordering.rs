// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Regression test for ordering on symbolic booleans (issue #5232).

use std::cmp::{self, Ordering};

#[kani::proof]
fn bool_lt_truth_table() {
    let a: bool = kani::any();
    let b: bool = kani::any();
    kani::cover!(a != b);
    assert!((a < b) == (!a && b));
}

#[kani::proof]
fn bool_other_comparisons() {
    let a: bool = kani::any();
    let b: bool = kani::any();

    assert!((a <= b) == (!a || b));
    assert!((a > b) == (a && !b));
    assert!((a >= b) == (a || !b));

    let expected = match (a, b) {
        (false, true) => Ordering::Less,
        (true, false) => Ordering::Greater,
        _ => Ordering::Equal,
    };
    assert!(a.cmp(&b) == expected);
    assert!(cmp::min(a, b) == (a && b));
    assert!(cmp::max(a, b) == (a || b));
}
