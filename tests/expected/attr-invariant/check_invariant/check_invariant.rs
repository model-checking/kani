// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Check that the `#[safety_constraint(...)]` attribute automatically generates
//! the `Arbitrary` and `Invariant` implementations for the annotated type.

extern crate kani;
use kani::Invariant;

#[derive(kani::Arbitrary)]
#[derive(kani::Invariant)]
#[safety_constraint(true)]
struct Point {
    x: i32,
    y: i32,
}

#[kani::proof]
fn check_invariant() {
    let point: Point = kani::any();
    assert!(point.is_safe());
}
