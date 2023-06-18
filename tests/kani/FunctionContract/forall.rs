// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-verify-fail

#[kani::ensures(kani::forall(|i : usize| i != x as usize))]
fn max(x: u32, y: u32) -> u32 {
    if x > y {
        x
    } else {
        y
    }
}

#[kani::proof]
fn main() {
    max(kani::any(), kani::any());
}