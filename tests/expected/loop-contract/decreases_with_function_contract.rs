// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts -Z function-contracts

//! Check decreases clause combined with function contracts.

#![feature(proc_macro_hygiene)]
#![feature(stmt_expr_attributes)]

#[kani::requires(i >= 2)]
#[kani::ensures(|ret| *ret == 2)]
pub fn has_loop_with_decreases(mut i: u16) -> u16 {
    #[kani::loop_invariant(i >= 2)]
    #[kani::loop_decreases(i)]
    while i > 2 {
        i = i - 1;
    }
    i
}

#[kani::proof_for_contract(has_loop_with_decreases)]
fn contract_proof() {
    let i: u16 = kani::any();
    let j = has_loop_with_decreases(i);
}
