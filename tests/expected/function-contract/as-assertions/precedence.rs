// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

// If a function is the target of a proof_for_contract or stub_verified, we should defer to the contract handling for those modes.

#[kani::modifies(add_three_ptr)]
#[kani::requires(*add_three_ptr < 100)]
fn add_three(add_three_ptr: &mut u32) {
    *add_three_ptr += 1;
    add_two(add_three_ptr);
}

#[kani::requires(*add_two_ptr < 101)]
#[kani::ensures(|_| old(*add_two_ptr + 2) == *add_two_ptr)]
fn add_two(add_two_ptr: &mut u32) {
    *add_two_ptr += 1;
    add_one(add_two_ptr);
}

#[kani::modifies(add_one_ptr)]
// 4 is arbitrary -- just needs to be some value that's possible after calling add_three and add_two
#[kani::requires(*add_one_ptr == 4)]
#[kani::ensures(|_| old(*add_one_ptr + 1) == *add_one_ptr)]
fn add_one(add_one_ptr: &mut u32) {
    *add_one_ptr += 1;
}

// Simple test that proof_for_contract takes precedence over the assert mode, i.e.
// that the target of the proof for contract still has its preconditions assumed.
// If the precondition wasn't assumed, then the addition would overflow,
// so if verification succeeds, we know that the precondition was assumed.
#[kani::proof_for_contract(add_one)]
fn simple_proof_for_contract_takes_precedence() {
    let mut i = kani::any();
    add_one(&mut i);
}

// Complex test that proof_for_contract takes precedence over the assert mode
// when combined with other contracts that are being asserted.
// In this harness, add_three and add_two's contracts are asserted, but add_one (the target) should have its precondition assumed.
// So, assume add_three's precondition to ensure that its precondition assertion passes,
// but do not assume add_one's stricter precondition--if precedence is implemented correctly
// it should get assumed without us having to specify it in the harness, and verification should succeed.
// For a version of this harness without the assumption, see assert-preconditions::prove_add_one.
#[kani::proof_for_contract(add_one)]
fn complex_proof_for_contract_takes_precedence() {
    let mut i = kani::any();
    kani::assume(i < 100);
    add_three(&mut i);
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
