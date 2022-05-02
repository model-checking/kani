// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// This test checks that kani injects a reachability check for arithmetic
// overflow checks and that it reports ones that are unreachable.
// The arithmetic overflow check in this test is reachable but does not hold, so
// should be reported as FAILURE

fn cond_reduce(thresh: u32, x: u32) -> u32 {
    if x > thresh { x - 50 } else { x }
}

#[kani::proof]
fn main() {
    cond_reduce(60, 70);
    cond_reduce(40, 42);
}
