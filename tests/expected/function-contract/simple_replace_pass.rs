// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

#[kani::requires(divisor != 0)]
#[kani::ensures(result <= dividend)]
fn div(dividend: u32, divisor: u32) -> u32 {
    dividend / divisor
}

#[kani::proof]
#[kani::stub_verified(div)]
fn main() {
    assert!(div(9, 1) != 10, "contract guarantees smallness");
}
