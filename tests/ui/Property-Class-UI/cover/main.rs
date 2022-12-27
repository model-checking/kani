// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[kani::proof]
fn main() {
    let i: i32 = kani::any();
    kani::cover!(i < 0, "i may be negative");
}
