// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

#![allow(unreachable_code, unused_variables)]

static mut B: bool = false;

#[kani::ensures(true)]
fn allowed_mut_return_ref<'a>() -> &'a mut bool {
    unsafe { &mut B as &'a mut bool }
}

#[kani::proof_for_contract(allowed_mut_return_ref)]
fn allowed_mut_return_ref_harness() {
    let _ = Box::new(());
    allowed_mut_return_ref();
}
