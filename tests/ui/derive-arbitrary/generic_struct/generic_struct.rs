// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Check that Kani can automatically derive Arbitrary for structs with named fields.

#[derive(kani::Arbitrary)]
struct Point<X, Y> {
    x: X,
    y: Y,
}

#[kani::proof]
fn check_arbitrary_point() {
    let point: Point<i32, i8> = kani::any();
    if kani::any() {
        assert!(point.x >= 0);
        assert!(point.x <= 0);
        assert!(point.x != 0);
    } else {
        assert!(point.y >= 0);
        assert!(point.y <= 0);
        assert!(point.y != 0);
    }
}
