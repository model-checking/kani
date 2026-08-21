// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

// Test that a contract modifying a whole slice `&mut [T]` can be used as a
// verified stub. This exercises the `write_any` -> `write_any_slice` rewrite in
// the contract replacement pass, which previously produced an invalid doubled
// slice type (`*mut [[T]]`) and crashed the compiler.
// See https://github.com/model-checking/kani/issues/4748

#[kani::modifies(x)]
#[kani::ensures(|_| x.iter().all(|v| *v == 0))]
fn zero(x: &mut [u8]) {
    x.fill(0)
}

#[kani::proof_for_contract(zero)]
fn zero_contract() {
    let mut x = [kani::any(), kani::any(), kani::any()];
    zero(&mut x);
}

#[kani::proof]
#[kani::stub_verified(zero)]
fn zero_replace() {
    let mut x = [kani::any(), kani::any(), kani::any()];
    zero(&mut x);
    assert!(x.iter().all(|v| *v == 0));
}
