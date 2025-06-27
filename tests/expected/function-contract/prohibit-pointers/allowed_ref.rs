// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

#![allow(unreachable_code, unused_variables)]

#[kani::ensures(|result| true)]
fn allowed_ref(t: &bool) {}

#[kani::proof_for_contract(allowed_ref)]
fn allowed_ref_harness() {
    let _ = Box::new(());
    let a = true;
    allowed_ref(&a)
}
