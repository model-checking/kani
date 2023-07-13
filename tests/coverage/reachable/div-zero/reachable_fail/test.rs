// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// This test checks that kani injects a reachability check for
// divide-by-zero checks and that it reports ones that are unreachable.
// The check in this test is reachable and doesn't hold, so should be reported
// as FAILURE

fn div(x: u16, y: u16) -> u16 {
    x / y
}

#[kani::proof]
fn main() {
    div(678, 0);
}
