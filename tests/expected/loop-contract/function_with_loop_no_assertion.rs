// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT

// kani-flags: -Z loop-contracts -Z function-contracts --no-assert-contracts

//Call a function with loop without checking the contract.

#![feature(proc_macro_hygiene)]
#![feature(stmt_expr_attributes)]

#[kani::requires(i>=2)]
#[kani::ensures(|ret| *ret == 2)]
pub fn has_loop(mut i: u16) -> u16 {
    #[kani::loop_invariant(i>=2)]
    while i > 2 {
        i = i - 1
    }
    i
}

#[kani::proof]
fn contract_proof() {
    let i: u16 = kani::any();
    let j = has_loop(i);
}
