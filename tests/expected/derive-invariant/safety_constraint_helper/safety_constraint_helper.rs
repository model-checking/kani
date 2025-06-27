// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Check that the `#[safety_constraint(...)]` attribute helper adds the conditions provided to
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
    // In this case, we build the struct from unconstrained arbitrary values
    // that do not respect `PositivePoint`'s safety constraints.
    let pos_point: PositivePoint = PositivePoint { x: kani::any(), y: kani::any() };
    assert!(pos_point.is_safe());
}

#[kani::proof]
fn check_invariant_helper_ok_manual() {
    // In this case, we build the struct from unconstrained arbitrary values
    // that do not respect `PositivePoint`'s safety constraints. However, we
    // manually constrain them later.
    let pos_point: PositivePoint = PositivePoint { x: kani::any(), y: kani::any() };
    kani::assume(pos_point.x >= 0);
    kani::assume(pos_point.y >= 0);
    assert!(pos_point.is_safe());
}
