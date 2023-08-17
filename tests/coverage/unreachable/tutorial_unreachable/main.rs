// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[kani::proof]
fn unreachable_example() {
    let x = 5;
    let y = x + 2;
    if x > y {
        assert!(x < 8);
    }
}
