// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

/// We prohibit nested calls to old since this should be
/// unobservable to the user.
#[kani::ensures(|result| old(*ptr + old(1)) == *ptr)]
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
