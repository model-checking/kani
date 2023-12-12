// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[kani::proof]
fn main() {
    let x: u32 = kani::any();
    if x > 0 {
        let y = x / 2;
        // y is strictly less than x
        if y == x {
            assert_ne!(y, 1);
        }
    }
}
