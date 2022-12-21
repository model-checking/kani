// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Check if `cover` uses a separate property class and description in its check id

#[kani::proof]
fn main() {
    let i: i32 = kani::any();
    kani::cover!(i > 20, "i may be greater than 20");
}
