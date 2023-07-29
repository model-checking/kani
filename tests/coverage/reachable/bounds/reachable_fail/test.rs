// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

fn get(s: &[i16], index: usize) -> i16 {
    s[index] // PARTIAL: `s[index]` is covered, but `index = 15` induces a failure
} // NONE: `index = 15` caused failure earlier

#[kani::proof]
fn main() {
    get(&[7, -83, 19], 15);
} // NONE: `index = 15` caused failure earlier
