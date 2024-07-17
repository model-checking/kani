// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Check that the invariant attribute helper adds the conditions provided to
//! the attribute to the derived `Invariant` implementation.

extern crate kani;
use kani::Invariant;

#[derive(kani::Arbitrary)]
#[derive(kani::Invariant)]
struct PositivePoint {
    #[safety_constraint(*x >= 0)]
    x: i32,
    #[safety_constraint(*y >= 0)]
    y: i32,
}

#[kani::proof]
fn check_invariant_helper_ok() {
    let pos_point: PositivePoint = kani::any();
    assert!(pos_point.is_safe());
}

#[kani::proof]
#[kani::should_panic]
fn check_invariant_helper_fail() {
    // In this case, `kani::any()` returns unconstrained values that don't
    // respect the invariants.
    let pos_point: PositivePoint = PositivePoint { x: kani::any(), y: kani::any() };
    assert!(pos_point.is_safe());
}

#[kani::proof]
fn check_invariant_helper_ok_manual() {
    // In this case, `kani::any()` returns unconstrained values that don't
    // respect the invariants. However, we manually constrain them later.
    let pos_point: PositivePoint = PositivePoint { x: kani::any(), y: kani::any() };
    kani::assume(pos_point.x >= 0);
    kani::assume(pos_point.y >= 0);
    assert!(pos_point.is_safe());
}
