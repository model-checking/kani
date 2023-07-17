// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// This test checks that kani injects a reachability check for debug_assert_ne
// macro and that it reports ones that are unreachable.
// The check in this test is reachable and does not hold, so should be reported
// as FAILURE

fn check(x: i32) {
    if x > 5 {
        kani::cover!();
        debug_assert_ne!(x, 17);
    }
}

#[kani::proof]
fn main() {
    check(17);
}
