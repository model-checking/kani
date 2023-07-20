// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

fn get(s: &[i16], index: usize) -> i16 {
    if index < s.len() { s[index] } else { -1 }
}

#[kani::proof]
fn main() {
    //get(&[7, -83, 19], 2);
    get(&[5, 206, -46, 321, 8], 8);
}
