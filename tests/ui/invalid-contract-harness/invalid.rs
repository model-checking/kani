// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

// This test is to check Kani's error handling of invalid usages of the `proof_for_contract` harness.
// We also ensure that all errors and warnings are printed in one compilation.

#[kani::requires(true)]
fn foo() {}

#[kani::proof_for_contract(foo)]
#[kani::proof_for_contract(foo)]
fn multiple_proof_annotations() {
    foo();
}

#[kani::proof_for_contract(foo)]
fn proof_with_arg(arg: bool) {
    foo();
}

#[kani::proof_for_contract(foo)]
fn generic_harness<T: Default>() {
    foo();
}
