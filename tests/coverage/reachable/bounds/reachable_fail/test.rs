// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

fn get(s: &[i16], index: usize) -> i16 {
    s[index]                // PARTIAL: `index = 15` causes failure, but `s[index]` is `COVERED`
}                           // NONE: `index = 15` caused failure earlier

#[kani::proof]
fn main() {
    get(&[7, -83, 19], 15);
}                           // NONE: `index = 15` caused failure earlier
