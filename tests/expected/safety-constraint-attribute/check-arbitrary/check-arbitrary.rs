// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Check that the `#[safety_constraint(...)]` attribute works as expected when
//! deriving `Arbitrary` and `Invariant` implementations.

extern crate kani;
use kani::Invariant;

#[derive(kani::Arbitrary)]
#[derive(kani::Invariant)]
#[safety_constraint(*x >= 0 && *y >= 0)]
struct Point {
    x: i32,
    y: i32,
}

#[kani::proof]
fn check_arbitrary() {
    let point: Point = kani::any();
    assert!(point.x >= 0);
    assert!(point.y >= 0);
    assert!(point.is_safe());
}
