// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

fn check(x: i32) {
    if x > 5 {
        debug_assert_eq!(x, 10);
    }
}

#[kani::proof]
fn main() {
    check(10);
}
