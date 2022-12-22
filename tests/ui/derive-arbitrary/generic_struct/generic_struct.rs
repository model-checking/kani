// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Check that Kani can automatically derive Arbitrary for structs with named fields.

extern crate kani;

use kani::cover;

#[derive(kani::Arbitrary)]
struct Point<X, Y> {
    x: X,
    y: Y,
}

#[kani::proof]
fn check_arbitrary_point() {
    let point: Point<i32, i8> = kani::any();
    cover!(point.x > 0);
    cover!(point.x < 0);
    cover!(point.x == 0);
    cover!(point.y > 0);
    cover!(point.y < 0);
    cover!(point.y == 0);
}
