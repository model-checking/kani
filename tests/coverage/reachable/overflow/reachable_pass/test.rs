// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// This test checks that kani injects a reachability check for arithmetic
// overflow checks and that it reports ones that are unreachable.
// The arithmetic overflow check in this test is reachable, so should be
// reported as SUCCESS

fn reduce(x: u32) -> u32 {
    if x > 1000 {
        kani::cover!();
        x - 1000
    } else {
        x
    }
}

#[kani::proof]
fn main() {
    reduce(7);
    reduce(33);
    reduce(728);
    reduce(1079);
}
