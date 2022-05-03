// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// This test checks that kani injects a reachability check for debug_assert
// macro and that it reports ones that are unreachable.
// The check in this test is unreachable, so should be reported as UNREACHABLE

fn check(x: i32) {
    if x < 0 {
        debug_assert!(x != -10);
    }
}

#[kani::proof]
fn main() {
    check(5);
}
