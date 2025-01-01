// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

#[kani::requires(divisor != 0)]
#[kani::ensures(|result : &u32| *result <= dividend)]
fn div(dividend: u32, divisor: u32) -> u32 {
    dividend / divisor
}

#[kani::requires(a >= b)]
#[kani::ensures(|result : &u32| *result <= a)]
fn sub(a: u32, b: u32) -> u32 {
    a - b
}

#[kani::proof]
#[kani::stub_verified(div)]
#[kani::stub_verified(sub)]
fn main() {
    assert!(div(sub(9, 1), 1) != 10, "contract guarantees smallness");
}
