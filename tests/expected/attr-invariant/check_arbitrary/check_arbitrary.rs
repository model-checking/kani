// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Check that the `kani::invariant` attribute automatically generates the
//! `Arbitrary` implementation, and also that the values it generates respect
//! the type invariant.

extern crate kani;
use kani::Invariant;

#[kani::invariant(*x >= 0 && *y >= 0)]
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
