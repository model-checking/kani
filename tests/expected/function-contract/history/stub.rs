// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

#[kani::ensures(|result| old(*ptr + *ptr) == *ptr)]
#[kani::requires(*ptr < 100)]
#[kani::modifies(ptr)]
fn double(ptr: &mut u32) {
    *ptr += *ptr;
}

#[kani::proof_for_contract(double)]
fn double_harness() {
    let mut i = kani::any();
    double(&mut i);
}

#[kani::ensures(|result| old(*ptr + *ptr + *ptr + *ptr) == *ptr)]
#[kani::requires(*ptr < 100)]
#[kani::modifies(ptr)]
#[kani::stub_verified(double)]
fn quadruple(ptr: &mut u32) {
    double(ptr);
    double(ptr)
}

#[kani::proof_for_contract(quadruple)]
fn quadruple_harness() {
    let mut i = kani::any();
    quadruple(&mut i);
}
