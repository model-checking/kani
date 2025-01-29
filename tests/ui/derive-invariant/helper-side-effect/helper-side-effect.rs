// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Check that side effect expressions in the `#[safety_constraint(...)]`
//! attribute helpers are not allowed.

extern crate kani;
use kani::Invariant;

#[derive(kani::Arbitrary)]
#[derive(kani::Invariant)]
struct PositivePoint {
    #[safety_constraint({*(x.as_mut()) = 0; true})]
    x: Box<i32>,
    y: i32,
}

#[kani::proof]
fn check_invariant_helper_ok() {
    let pos_point: PositivePoint = kani::any();
    assert!(pos_point.is_safe());
}
