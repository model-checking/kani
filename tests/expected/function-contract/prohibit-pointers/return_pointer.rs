// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

#![allow(unreachable_code, unused_variables)]

#[kani::ensures(true)]
fn return_pointer() -> *const usize {
    unreachable!()
}

#[kani::proof_for_contract(return_pointer)]
fn return_ptr_harness() {
    return_pointer();
}
