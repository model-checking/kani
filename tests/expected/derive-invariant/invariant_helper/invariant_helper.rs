// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Check that the invariant attribute helper adds the conditions provided to
//! the attribute to the derived `Invariant` implementation.

extern crate kani;
use kani::Invariant;

#[derive(kani::Arbitrary)]
#[derive(kani::Invariant)]
struct PositivePoint {
    #[invariant(self.x >= 0)]
    x: i32,
    #[invariant(self.y >= 0)]
    y: i32,
}

#[kani::proof]
fn check_invariant_helper_ok() {
    let pos_point: PositivePoint = kani::any();
    kani::assume(pos_point.x >= 0);
    kani::assume(pos_point.y >= 0);
    assert!(pos_point.is_safe());
}

#[kani::proof]
#[kani::should_panic]
fn check_invariant_helper_fail() {
    let pos_point: PositivePoint = kani::any();
    assert!(pos_point.is_safe());
}
