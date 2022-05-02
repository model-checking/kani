// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// This test has more than 10 asserts to satisfy the condition that two check
// IDs are a prefix of each other, e.g.:
//     KANI_CHECK_ID_reach_check.225019a5::reach_check_1
//     KANI_CHECK_ID_reach_check.225019a5::reach_check_10

fn foo(x: i32) {
    assert!(1 + 1 == 2);
    if x < 9 {
        // unreachable
        assert!(2 + 2 == 4);
    }
}

#[kani::proof]
fn main() {
    assert!(1 + 1 == 2);
    let x = if kani::any() { 33 } else { 57 };
    foo(x);
    assert!(1 + 1 == 2);
    assert!(1 + 1 == 2);
    assert!(1 + 1 == 2);
    assert!(1 + 1 == 2);
    assert!(1 + 1 == 2);
    assert!(1 + 1 == 2);
    assert!(1 + 1 == 2);
    assert!(3 + 3 == 5);
}
