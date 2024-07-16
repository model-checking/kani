// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

// Test that modifies a slice containing u32 size data

#[kani::modifies(x)]
#[kani::ensures(|_| x.iter().map(|v| *v == 0).fold(true,|a,b|a&b))]
fn zero(x: &mut [u32]) {
    x.fill(0)
}

#[kani::proof_for_contract(zero)]
fn harness() {
    let mut x = [kani::any(), kani::any(), kani::any()];
    zero(&mut x);
}
