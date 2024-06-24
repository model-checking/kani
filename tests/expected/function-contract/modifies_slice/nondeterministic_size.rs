// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

// Test that modifies a slice of nondeterministic size

#[kani::modifies_slice(x)]
#[kani::ensures(|_| x.into_iter().map(|v| *v == 0).fold(true,|a,b|a&b))]
fn zero(x: &mut [u8]) {
    x.fill(0)
}

#[kani::proof_for_contract(zero)]
fn main() {
    let mut x = [0..kani::any()].map(|_| kani::any());
    zero(&mut x);
}
