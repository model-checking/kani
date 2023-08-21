// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

#![allow(unreachable_code, unused_variables)]

#[kani::ensures(true)]
fn plain_pointer(t: *mut i32) {}

#[kani::proof_for_contract(plain_pointer)]
fn plain_ptr_harness() {
    let mut a = 0;
    plain_pointer(&mut a)
}
