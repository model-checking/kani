// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

// See https://github.com/model-checking/kani/issues/2793
#[kani::requires(a > 5)]
#[kani::requires(a < 4)]
#[kani::ensures(|result| *result == a)]
fn buggy(a: u32) -> u32 {
    panic!("This code is never tested")
}

#[kani::proof_for_contract(buggy)]
fn prove_buggy() {
    let x: u32 = kani::any();
    buggy(x);
}
