// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// This test checks that kani injects a reachability check for debug_assert_eq
// macro and that it reports ones that are unreachable.
// The check in this test is reachable, so should be reported as SUCCESS

fn check(x: i32) {
    if x > 5 {
        kani::cover!();
        debug_assert_eq!(x, 10);
    }
}

#[kani::proof]
fn main() {
    check(10);
}
