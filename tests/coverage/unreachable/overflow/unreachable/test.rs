// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// This test checks that kani injects a reachability check for arithmetic
// overflow checks and that it reports ones that are unreachable
// The arithmetic overflow check in this test is unreachable, so should be
// reported as UNREACHABLE

fn reduce(x: u32) -> u32 {
    if x > 1000 { x - 1000 } else { x }
}

#[kani::proof]
fn main() {
    reduce(7);
    reduce(33);
    reduce(728);
}
