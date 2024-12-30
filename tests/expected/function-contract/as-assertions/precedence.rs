// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts -Zcontracts-as-assertions

// If a function is the target of a proof_for_contract or stub_verified, we should defer to the contract handling for those modes.
// i.e., test that -Zcontracts-as-assertions does not override the contract handling for proof_for_contract and stub_verified.

#[kani::modifies(add_three_ptr)]
#[kani::requires(*add_three_ptr < 100)]
fn add_three(add_three_ptr: &mut u32) {
    *add_three_ptr += 1;
    add_two(add_three_ptr);
}

#[kani::requires(*add_two_ptr < 101)]
#[kani::ensures(|result| old(*add_two_ptr + 2) == *add_two_ptr)]
fn add_two(add_two_ptr: &mut u32) {
    *add_two_ptr += 1;
    add_one(add_two_ptr);
}

#[kani::modifies(add_one_ptr)]
#[kani::requires(*add_one_ptr == 1)]
#[kani::ensures(|result| old(*add_one_ptr + 1) == *add_one_ptr)]
fn add_one(add_one_ptr: &mut u32) {
    *add_one_ptr += 1;
}

// Test that proof_for_contract takes precedence over the assert mode, i.e.
// that the target of the proof for contract still has its preconditions assumed.
#[kani::proof_for_contract(add_one)]
fn proof_for_contract_takes_precedence() {
    let mut i = kani::any();
    // if add_one's precondition was asserted, verification would fail,
    // but since it's assumed, we get a vacuously successful proof instead.
    kani::assume(i == 2);
    add_one(&mut i);
}

// Test that stub_verified takes precedence over the assert mode.
// Verification should succeed because we stub add_two by its contract,
// meaning we never reach add_one's contract.
#[kani::proof_for_contract(add_three)]
#[kani::stub_verified(add_two)]
fn stub_verified_takes_precedence() {
    let mut i = kani::any();
    add_three(&mut i);
}
