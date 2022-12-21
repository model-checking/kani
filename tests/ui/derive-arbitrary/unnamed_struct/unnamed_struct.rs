// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Check that Kani can automatically derive Arbitrary for structs with unnamed fields.

#[derive(kani::Arbitrary)]
struct Point(i32, i32);

#[kani::proof]
fn check_arbitrary_point() {
    let point: Point = kani::any();
    kani::cover!(point.0 > 0);
    kani::cover!(point.0 < 0);
    kani::cover!(point.0 == 0);
    kani::cover!(point.1 > 0);
    kani::cover!(point.1 < 0);
    kani::cover!(point.1 == 0);
}
