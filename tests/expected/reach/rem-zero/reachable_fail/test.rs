// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// This test checks that kani injects a reachability check for
// remainder-by-zero checks and that it reports ones that are unreachable.
// The check in this test is reachable and doesn't hold, so should be reported
// as FAILURE

fn rem(x: u16, y: u16) -> u16 {
    x % y
}

#[kani::proof]
fn main() {
    rem(678, 0);
}
