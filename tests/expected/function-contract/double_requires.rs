// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[kani::ensures(result <= x || result == 1)]
#[kani::ensures(result != 0)]
#[kani::requires(x < u32::MAX)]
fn reduce1(x: u32, y: u32) -> u32 {
    x + 1
}

#[kani::ensures(result <= x || result == 1)]
#[kani::ensures(result != 0)]
#[kani::requires(x < u32::MAX)]
fn reduce2(x: u32, y: u32) -> u32 {
    x
}

#[kani::proof]
fn main2() {
    reduce2(kani::any(), kani::any());
}

#[kani::proof]
fn main() {
    reduce1(kani::any(), kani::any());
}
