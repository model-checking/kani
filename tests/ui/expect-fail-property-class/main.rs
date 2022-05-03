// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Check if expect_fail uses new property class and description in it's check id

#[kani::proof]
fn main() {
    let i: i32 = kani::any();
    kani::assume(i < 10);
    kani::expect_fail(i > 20, "Blocked by assumption above.");
}
