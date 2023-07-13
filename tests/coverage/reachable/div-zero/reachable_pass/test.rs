// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// This test checks that kani injects a reachability check for
// divide-by-zero checks and that it reports ones that are unreachable.
// The check in this test is reachable, so should be reported as SUCCESS

fn div(x: u16, y: u16) -> u16 {
    if y != 0 { x / y } else { 0 }
}

#[kani::proof]
fn main() {
    div(11, 3);
}
