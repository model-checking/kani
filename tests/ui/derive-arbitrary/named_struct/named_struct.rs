// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Check that Kani can automatically derive Arbitrary for structs with named fields.

#[derive(kani::Arbitrary)]
struct Point {
    x: i32,
    y: i32,
}

#[kani::proof]
fn check_arbitrary_point() {
    let point: Point = kani::any();
    kani::cover!(point.x > 0);
    kani::cover!(point.x <= 0);
    kani::cover!(point.y > 0);
    kani::cover!(point.y <= 0);
}
