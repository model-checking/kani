// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// This test consumes > 9 GB of memory with 16 object bits. Reducing the number
// of object bits to 8 to avoid running out of memory.
// kani-flags: -Zfunction-contracts --enable-unstable --cbmc-args --object-bits 8

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
#[kani::requires(*ptr < 50)]
#[kani::modifies(ptr)]
fn quadruple(ptr: &mut u32) {
    double(ptr);
    double(ptr)
}

#[kani::proof_for_contract(quadruple)]
#[kani::stub_verified(double)]
fn quadruple_harness() {
    let mut i = kani::any();
    quadruple(&mut i);
}
