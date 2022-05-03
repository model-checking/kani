// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
#[kani::proof]
fn main() {
    let x: u32 = kani::any();
    if x < u32::MAX >> 1 {
        let y = x * 2;

        assert!(y % 2 == 0);
        assert!(y % 2 != 3);
    }
}
