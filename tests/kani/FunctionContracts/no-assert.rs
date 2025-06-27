// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts --no-assert-contracts

// Check that the -no-assert-contracts option disables the default behavior of asserting contracts of dependencies.

#[kani::requires(*add_three_ptr < 100)]
#[kani::modifies(add_three_ptr)]
fn add_three(add_three_ptr: &mut u32) {
    *add_three_ptr += 1;
    add_two(add_three_ptr);
}

#[kani::ensures(|_| old(*add_two_ptr + 1) == *add_two_ptr)] // incorrect -- should be old(*add_two_ptr + 2)
fn add_two(add_two_ptr: &mut u32) {
    *add_two_ptr += 1;
    add_one(add_two_ptr)
}

#[kani::ensures(|_| old(*add_one_ptr + 1) == *add_one_ptr)] // correct -- assertion should always succeed
fn add_one(add_one_ptr: &mut u32) {
    *add_one_ptr += 1;
}

// With --no-assert-contracts, add_two's and add_one's contracts are ignored, so verification should succeed.
#[kani::proof_for_contract(add_three)]
fn prove_add_three() {
    let mut i = kani::any();
    add_three(&mut i);
}
