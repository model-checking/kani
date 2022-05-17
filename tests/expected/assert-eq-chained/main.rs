// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// This test checks that chained operators work with Kani's overridden assert_eq
// macros

#[kani::proof]
fn check_chained_operators() {
    let x = 5;
    assert_eq!(x > 3, true);
    debug_assert_eq!(x == 1, false);
    assert_ne!(x < 10, false);
    debug_assert_ne!(x == 7, false); // fails
}
