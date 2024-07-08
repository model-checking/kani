// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Check that a verification failure is triggered if we check the invariant
//! after mutating an object to violate it.

extern crate kani;
use kani::Invariant;

#[derive(kani::Arbitrary)]
#[derive(kani::Invariant)]
struct PositivePoint {
    #[invariant(*x >= 0)]
    x: i32,
    #[invariant(*y >= 0)]
    y: i32,
}

#[kani::proof]
#[kani::should_panic]
fn check_invariant_fail_mut() {
    let mut pos_point: PositivePoint = kani::any();
    assert!(pos_point.is_safe());
    // Set the `x` field to an unsafe value
    pos_point.x = -1;
    // The object's invariant isn't preserved anymore so the next check fails
    assert!(pos_point.is_safe());
}
