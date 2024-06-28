// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Check that the invariant attribute helper adds the conditions provided to
//! the attribute to the derived `Arbitrary` implementation.

extern crate kani;
use kani::Invariant;

#[derive(kani::Arbitrary)]
struct PositivePoint {
    #[invariant(*x >= 0)]
    x: i32,
    #[invariant(*y >= 0)]
    y: i32,
}

#[kani::proof]
fn check_invariant_helper_ok() {
    let pos_point: PositivePoint = kani::any();
    assert!(pos_point.x >= 0);
    assert!(pos_point.y >= 0);
}

#[kani::proof]
#[kani::should_panic]
fn check_invariant_helper_fail() {
    let pos_point: PositivePoint = PositivePoint { x: kani::any(), y: kani::any() };
    assert!(pos_point.x >= 0);
    assert!(pos_point.y >= 0);
}
