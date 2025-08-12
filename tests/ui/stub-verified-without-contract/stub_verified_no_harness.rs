// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-flags: -Z function-contracts

// Test that Kani catches stub_verified attributes without corresponding proof_for_contract harnesses

#[kani::requires(x > 0)]
#[kani::ensures(|result| *result > x)]
fn target_function(x: u32) -> u32 {
    x + 1
}

fn no_contract() -> u32 {
    42
}

// This should fail because target_function has no `#[proof_for_contract]` harness
#[kani::stub_verified(target_function)]
#[kani::proof]
fn test_stub_without_harness() {
    let x = target_function(5);
    assert!(x > 5);
}

// This should also fail because no_contract has no contract at all
#[kani::stub_verified(no_contract)]
#[kani::proof]
fn test_stub_without_contract() {
    let x = no_contract();
    assert!(x == 42);
}
