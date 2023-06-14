// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

#[kani::ensures(result == x || result == y)]
fn max(x: u32, y: u32) -> u32 {
    if x > y {
        x
    } else {
        y
    }
}

#[kani::requires(divisor != 0)]
fn div(dividend: u32, divisor: u32) -> u32 {
    dividend
}

#[kani::proof]
fn main() {
    div(9, 0);
}