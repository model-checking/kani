// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// @flag --integer-overflow
// @expect overflow
// kani-verify-fail

fn get128() -> u8 {
    128
}

#[kani::proof]
pub fn main() {
    let a: u8 = get128();
    let b: u8 = 2;
    let c = a * b;
}
