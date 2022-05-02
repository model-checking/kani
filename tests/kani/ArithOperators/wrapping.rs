// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Check that none of these operations trigger spurious overflow checks.
#![feature(core_intrinsics)]

#[kani::proof]
fn main() {
    let a: u8 = kani::nondet();
    let b: u8 = kani::nondet();
    let sum0 = core::intrinsics::wrapping_add(a, b);
    let sum1 = a.wrapping_add(b);
    let sum2 = a.checked_add(b);
    assert!(sum0 == sum1);
    assert!(sum1 >= b || sum2.is_none());
    assert!(sum1 >= a || sum2.is_none());
}
