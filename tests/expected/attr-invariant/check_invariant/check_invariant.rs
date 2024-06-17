// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Check that Kani can automatically derive `Invariant` for structs with named fields.

extern crate kani;
use kani::Invariant;

#[derive(kani::Arbitrary)]
#[kani::invariant(self.x.is_safe() && self.y.is_safe())]
struct Point {
    x: i32,
    y: i32,
}

#[kani::proof]
fn check_invariant() {
    let point: Point = kani::any();
    assert!(point.is_safe());
}
