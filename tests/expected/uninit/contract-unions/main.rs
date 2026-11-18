// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Z uninit-checks -Z function-contracts

//! Unions passed to and returned from a function under contract: the buffer that carries
//! their state across calls starts empty and is part of every contract's write set.

use std::mem::MaybeUninit;

#[kani::ensures(|result: &MaybeUninit<u32>| true)]
fn make(value: u32) -> MaybeUninit<u32> {
    MaybeUninit::new(value)
}

#[kani::proof_for_contract(make)]
fn returns_union_ok() {
    let value: u32 = kani::any();
    let m = make(value);
    assert_eq!(unsafe { m.assume_init() }, value);
}

#[kani::requires(true)]
fn read(m: MaybeUninit<u32>) -> u32 {
    unsafe { m.assume_init() }
}

#[kani::proof_for_contract(read)]
fn takes_initialized_union_ok() {
    let value: u32 = kani::any();
    assert_eq!(read(MaybeUninit::new(value)), value);
}

#[kani::proof_for_contract(read)]
fn takes_uninitialized_union_ub() {
    let _ = read(MaybeUninit::uninit());
}
