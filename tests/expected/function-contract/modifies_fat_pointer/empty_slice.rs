// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts -Zstubbing

// Test that modifies an empty slice whose data pointer is dangling

#[kani::modifies(x)]
#[kani::ensures(|_| x.iter().map(|v| *v == 0).fold(true,|a,b|a&b))]
fn zero(x: &mut [u8]) {
    x.fill(0)
}

#[kani::proof_for_contract(zero)]
fn check_zero() {
    let mut v: Vec<u8> = Vec::new();
    zero(v.as_mut_slice());
}

#[kani::proof]
#[kani::stub_verified(zero)]
fn use_zero() {
    let mut v: Vec<u8> = Vec::new();
    zero(v.as_mut_slice());
    assert!(v.is_empty());
}
