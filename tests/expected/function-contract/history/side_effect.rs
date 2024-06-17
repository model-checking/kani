// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

/// This test performs a side effect within the `old` expression
/// which leads to confusion in the ensures clause. There are
/// currently no checks to prevent side effects within `old`
/// expressions.
#[kani::ensures(|result| old({*ptr+=1; *ptr}) == _val)]
#[kani::requires(*ptr < 100)]
#[kani::requires(*ptr == _val)]
#[kani::modifies(ptr)]
fn modify(ptr: &mut u32, _val: u32) {
    *ptr += 1;
}

#[kani::proof_for_contract(modify)]
fn main() {
    let x = kani::any();
    let mut i = x;
    modify(&mut i, x);
}
