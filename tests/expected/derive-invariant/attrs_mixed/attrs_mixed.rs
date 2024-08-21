// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Check that in the `#[safety_constraint(...)]` attribute helper it is
//! possible to refer to other struct fields, not just the one associated with
//! the attribute.

extern crate kani;
use kani::Invariant;

#[derive(kani::Arbitrary)]
#[derive(kani::Invariant)]
struct PositivePoint {
    #[safety_constraint(*x >= 0 && *y >= 0)]
    x: i32,
    y: i32,
}

#[kani::proof]
fn check_safety_constraint_cfg() {
    let pos_point: PositivePoint = kani::any();
    assert!(pos_point.x >= 0);
    assert!(pos_point.y >= 0);
}
