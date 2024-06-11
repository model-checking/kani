// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts -Zmem-predicates

//! Check that Kani contract can be applied to a constant function.
//! <https://github.com/model-checking/kani/issues/3258>

#![feature(effects)]

#[kani::requires(kani::mem::can_dereference(arg))]
const unsafe fn dummy<T>(arg: *const T) -> T {
    std::ptr::read(arg)
}

#[kani::proof_for_contract(dummy)]
fn check() {
    unsafe { dummy(&kani::any::<u8>()) };
}
