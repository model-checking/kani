// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts
type T = u8;

/// Euclid's algorithm for calculating the GCD of two numbers
#[kani::requires(x != 0 && y != 0)]
// Changed `0` to `1` in `x % result == 0` to mess with this contract
#[kani::ensures(|result| result != 0 && x % result == 1 && y % result == 0)]
fn gcd(x: T, y: T) -> T {
    let mut max = x;
    let mut min = y;
    if min > max {
        let val = max;
        max = min;
        min = val;
    }

    let res = max % min;
    if res == 0 { min } else { gcd(min, res) }
}
#[kani::proof_for_contract(gcd)]
fn simple_harness() {
    let _ = gcd(kani::any(), kani::any());
}
