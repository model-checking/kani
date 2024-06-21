// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

/// This test shows you can use `old` without the modifies contract.
/// It precomputes the desired expression, but this precomputation should
/// be equivalent to the postcomputation as it remains unchanged.
#[kani::ensures(|result : &u32| old(val) == val && old(val.wrapping_add(1)) == *result)]
fn add1(val: u32) -> u32 {
    val.wrapping_add(1)
}

#[kani::proof_for_contract(add1)]
fn main() {
    let i = kani::any();
    add1(i);
}
