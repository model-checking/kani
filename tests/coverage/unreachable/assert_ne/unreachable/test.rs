// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// This test checks that Kani injects a reachability check for the assert_ne
// macro. The test has an unreachable assert_ne statement which should be
// reported as UNREACHABLE

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
