// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Check that functions can be called in the `#[safety_constraint(...)]` attribute helpers.
//! This is like the `invariant_helper` test but using a function instead
//! of passing in a predicate.

extern crate kani;
use kani::Invariant;

#[derive(kani::Arbitrary)]
#[derive(kani::Invariant)]
struct PositivePoint {
    #[safety_constraint(is_coordinate_safe(x))]
    x: i32,
    #[safety_constraint(is_coordinate_safe(y))]
    y: i32,
}

fn is_coordinate_safe(val: &i32) -> bool {
    *val >= 0
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
