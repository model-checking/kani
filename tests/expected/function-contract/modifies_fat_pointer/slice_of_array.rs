// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

// Test that modifies a slice containing u32 size data

#[kani::modifies(&x[0..3])]
#[kani::ensures(|_| x[0..3].iter().map(|v| *v == 0).fold(true,|a,b|a&b))]
fn zero(x: &mut [u32; 6]) {
    x[0..3].fill(0)
}

#[kani::proof_for_contract(zero)]
fn harness() {
    let mut x = [kani::any(), kani::any(), kani::any(), kani::any(), kani::any(), kani::any()];
    zero(&mut x);
}
