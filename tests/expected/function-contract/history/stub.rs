// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

fn double1(ptr: &mut u32) {
    *ptr += *ptr;
}

fn double2(ptr: &mut u32) {
    *ptr = *ptr + *ptr;
}

#[kani::ensures(|result| old(*ptr + *ptr) == *ptr)]
#[kani::requires(*ptr < 100)]
#[kani::modifies(ptr)]
fn modify(ptr: &mut u32) {
    double2(ptr)
}

/// This tests using `stub` within a test that uses `old`
#[kani::proof_for_contract(modify)]
#[kani::stub(double2, double1)]
fn main() {
    let mut i = kani::any();
    modify(&mut i);
}
