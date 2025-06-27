// Copyright Kani Contributors
// SPDX-License-Identifier: Apache-2.0 OR MIT
// kani-flags: -Zfunction-contracts

static mut PTR: u32 = 0;

#[kani::requires(PTR < 100)]
unsafe fn modify() {
    PTR += 1;
}

#[kani::proof_for_contract(modify)]
fn main() {
    unsafe {
        modify();
    }
}
