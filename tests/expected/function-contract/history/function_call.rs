// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

fn dereference(x: &u32) -> u32 {
    *x
}

fn add1(x: u32) -> u32 {
    x + 1
}

#[kani::ensures(|result| old(add1(dereference(ptr))) == *ptr)]
#[kani::requires(*ptr < 100)]
#[kani::modifies(ptr)]
fn modify(ptr: &mut u32) {
    *ptr += 1;
}

#[kani::proof_for_contract(modify)]
fn main() {
    let mut i = kani::any();
    modify(&mut i);
}
