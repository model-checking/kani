// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[kani::proof]
fn main() {
    let i: i32 = kani::any();
    kani::assume(i < 10);
    kani::expect_fail(i > 10, "Blocked by assumption above.");
}
