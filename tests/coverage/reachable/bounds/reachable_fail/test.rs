// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// This test checks that kani injects a reachability check for
// index-out-of-bounds checks and that it reports ones that are unreachable.
// The check in this test is reachable and doesn't hold, so should be reported
// as FAILURE

fn get(s: &[i16], index: usize) -> i16 {
    s[index]
}

#[kani::proof]
fn main() {
    get(&[7, -83, 19], 15);
}
