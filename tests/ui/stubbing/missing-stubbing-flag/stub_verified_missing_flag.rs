// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// kani-flags: -Z function-contracts

// Test that Kani complains when stub_verified is used without the stubbing feature enabled

#[kani::requires(true)]
fn some_function() {}

#[kani::proof_for_contract(some_function)]
fn harness() {
    some_function();
}

#[kani::stub_verified(some_function)]
#[kani::proof]
fn test_missing_stubbing_flag() {}
