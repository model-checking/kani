// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Modifications Copyright Kani Contributors
// See GitHub history for details.

/// Adapted from:
/// https://github.com/rust-lang/rust/blob/29c5a028b0c92aa5da6a8eb6d6585a389fcf1035/src/test/mir-opt/derefer_test.rs
#[kani::proof]
fn check_deref_copy() {
    let mut a = (42, 43);
    let mut b = (99, &mut a);
    let x = &mut (*b.1).0;
    let y = &mut (*b.1).1;
    assert_eq!(*x, 42);
    assert_eq!(*y, 43);
}
