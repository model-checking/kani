// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// This test checks that kani injects a reachability check for
// index-out-of-bounds checks and that it reports ones that are unreachable.
// The check in this test is unreachable, so should be reported as UNREACHABLE

fn get(s: &[i16], index: usize) -> i16 {
    if index < s.len() { s[index] } else { -1 }
}

#[kani::proof]
fn main() {
    //get(&[7, -83, 19], 2);
    get(&[5, 206, -46, 321, 8], 8);
}
