// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

// Test `conver_contract` functionality, which fails verification for a vacuous precondition
// or unreachable postcondition.
// See https://github.com/model-checking/kani/issues/2793

// Vacuous precondition; separate requires clauses
#[kani::requires(a > 5)]
#[kani::requires(a < 4)]
#[kani::ensures(|result: &u32| *result == a)]
fn separate_requires(a: u32) -> u32 {
    panic!("This code is never tested")
}

#[kani::proof_for_contract(separate_requires)]
fn prove_separate_requires() {
    let x: u32 = kani::any();
    separate_requires(x);
}

// Vacuous precondition; single requires clause
#[kani::requires(a > 5 && a < 4)]
#[kani::ensures(|result: &u32| *result == a)]
fn conjoined_requires(a: u32) -> u32 {
    panic!("This code is never tested")
}

#[kani::proof_for_contract(conjoined_requires)]
fn prove_conjoined_requires() {
    let x: u32 = kani::any();
    conjoined_requires(x);
}
