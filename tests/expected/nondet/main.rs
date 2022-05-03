// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[kani::proof]
fn main() {
    let x: i32 = kani::any();
    if (x > -500 && x < 500) {
        // x * x - 2 * x + 1 == 4 -> x == -1 || x == 3
        assert!(x * x - 2 * x + 1 != 4 || (x == -1 || x == 3));
    }
}
