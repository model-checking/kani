// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

#![allow(unreachable_code, unused_variables)]

#[kani::ensures(true)]
fn allowed_pointer(t: *const bool) {}

#[kani::proof_for_contract(allowed_pointer)]
fn allowed_pointer_harness() {
    let _ = Box::new(());
    let mut a = Box::new(true);
    allowed_pointer(Box::into_raw(a))
}