// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

/// Tests the error message when the number of arguments to
/// `old` is not equal to 1
#[kani::ensures(|result| {let x = old(); true})]
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
