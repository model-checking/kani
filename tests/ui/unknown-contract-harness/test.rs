// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

// This test checks Kani's error when function specified in `proof_for_contract`
// harness is not found (e.g. because it's not reachable from the harness)

#[kani::requires(true)]
fn foo() {}

#[kani::proof_for_contract(foo)]
fn check_foo() {
    assert!(true);
}
