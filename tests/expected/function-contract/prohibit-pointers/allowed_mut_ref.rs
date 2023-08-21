// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

#![allow(unreachable_code, unused_variables)]

#[kani::ensures(true)]
fn allowed_mut_ref(t: &mut bool) {}

#[kani::proof_for_contract(allowed_mut_ref)]
fn allowed_mut_ref_harness() {
    let _ = Box::new(());
    let mut a = true;
    allowed_mut_ref(&mut a)
}