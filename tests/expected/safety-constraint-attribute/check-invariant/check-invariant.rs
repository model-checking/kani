// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Check that the `#[safety_constraint(...)]` attribute automatically generates
//! the `Arbitrary` and `Invariant` implementations for the annotated type.

extern crate kani;
use kani::Invariant;

#[derive(kani::Arbitrary)]
#[derive(kani::Invariant)]
#[safety_constraint(*x == *y)]
struct SameCoordsPoint {
    x: i32,
    y: i32,
}

#[kani::proof]
fn check_invariant() {
    let point: SameCoordsPoint = kani::any();
    assert!(point.is_safe());

    // Assuming `point.x != point.y` here should be like assuming `false`.
    // The assertion should be unreachable because we're blocking the path.
    kani::assume(point.x != point.y);
    assert!(false, "this assertion should unreachable");
}
